//! Creating a recipe or menu file from the web UI.
//!
//! Shared by the new-file form (`/new`) and "Save as menu" on the shopping
//! list, so that both get the same name rules and the same refusal of paths
//! that leave the recipe directory.

use camino::{Utf8Path, Utf8PathBuf};

/// A file [`create`] made.
#[derive(Debug)]
pub(crate) struct NewFile {
    /// The name as cleaned up, relative to the recipe directory and without
    /// its extension: `Plans/Week 12`.
    pub name: String,
    /// Where it was written.
    pub file: Utf8PathBuf,
}

/// Why [`create`] made nothing.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NewFileError {
    /// Nothing is left of the name once cleaned up.
    EmptyName,
    /// The recipe directory itself cannot be resolved.
    BasePath,
    /// The file would land outside the recipe directory.
    OutsideCollection,
    /// Its folder could not be created.
    Directory,
    /// A file with this name exists.
    Exists,
    /// It was created but could not be written.
    Write,
    /// It could not be created.
    Create,
}

impl NewFileError {
    /// The message the web UI shows, `noun` naming the kind of file.
    pub fn message(&self, noun: &str) -> String {
        match self {
            Self::EmptyName => format!("{} name cannot be empty", capitalize(noun)),
            Self::BasePath => "Internal error: invalid base path".to_string(),
            Self::OutsideCollection => format!("Invalid {noun} path"),
            Self::Directory => {
                "Failed to create directory. Check that the recipes folder has write permissions."
                    .to_string()
            }
            Self::Exists => format!("A {noun} with this name already exists"),
            Self::Write => format!("Failed to write {noun} file"),
            Self::Create => format!("Failed to create {noun} file"),
        }
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// `name` as a file name: letters, digits, spaces, `-`, `_` and `/` kept,
/// empty folders dropped. `None` when nothing is left.
pub(crate) fn clean_name(name: &str) -> Option<String> {
    let kept: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_' || *c == '/')
        .collect();
    // Spaces around a folder or file name are dropped too: a folder called
    // "  " or a file called " .menu" is only ever a typo.
    let cleaned = kept
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");
    (!cleaned.is_empty()).then_some(cleaned)
}

/// The title a new file gets: the last part of its name, with `-` and `_` as
/// spaces.
pub(crate) fn title_of(name: &str) -> String {
    name.split('/')
        .next_back()
        .unwrap_or(name)
        .replace(['-', '_'], " ")
}

/// Creates `<name>.<extension>` under `base_path`, its folders too, holding
/// `content(title)`. Never overwrites a file, and never creates or writes
/// anything that resolves, through a symlink or otherwise, outside
/// `base_path`.
pub(crate) async fn create(
    base_path: &Utf8Path,
    name: &str,
    extension: &str,
    content: impl FnOnce(&str) -> String,
) -> Result<NewFile, NewFileError> {
    let name = clean_name(name).ok_or(NewFileError::EmptyName)?;
    let file = base_path.join(format!("{name}.{extension}"));

    // The recipe directory, resolved, for the containment check below
    let base = base_path.to_owned();
    let base_canonical = match tokio::task::spawn_blocking(move || base.canonicalize_utf8()).await {
        Ok(Ok(p)) => p,
        _ => return Err(NewFileError::BasePath),
    };

    // Decide before touching the disk: the folder the file goes in, resolved
    // through any symlink as far as it exists, must sit under the recipe
    // directory. Nothing is created until that holds, so a refusal leaves
    // nothing behind — and nothing that already existed is ever removed. A
    // sub-folder symlinked elsewhere (a NAS share) used to be deleted here by
    // a "clean-up" of the folder this request had not created (#549).
    if let Some(parent) = file.parent() {
        let parent_owned = parent.to_owned();
        let base = base_canonical.clone();
        let inside = tokio::task::spawn_blocking(move || {
            super::canonical_or_nearest(&parent_owned).starts_with(&base)
        })
        .await
        .unwrap_or(false);
        if !inside {
            tracing::warn!("Refused to create {file}: it resolves outside {base_canonical}");
            return Err(NewFileError::OutsideCollection);
        }

        if !parent.exists() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                tracing::error!("Failed to create directories: {}", e);
                return Err(NewFileError::Directory);
            }
        }
    }

    let text = content(&title_of(&name));

    // Use OpenOptions with create_new to atomically check existence and create
    // This prevents TOCTOU race conditions
    use tokio::io::AsyncWriteExt;
    let opened = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true) // Fails if file exists - atomic check + create
        .open(&file)
        .await;

    match opened {
        Ok(mut f) => {
            // A tokio `File` hands writes to a background task: without the
            // flush, a redirect can reach the browser, and the editor load
            // the file, before anything is on disk.
            let written = match f.write_all(text.as_bytes()).await {
                Ok(()) => f.flush().await,
                Err(e) => Err(e),
            };
            if let Err(e) = written {
                tracing::error!("Failed to write {file}: {}", e);
                return Err(NewFileError::Write);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(NewFileError::Exists);
        }
        Err(e) => {
            tracing::error!("Failed to create {file}: {}", e);
            return Err(NewFileError::Create);
        }
    }

    Ok(NewFile { name, file })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_keep_only_what_a_file_name_may_hold() {
        assert_eq!(
            clean_name("Plans/Week 12").as_deref(),
            Some("Plans/Week 12")
        );
        assert_eq!(
            clean_name("/Plans//Week-12/").as_deref(),
            Some("Plans/Week-12")
        );
        assert_eq!(
            clean_name("../../etc/passwd").as_deref(),
            Some("etc/passwd")
        );
        assert_eq!(
            clean_name("Soupe à l'oignon").as_deref(),
            Some("Soupe à loignon")
        );
        assert_eq!(
            clean_name(" Plans / Week 12 ").as_deref(),
            Some("Plans/Week 12")
        );
        assert_eq!(clean_name("  "), None);
        assert_eq!(clean_name("  /// "), None);
        assert_eq!(clean_name("../.."), None);
    }

    #[test]
    fn a_title_is_the_last_part_of_the_name() {
        assert_eq!(title_of("Plans/week_12-b"), "week 12 b");
        assert_eq!(title_of("Soup"), "Soup");
    }
}
