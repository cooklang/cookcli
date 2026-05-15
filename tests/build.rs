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

#[test]
fn build_writes_recipe_pages() {
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

    // The seed contains "Easy Pancakes.cook" under Breakfast.
    let pancakes = out.join("recipe/Breakfast/Easy Pancakes.html");
    assert!(
        pancakes.is_file(),
        "pancakes html should exist at {pancakes:?}"
    );

    let html = std::fs::read_to_string(&pancakes).unwrap();
    assert!(html.contains("Pancakes"), "title should be present");
    assert!(
        !html.contains("/api/shopping_list"),
        "no shopping-list api references"
    );
}

#[test]
fn build_renders_recipes_with_title_metadata() {
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

    // Risotto.cook has title metadata "Classic Risotto alla Milanese".
    // The output URL/path must use the file stem, not the title.
    assert!(
        out.join("recipe/Risotto.html").is_file(),
        "Risotto.html should exist (title-metadata regression)"
    );
    assert!(
        out.join("recipe/lamb-chops.html").is_file(),
        "lamb-chops.html should exist (title-metadata regression)"
    );
}

#[test]
fn build_copies_images_when_present() {
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

    // At minimum, no panic. If the seed has images, api/static exists.
    let images_dir = out.join("api/static");
    if images_dir.is_dir() {
        let entries: Vec<_> = std::fs::read_dir(&images_dir).unwrap().collect();
        assert!(!entries.is_empty(), "api/static is empty");
    }
}
