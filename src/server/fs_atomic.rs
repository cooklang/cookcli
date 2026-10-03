//! Filesystem helpers that work around platform syscall restrictions.
//!
//! On aarch64 the kernel exposes no legacy `rename`/`renameat` syscall, so
//! libc implements `rename()` via `renameat2`. Android's seccomp filter — and
//! especially aggressive vendor policies such as Samsung's — blocks
//! `renameat2`, so any rename raises SIGSYS and the process dies with
//! "Bad system call". That is *not* a catchable error: SIGSYS terminates the
//! process before `rename()` returns, so we must avoid the syscall entirely
//! rather than handle its failure.
//!
//! See <https://github.com/cooklang/cookcli/issues/349>.
//!
//! `cookcli-core` carries the same workaround in its own `fs_atomic` module,
//! for the pantry and shopping list files it writes. Kept separate because that
//! one is crate-private (a library about recipes should not publish filesystem
//! helpers) and has no async runtime to offer [`rename_replace_async`], which
//! is what this copy is really here for.

use camino::Utf8PathBuf;
use std::io;
use std::path::Path;

/// Move `from` onto `to`, replacing `to` if it exists.
///
/// Uses an atomic `rename` everywhere except Android, where `rename` would hit
/// the seccomp-blocked `renameat2` syscall. There we fall back to copy + remove,
/// which is not atomic but uses only permitted syscalls (`openat`/`read`/
/// `write`/`unlinkat`).
pub fn rename_replace(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(target_os = "android")]
    {
        std::fs::copy(from, to)?;
        std::fs::remove_file(from)?;
        Ok(())
    }
    #[cfg(not(target_os = "android"))]
    {
        std::fs::rename(from, to)
    }
}

/// Async wrapper around [`rename_replace`], run on the blocking pool so it does
/// not stall the async runtime.
pub async fn rename_replace_async(from: Utf8PathBuf, to: Utf8PathBuf) -> io::Result<()> {
    tokio::task::spawn_blocking(move || rename_replace(from.as_std_path(), to.as_std_path()))
        .await
        .map_err(io::Error::other)?
}

/// Move `from` to `to`, failing with [`io::ErrorKind::AlreadyExists`] rather
/// than replacing anything already at `to`.
///
/// A check followed by [`rename_replace`] would race: a file created at `to`
/// between the two would be overwritten. Here the refusal is the file
/// system's own. A hard link is made first, and `link` never replaces its
/// target; only then is `from` removed. Where hard links are not available
/// (FAT and exFAT, Android shared storage, Android itself — see the module
/// note), `to` is created with `create_new`, which refuses an existing file
/// just as atomically, and `from` is copied into it. Either way `rename` is
/// never called.
///
/// On failure nothing is left at `to` that this call put there, and `from` is
/// still in place.
pub fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(not(target_os = "android"))]
    match std::fs::hard_link(from, to) {
        Ok(()) => {
            return std::fs::remove_file(from).inspect_err(|_| {
                let _ = std::fs::remove_file(to);
            });
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Err(e),
        Err(e) => {
            tracing::debug!("hard link {} failed ({e}), copying instead", to.display());
        }
    }

    copy_no_replace(from, to)?;
    std::fs::remove_file(from).inspect_err(|_| {
        let _ = std::fs::remove_file(to);
    })
}

/// Copies `from` to a new file at `to`, keeping its permissions and
/// modification time, and refusing an existing `to`.
fn copy_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    let mut source = std::fs::File::open(from)?;
    let metadata = source.metadata()?;
    let mut target = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)?;

    let copied = (|| {
        io::copy(&mut source, &mut target)?;
        target.set_permissions(metadata.permissions())?;
        if let Ok(modified) = metadata.modified() {
            target.set_modified(modified)?;
        }
        target.sync_all()
    })();
    if copied.is_err() {
        drop(target);
        let _ = std::fs::remove_file(to);
    }
    copied
}

/// [`move_no_replace`], except that `to` may name `from` itself under another
/// spelling — `pasta.cook` to `Pasta.cook` on a case-insensitive file system,
/// where both names reach the one file and a link from one to the other is
/// refused as already existing. That move goes through a hidden temporary
/// name in the same directory instead.
pub fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    if !same_file::is_same_file(from, to).unwrap_or(false) {
        return move_no_replace(from, to);
    }

    let temporary = hidden_sibling(to, "rename");
    move_no_replace(from, &temporary)?;
    move_no_replace(&temporary, to).inspect_err(|_| {
        let _ = move_no_replace(&temporary, from);
    })
}

/// Async wrapper around [`move_file`], run on the blocking pool.
pub async fn move_file_async(from: Utf8PathBuf, to: Utf8PathBuf) -> io::Result<()> {
    tokio::task::spawn_blocking(move || move_file(from.as_std_path(), to.as_std_path()))
        .await
        .map_err(io::Error::other)?
}

/// Replaces the contents of `path` with `contents` in one step: they are
/// written to a new hidden file beside it, flushed, and moved over it with
/// [`rename_replace`], so a reader sees the old file or the new one and never
/// half of either.
///
/// The temporary file is opened with `create_new` under a random name, so it
/// cannot be another file that happened to sit there.
pub fn write_replace(path: &Path, contents: &[u8]) -> io::Result<()> {
    use std::io::Write;

    let temporary = hidden_sibling(path, "tmp");
    let written = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        if let Ok(metadata) = std::fs::metadata(path) {
            file.set_permissions(metadata.permissions())?;
        }
        file.write_all(contents)?;
        file.sync_all()
    })();

    written
        .and_then(|()| rename_replace(&temporary, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temporary);
        })
}

/// Async wrapper around [`write_replace`], run on the blocking pool.
pub async fn write_replace_async(path: Utf8PathBuf, contents: Vec<u8>) -> io::Result<()> {
    tokio::task::spawn_blocking(move || write_replace(path.as_std_path(), &contents))
        .await
        .map_err(io::Error::other)?
}

/// `.name.<random>.<suffix>` in the directory of `path`: hidden, so neither
/// the recipe listing nor the request paths ever reach it.
fn hidden_sibling(path: &Path, suffix: &str) -> std::path::PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.{:016x}.{suffix}", fastrand::u64(..)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn replaces_destination_and_removes_source() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("from.tmp");
        let to = dir.path().join("to.cook");
        fs::write(&from, b"new contents").unwrap();
        fs::write(&to, b"old contents").unwrap();

        rename_replace(&from, &to).unwrap();

        assert_eq!(fs::read_to_string(&to).unwrap(), "new contents");
        assert!(!from.exists(), "source temp file should be gone");
    }

    #[test]
    fn creates_destination_when_absent() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("from.tmp");
        let to = dir.path().join("to.cook");
        fs::write(&from, b"contents").unwrap();

        rename_replace(&from, &to).unwrap();

        assert_eq!(fs::read_to_string(&to).unwrap(), "contents");
        assert!(!from.exists());
    }

    #[tokio::test]
    async fn async_wrapper_replaces_destination() {
        let dir = TempDir::new().unwrap();
        let from = Utf8PathBuf::from_path_buf(dir.path().join("from.tmp")).unwrap();
        let to = Utf8PathBuf::from_path_buf(dir.path().join("to.cook")).unwrap();
        fs::write(&from, b"async contents").unwrap();

        rename_replace_async(from.clone(), to.clone())
            .await
            .unwrap();

        assert_eq!(fs::read_to_string(&to).unwrap(), "async contents");
        assert!(!from.exists());
    }

    #[test]
    fn move_no_replace_moves_and_keeps_the_contents() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("Pasta.cook");
        let to = dir.path().join("Noodles.cook");
        fs::write(&from, b"pasta").unwrap();

        move_no_replace(&from, &to).unwrap();

        assert_eq!(fs::read_to_string(&to).unwrap(), "pasta");
        assert!(!from.exists());
    }

    #[test]
    fn move_no_replace_never_overwrites() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("Pasta.cook");
        let to = dir.path().join("Soup.cook");
        fs::write(&from, b"pasta").unwrap();
        fs::write(&to, b"soup").unwrap();

        let err = move_no_replace(&from, &to).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&from).unwrap(), "pasta");
        assert_eq!(fs::read_to_string(&to).unwrap(), "soup");
    }

    #[test]
    fn the_copy_fallback_never_overwrites_and_keeps_the_mtime() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("Pasta.cook");
        let to = dir.path().join("Noodles.cook");
        let taken = dir.path().join("Soup.cook");
        fs::write(&from, b"pasta").unwrap();
        fs::write(&taken, b"soup").unwrap();
        let mtime = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        fs::File::options()
            .write(true)
            .open(&from)
            .unwrap()
            .set_modified(mtime)
            .unwrap();

        let err = copy_no_replace(&from, &taken).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&taken).unwrap(), "soup");

        copy_no_replace(&from, &to).unwrap();
        assert_eq!(fs::read_to_string(&to).unwrap(), "pasta");
        assert_eq!(fs::metadata(&to).unwrap().modified().unwrap(), mtime);
    }

    #[test]
    fn move_file_changes_only_the_case_of_a_name() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("pasta.cook");
        let to = dir.path().join("Pasta.cook");
        fs::write(&from, b"pasta").unwrap();

        move_file(&from, &to).unwrap();

        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["Pasta.cook"], "no temporary file is left behind");
        assert_eq!(fs::read_to_string(&to).unwrap(), "pasta");
    }

    #[test]
    fn move_file_refuses_a_second_name_of_the_same_file() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("Pasta.cook");
        let to = dir.path().join("Linked.cook");
        fs::write(&from, b"pasta").unwrap();
        if fs::hard_link(&from, &to).is_err() {
            return; // No hard links here; nothing to check.
        }

        assert!(move_file(&from, &to).is_err());
        assert_eq!(fs::read_to_string(&from).unwrap(), "pasta");
        assert_eq!(fs::read_to_string(&to).unwrap(), "pasta");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn write_replace_swaps_the_contents_and_leaves_no_temporary_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("Plan.menu");
        fs::write(&path, b"old").unwrap();

        write_replace(&path, b"new").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
