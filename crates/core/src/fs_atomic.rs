//! Replacing a file the user owns without ever leaving it half-written.
//!
//! Two things in this crate rewrite a whole file someone else maintains: the
//! pantry configuration and the saved shopping list. Both re-serialise the
//! entire file on every change, so a plain [`std::fs::write`] — which truncates
//! before it writes — turns a full disk or a killed process into a truncated
//! pantry or a lost shopping list. Everything here exists to make that
//! impossible.
//!
//! This module is deliberately not public. It is a filesystem detail, not part
//! of what this crate is about, and the CLI needs an async form of the rename
//! that this crate cannot provide (it has no async runtime) — so the CLI keeps
//! its own copy in `src/server/fs_atomic.rs`. Both cite
//! <https://github.com/cooklang/cookcli/issues/349>.

use crate::CoreError;
use camino::Utf8Path;

/// Write `contents` over `path` so that a failure leaves the previous file
/// intact.
///
/// The new bytes go to a temporary file beside the target, are flushed to the
/// disk, and are then renamed over it — which either happens completely or not
/// at all.
///
/// Three details that keep a hand-maintained file working:
///
/// - **Symlinks are followed.** An existing target is resolved to the file it
///   names before anything is written, so a `config/pantry.conf` symlinked into
///   a dotfiles repository is updated through the link rather than replaced by
///   a regular file — and the temporary file lands on the same filesystem as
///   the file it replaces, which rename requires.
/// - **Permissions are carried over**, because a temporary file is created from
///   the process umask, which may be more permissive than the target was.
/// - **The temporary file is fsynced** before the rename, so a crash right
///   after it cannot leave a file that has been renamed into place but whose
///   contents never reached the disk.
///
/// A failure leaves no temporary file behind unless removing it fails too, and
/// names the target as the caller gave it rather than wherever the symlinks and
/// `..`s led — that is the file the caller knows about.
pub(crate) fn write_atomically(
    path: &Utf8Path,
    contents: impl AsRef<[u8]>,
) -> Result<(), CoreError> {
    let failed = |source: std::io::Error| CoreError::Io {
        path: path.to_owned(),
        source,
    };
    let target = path.canonicalize_utf8().unwrap_or_else(|_| path.to_owned());

    // `parent` is empty for a bare relative filename, which is the current
    // directory and needs no creating.
    if let Some(parent) = target.parent().filter(|parent| !parent.as_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(failed)?;
    }

    // Named after the process, so two `cook`s writing the same file do not
    // overwrite each other's half-written temporary file. They can still race
    // over the file itself; last rename wins, and neither leaves it corrupt.
    // The name starts with a dot so it is hidden from a recipe collection, and
    // matches neither of the names the web server's filesystem watcher reacts
    // to.
    let name = target.file_name().unwrap_or("file");
    let temp = target.with_file_name(format!(".{name}.{}.tmp", std::process::id()));

    let written = (|| -> std::io::Result<()> {
        let file = std::fs::File::create(&temp)?;
        {
            use std::io::Write as _;
            let mut file = &file;
            file.write_all(contents.as_ref())?;
            file.flush()?;
        }
        file.sync_all()?;
        drop(file);
        if let Ok(metadata) = std::fs::metadata(&target) {
            std::fs::set_permissions(&temp, metadata.permissions())?;
        }
        rename_replace(&temp, &target)
    })();

    if let Err(source) = written {
        // Best effort. The target itself is untouched either way, which is the
        // thing worth protecting.
        let _ = std::fs::remove_file(&temp);
        return Err(failed(source));
    }
    Ok(())
}

/// Move `from` onto `to`, replacing `to` if it is there.
///
/// A plain rename everywhere but Android, where libc implements `rename` with
/// the `renameat2` syscall that Android's seccomp filter blocks: the process is
/// killed with SIGSYS rather than handed an error, so the syscall has to be
/// avoided rather than recovered from. The fallback there is copy then remove,
/// which is *not* atomic — an interrupted write on Android can still truncate
/// the file.
pub(crate) fn rename_replace(from: &Utf8Path, to: &Utf8Path) -> std::io::Result<()> {
    #[cfg(target_os = "android")]
    {
        std::fs::copy(from, to)?;
        std::fs::remove_file(from)
    }
    #[cfg(not(target_os = "android"))]
    {
        std::fs::rename(from, to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(dir: &tempfile::TempDir) -> camino::Utf8PathBuf {
        camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
    }

    fn names_in(dir: &Utf8Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir.as_std_path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn writes_bytes_and_leaves_nothing_behind() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = base(&dir);
        let target = base.join(".shopping-list");

        write_atomically(&target, b"./Soup\n").expect("writes");

        assert_eq!(
            std::fs::read_to_string(target.as_std_path()).unwrap(),
            "./Soup\n"
        );
        assert_eq!(
            names_in(&base),
            [".shopping-list"],
            "no temporary file left"
        );
    }

    #[test]
    fn replaces_an_existing_file_wholesale() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = base(&dir).join(".shopping-list");
        std::fs::write(target.as_std_path(), "a much longer previous list\n").unwrap();

        write_atomically(&target, b"./Soup\n").expect("writes");

        assert_eq!(
            std::fs::read_to_string(target.as_std_path()).unwrap(),
            "./Soup\n"
        );
    }

    /// The one failure that happens *after* the temporary file exists.
    /// Renaming onto a non-empty directory cannot succeed, which reaches that
    /// branch without needing permission bits.
    #[test]
    fn a_failure_after_the_temporary_file_exists_still_removes_it() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = base(&dir);
        let target = base.join("a-directory");
        std::fs::create_dir_all(target.join("child").as_std_path()).unwrap();

        match write_atomically(&target, b"contents") {
            Err(CoreError::Io { path, .. }) => assert_eq!(path, target),
            other => panic!("expected Io, got {other:?}"),
        }

        assert_eq!(names_in(&base), ["a-directory"], "no temporary file left");
    }
}
