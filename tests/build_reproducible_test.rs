//! `cook build web` writes the same site for the same collection.
//!
//! The search index was listed in the order of a `HashMap` walk, which
//! changes from one process to the next: every build of an unchanged
//! collection rewrote `static/search-index.js`, so a site published through
//! git or rsync always had a change to ship.

#[path = "common/mod.rs"]
mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn seed_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("seed")
}

/// Builds the seed into `out`, with every optional output on.
fn build(home: &Path, out: &Path) {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.args(["build", "web"])
        .arg(out)
        .arg("--base-path")
        .arg(seed_dir())
        .args([
            "--sitemap",
            "https://recipes.example.com",
            "--feed=https://recipes.example.com",
        ]);
    let status = common::with_isolated_config(&mut cmd, home)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run cook build web");
    assert!(status.success(), "cook build web failed");
}

/// Every file under `root`, by its path relative to `root`.
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                found.insert(relative, std::fs::read(&path).unwrap());
            }
        }
    }
    found
}

#[test]
fn two_builds_of_one_collection_are_identical() {
    let home = TempDir::new().unwrap();
    let first = home.path().join("first");
    let second = home.path().join("second");
    build(home.path(), &first);
    build(home.path(), &second);

    let first = files(&first);
    let second = files(&second);
    assert_eq!(
        first.keys().collect::<Vec<_>>(),
        second.keys().collect::<Vec<_>>(),
        "both builds must write the same files"
    );
    assert!(
        first.contains_key(Path::new("static").join("search-index.js").as_path()),
        "the search index must be written"
    );
    let differing: Vec<_> = first
        .iter()
        .filter(|(path, bytes)| second[*path] != **bytes)
        .map(|(path, _)| path)
        .collect();
    assert!(differing.is_empty(), "differ between builds: {differing:?}");
}
