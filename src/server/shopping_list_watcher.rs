//! Filesystem watcher that broadcasts `.shopping-list` / `.shopping-checked`
//! changes so open browsers can refresh without reload.
//!
//! Startup is best-effort: if `notify` fails to initialize (permission
//! issues, unsupported platform), the server logs a warning and continues
//! without live updates. The SSE endpoint still serves — it just never emits.

use camino::Utf8Path;
use serde::Serialize;
use std::path::Path;

/// Which of the two watched files changed. The client uses this only as a
/// hint for logging; it re-fetches regardless.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WatchedFile {
    List,
    Checked,
}

/// Event broadcast to every subscribed SSE connection.
#[derive(Debug, Clone, Serialize)]
pub struct ShoppingListChangeEvent {
    pub file: WatchedFile,
}

/// Classify a filesystem path as one of the two watched files, or None if
/// it's something we don't care about (recipe files, temp files, backups,
/// directories, etc.).
///
/// `base_path` is the server's recipe directory; paths outside of it (or
/// nested deeper than immediate children) are ignored.
pub fn classify_path(base_path: &Utf8Path, path: &Path) -> Option<WatchedFile> {
    let parent = path.parent()?;
    // Compare as Utf8Path to avoid surprises with non-UTF8 OsStr on the LHS;
    // our base_path is already Utf8Path-typed.
    let parent = camino::Utf8Path::from_path(parent)?;
    if parent != base_path {
        return None;
    }
    let name = path.file_name()?.to_str()?;
    match name {
        ".shopping-list" => Some(WatchedFile::List),
        ".shopping-checked" => Some(WatchedFile::Checked),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use std::path::PathBuf;

    fn base() -> Utf8PathBuf {
        Utf8PathBuf::from("/tmp/recipes")
    }

    #[test]
    fn classifies_shopping_list() {
        let p = PathBuf::from("/tmp/recipes/.shopping-list");
        assert_eq!(classify_path(&base(), &p), Some(WatchedFile::List));
    }

    #[test]
    fn classifies_shopping_checked() {
        let p = PathBuf::from("/tmp/recipes/.shopping-checked");
        assert_eq!(classify_path(&base(), &p), Some(WatchedFile::Checked));
    }

    #[test]
    fn ignores_temp_file_from_atomic_rename() {
        let p = PathBuf::from("/tmp/recipes/.shopping-checked.tmp");
        assert_eq!(classify_path(&base(), &p), None);
    }

    #[test]
    fn ignores_recipe_files() {
        let p = PathBuf::from("/tmp/recipes/Breakfast/Pancakes.cook");
        assert_eq!(classify_path(&base(), &p), None);
    }

    #[test]
    fn ignores_nested_shopping_list() {
        // A `.shopping-list` inside a subdirectory is not our file.
        let p = PathBuf::from("/tmp/recipes/subdir/.shopping-list");
        assert_eq!(classify_path(&base(), &p), None);
    }

    #[test]
    fn ignores_path_outside_base() {
        let p = PathBuf::from("/etc/.shopping-list");
        assert_eq!(classify_path(&base(), &p), None);
    }
}
