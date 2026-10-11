//! The server's own folders, kept out of what it sends.
//!
//! Errors from the filesystem, the recipe finder, the parser and core name the
//! file they are about by its absolute path: `Directory does not exist:
//! /home/alice/recipes/Soup`, `failed to parse recipe
//! '/home/alice/recipes/Soup.cook'`, `invalid configuration at
//! /home/alice/.config/cook/pantry.conf`. Sent as they are, they tell any
//! visitor where the server keeps its files and, usually, the account it runs
//! as.
//!
//! So every error that reaches a client — [`json_error`], the error page, the
//! pantry page — goes through [`hide`] first. A path inside the recipe
//! directory comes out relative to it (`Soup.cook`, `config/pantry.conf`), and
//! one inside a configuration folder as `…/pantry.conf`. The server's log keeps
//! the full error.
//!
//! [`json_error`]: super::handlers::common::json_error

use camino::{Utf8Path, Utf8PathBuf};
use std::sync::RwLock;

/// Each hidden folder, with what it is replaced by when a path goes on below it
/// and when it is named by itself.
///
/// Process-wide because errors are turned into responses in many places that
/// have no `AppState` at hand. A process normally runs one server; if it builds
/// more, each one's folders are hidden from all of them, which hides more than
/// needed and never less.
static FOLDERS: RwLock<Vec<Folder>> = RwLock::new(Vec::new());

struct Folder {
    path: String,
    below: &'static str,
    alone: &'static str,
}

/// Hide `recipes`, the recipe directory, and the folders in `config`: the
/// global configuration folder and wherever the aisle, pantry and users files
/// are. A configuration folder inside the recipe directory is left to it, so
/// its files keep their relative path.
pub fn register<'a>(recipes: &Utf8Path, config: impl IntoIterator<Item = &'a Utf8Path>) {
    let recipe_dirs = spellings(recipes);
    let mut folders = FOLDERS.write().unwrap_or_else(|e| e.into_inner());
    for path in &recipe_dirs {
        add(&mut folders, path, "", ".");
    }
    for dir in config {
        for path in spellings(dir) {
            if !recipe_dirs.iter().any(|recipes| path.starts_with(recipes)) {
                add(&mut folders, &path, "…/", "…");
            }
        }
    }
    // Longest first, so a folder inside another one is matched before it.
    folders.sort_by_key(|folder| std::cmp::Reverse(folder.path.len()));
}

/// `dir` as given, and as the filesystem resolves it when that differs (a
/// symlink, or the `\\?\` prefix Windows adds): errors can carry either.
fn spellings(dir: &Utf8Path) -> Vec<Utf8PathBuf> {
    let mut paths = vec![dir.to_path_buf()];
    if let Ok(resolved) = dir.canonicalize_utf8() {
        if resolved != dir {
            paths.push(resolved);
        }
    }
    paths
}

fn add(folders: &mut Vec<Folder>, path: &Utf8Path, below: &'static str, alone: &'static str) {
    let path = path.as_str().trim_end_matches(['/', '\\']);
    // The filesystem root would turn every path into a relative one, and
    // hiding it hides nothing.
    if path.is_empty() || folders.iter().any(|folder| folder.path == path) {
        return;
    }
    folders.push(Folder {
        path: path.to_string(),
        below,
        alone,
    });
}

/// `text` with every path into a registered folder written without it.
pub fn hide(text: &str) -> String {
    let folders = FOLDERS.read().unwrap_or_else(|e| e.into_inner());
    folders.iter().fold(text.to_string(), |text, folder| {
        replace_folder(&text, folder)
    })
}

fn replace_folder(text: &str, folder: &Folder) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(&folder.path) {
        out.push_str(&rest[..at]);
        let after = &rest[at + folder.path.len()..];
        match after.chars().next() {
            // `/srv/recipes/Soup.cook`
            Some('/' | '\\') => {
                out.push_str(folder.below);
                rest = &after[1..];
            }
            // `/srv/recipes2` is another folder, but `/srv/recipes.` ends a
            // sentence.
            Some(c) if is_name_char(c) && !ends_sentence(after) => {
                out.push_str(&folder.path);
                rest = after;
            }
            // `/srv/recipes` by itself.
            _ => {
                out.push_str(folder.alone);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Whether `c` can continue a file name, so the folder seen so far is only the
/// start of a longer one.
fn is_name_char(c: char) -> bool {
    !c.is_whitespace() && !matches!(c, '\'' | '"' | '`' | ':' | ',' | ';' | ')' | ']' | '}')
}

/// Whether `after` starts with a full stop that ends the text or a sentence.
fn ends_sentence(after: &str) -> bool {
    let mut chars = after.chars();
    chars.next() == Some('.') && chars.next().is_none_or(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folders() -> Vec<Folder> {
        let mut folders = Vec::new();
        add(&mut folders, Utf8Path::new("/home/alice/recipes"), "", ".");
        add(
            &mut folders,
            Utf8Path::new("/home/alice/.config/cook/"),
            "…/",
            "…",
        );
        folders
    }

    fn hidden(text: &str) -> String {
        folders().iter().fold(text.to_string(), |text, folder| {
            replace_folder(&text, folder)
        })
    }

    #[test]
    fn a_path_in_the_recipe_directory_becomes_relative() {
        assert_eq!(
            hidden("Directory does not exist: /home/alice/recipes/Nope"),
            "Directory does not exist: Nope"
        );
        assert_eq!(
            hidden("failed to parse recipe '/home/alice/recipes/Soups/Leek.cook'"),
            "failed to parse recipe 'Soups/Leek.cook'"
        );
    }

    #[test]
    fn the_recipe_directory_itself_becomes_a_dot() {
        assert_eq!(
            hidden("i/o error on /home/alice/recipes: denied"),
            "i/o error on .: denied"
        );
        assert_eq!(hidden("`/home/alice/recipes`"), "`.`");
        assert_eq!(
            hidden("Cannot read /home/alice/recipes. Check it."),
            "Cannot read .. Check it."
        );
    }

    #[test]
    fn a_configuration_folder_keeps_only_the_file_name() {
        assert_eq!(
            hidden("invalid configuration at /home/alice/.config/cook/pantry.conf: bad"),
            "invalid configuration at …/pantry.conf: bad"
        );
    }

    #[test]
    fn a_longer_folder_with_the_same_start_is_left_alone() {
        assert_eq!(
            hidden("/home/alice/recipes-old/Soup.cook"),
            "/home/alice/recipes-old/Soup.cook"
        );
    }

    #[test]
    fn every_occurrence_is_hidden() {
        assert_eq!(
            hidden("--> /home/alice/recipes/A.cook:1:5\n/home/alice/recipes/A.cook"),
            "--> A.cook:1:5\nA.cook"
        );
    }

    #[test]
    fn windows_separators_count_too() {
        let mut folders = Vec::new();
        add(
            &mut folders,
            Utf8Path::new(r"C:\Users\alice\recipes"),
            "",
            ".",
        );
        assert_eq!(
            replace_folder(
                r"i/o error on C:\Users\alice\recipes\Soup.cook",
                &folders[0]
            ),
            "i/o error on Soup.cook"
        );
    }

    #[test]
    fn the_root_is_never_hidden() {
        let mut folders = Vec::new();
        add(&mut folders, Utf8Path::new("/"), "", ".");
        assert!(folders.is_empty());
    }
}
