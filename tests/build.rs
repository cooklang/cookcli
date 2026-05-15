use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn seed_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("seed")
}

#[test]
fn build_command_help_works() {
    let mut cmd = Command::cargo_bin("cook").unwrap();
    cmd.args(["build", "--help"]).assert().success();
}

#[test]
fn build_creates_output_dir() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("_site");
    let seed = seed_dir();

    let mut cmd = Command::cargo_bin("cook").unwrap();
    cmd.args([
        "build",
        out.to_str().unwrap(),
        "--base-path",
        seed.to_str().unwrap(),
    ])
    .assert()
    .success();

    assert!(out.is_dir(), "output dir should exist after build");
}

#[test]
fn build_writes_index_and_static_assets() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("_site");
    let seed = seed_dir();

    Command::cargo_bin("cook")
        .unwrap()
        .args([
            "build",
            out.to_str().unwrap(),
            "--base-path",
            seed.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out.join("index.html").is_file(), "index.html should exist");
    assert!(
        out.join("static/css/output.css").is_file(),
        "css should exist"
    );

    let index = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(
        !index.contains("/api/search"),
        "static index should not reference api search"
    );
    assert!(
        !index.contains("Add to shopping list"),
        "no shopping list UI"
    );
}
