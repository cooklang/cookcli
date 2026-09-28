//! Every locale translates every message the English one has.
//!
//! A missing message does not fail anywhere else: Fluent falls back to
//! en-US, so the page renders, half in English. That is how the error page's
//! title and link, the recipe page's "Include in shopping list" tooltip, and
//! the Italian editor toolbar went untranslated. A feature branch started
//! before a new locale landed adds its strings to every locale it knows of,
//! and the newcomer is left out without anyone noticing.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn locales_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("locales")
}

/// The message ids a `.ftl` file defines: lines starting with an identifier
/// followed by `=`. Terms (`-name`), attributes (`.attr`), comments and
/// continuation lines all start with something else.
fn message_ids(path: &Path) -> BTreeSet<String> {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter(|line| line.starts_with(|c: char| c.is_ascii_alphabetic()))
        .filter_map(|line| line.split_once('='))
        .map(|(id, _)| id.trim().to_string())
        .collect()
}

#[test]
fn every_locale_has_every_english_message() {
    let root = locales_dir();
    let english = root.join("en-US");
    let mut locales: Vec<PathBuf> = fs::read_dir(&root)
        .expect("read locales/")
        .map(|entry| entry.expect("read locales/ entry").path())
        .filter(|path| path.is_dir() && *path != english)
        .collect();
    locales.sort();
    assert!(!locales.is_empty(), "no locale besides en-US in {root:?}");

    let mut english_files: Vec<PathBuf> = fs::read_dir(&english)
        .expect("read locales/en-US")
        .map(|entry| entry.expect("read locales/en-US entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ftl"))
        .collect();
    english_files.sort();

    let mut gaps = Vec::new();
    for file in &english_files {
        let name = file.file_name().unwrap();
        let wanted = message_ids(file);
        assert!(!wanted.is_empty(), "no messages found in {file:?}");
        for locale in &locales {
            let have = message_ids(&locale.join(name));
            let missing: Vec<_> = wanted.difference(&have).cloned().collect();
            if !missing.is_empty() {
                gaps.push(format!(
                    "{}/{}: {}",
                    locale.file_name().unwrap().to_string_lossy(),
                    name.to_string_lossy(),
                    missing.join(", ")
                ));
            }
        }
    }
    assert!(
        gaps.is_empty(),
        "messages missing from a locale (they would show in English):\n{}",
        gaps.join("\n")
    );
}
