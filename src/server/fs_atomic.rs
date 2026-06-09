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
}
