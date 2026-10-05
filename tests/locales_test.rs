//! Every locale translates every message the English one has, once.
//!
//! A missing message does not fail anywhere else: Fluent falls back to
//! en-US, so the page renders, half in English. That is how the error page's
//! title and link, the recipe page's "Include in shopping list" tooltip, and
//! the Italian editor toolbar went untranslated. A feature branch started
//! before a new locale landed adds its strings to every locale it knows of,
//! and the newcomer is left out without anyone noticing.
//!
//! A message defined twice is the opposite failure, and a louder one: Fluent
//! refuses the whole bundle, and every page panics. Two branches adding the
//! same strings (#563 and #567 both added the error page's) merge cleanly into
//! exactly that.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn locales_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("locales")
}

/// The message ids a `.ftl` file defines, in order and repeats included:
/// lines starting with an identifier followed by `=`. Terms (`-name`),
/// attributes (`.attr`), comments and continuation lines all start with
/// something else.
fn message_id_list(path: &Path) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter(|line| line.starts_with(|c: char| c.is_ascii_alphabetic()))
        .filter_map(|line| line.split_once('='))
        .map(|(id, _)| id.trim().to_string())
        .collect()
}

fn message_ids(path: &Path) -> BTreeSet<String> {
    message_id_list(path).into_iter().collect()
}

fn ftl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("read {dir:?}: {err}"))
        .map(|entry| entry.expect("read locale entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ftl"))
        .collect();
    files.sort();
    files
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

    let english_files = ftl_files(&english);

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

#[test]
fn no_locale_defines_a_message_twice() {
    let root = locales_dir();
    let mut locales: Vec<PathBuf> = fs::read_dir(&root)
        .expect("read locales/")
        .map(|entry| entry.expect("read locales/ entry").path())
        .filter(|path| path.is_dir())
        .collect();
    locales.sort();

    let mut repeats = Vec::new();
    for locale in &locales {
        // All of a locale's files load into one bundle, so a message is a
        // duplicate whether its copies share a file or not.
        let mut seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for file in ftl_files(locale) {
            let name = file.file_name().unwrap().to_string_lossy().into_owned();
            for id in message_id_list(&file) {
                seen.entry(id).or_default().push(name.clone());
            }
        }
        for (id, files) in seen.into_iter().filter(|(_, files)| files.len() > 1) {
            repeats.push(format!(
                "{}: {id} ({})",
                locale.file_name().unwrap().to_string_lossy(),
                files.join(", ")
            ));
        }
    }
    assert!(
        repeats.is_empty(),
        "messages defined more than once (Fluent rejects the locale, and every page panics):\n{}",
        repeats.join("\n")
    );
}

/// Every `.html` file under `dir`, at any depth.
fn template_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap_or_else(|err| panic!("read {dir:?}: {err}")) {
        let path = entry.expect("read template entry").path();
        if path.is_dir() {
            files.extend(template_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "html") {
            files.push(path);
        }
    }
    files.sort();
    files
}

/// A message whose English text carries its own parentheses must not be
/// wrapped in a second pair by the template. The pantry's add-item dialog did
/// that with `pantry-optional`, and every language read "((optional))".
#[test]
fn no_template_wraps_a_parenthesised_message_in_parentheses() {
    let mut parenthesised = Vec::new();
    for file in ftl_files(&locales_dir().join("en-US")) {
        let text = fs::read_to_string(&file).unwrap_or_default();
        for line in text.lines() {
            if !line.starts_with(|c: char| c.is_ascii_alphabetic()) {
                continue;
            }
            if let Some((id, value)) = line.split_once('=') {
                let value = value.trim();
                if value.starts_with('(') && value.ends_with(')') {
                    parenthesised.push(id.trim().to_string());
                }
            }
        }
    }

    let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let mut doubled = Vec::new();
    for file in template_files(&templates) {
        let text = fs::read_to_string(&file).unwrap_or_default();
        for id in &parenthesised {
            if text.contains(&format!("({{{{ tr.t(\"{id}\") }}}})")) {
                doubled.push(format!(
                    "{}: {id}",
                    file.strip_prefix(&templates).unwrap().display()
                ));
            }
        }
    }
    assert!(
        doubled.is_empty(),
        "templates wrapping an already parenthesised message in a second pair of parentheses:\n{}",
        doubled.join("\n")
    );
}
