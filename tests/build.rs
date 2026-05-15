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
