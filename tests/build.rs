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

    // The .cook source should be copied alongside the rendered HTML so visitors
    // can download the canonical recipe data.
    let source = out.join("recipe/Breakfast/Easy Pancakes.cook");
    assert!(source.is_file(), "source .cook should exist at {source:?}");

    // And the recipe page should expose a download link to it.
    assert!(
        html.contains("Easy Pancakes.cook"),
        "recipe page should link to the .cook source"
    );
    assert!(
        html.contains("download"),
        "recipe page should use a download attribute"
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
fn build_writes_search_index() {
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

    let idx = out.join("static/search-index.json");
    assert!(idx.is_file(), "search-index.json should exist");

    let json: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&idx).unwrap()).unwrap();
    let arr = json.as_array().expect("index is array");
    assert!(!arr.is_empty(), "index should not be empty for seed");

    let first = &arr[0];
    assert!(first.get("title").is_some());
    assert!(first.get("path").is_some());
    assert!(first.get("tags").is_some());
    assert!(first.get("ingredients").is_some());

    // Find the Risotto entry (title from metadata) and confirm its path uses file stem.
    let risotto = arr
        .iter()
        .find(|e| e.get("path").and_then(|p| p.as_str()) == Some("recipe/Risotto.html"))
        .expect("entry for recipe/Risotto.html");
    assert_eq!(
        risotto.get("title").and_then(|t| t.as_str()),
        Some("Classic Risotto alla Milanese"),
        "title should be the metadata title; path should be the file stem"
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

    // The seed contains "Easy Pancakes.jpg" alongside "Easy Pancakes.cook".
    let pancakes_image = out.join("api/static/Breakfast/Easy Pancakes.jpg");
    assert!(
        pancakes_image.is_file(),
        "expected copied image at {pancakes_image:?}"
    );
}

#[test]
fn build_writes_search_js() {
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

    assert!(
        out.join("static/js/search.js").is_file(),
        "search.js should exist"
    );

    // __PREFIX__ must be assigned BEFORE search.js loads, otherwise the IIFE
    // snapshots `undefined` and fetches use the wrong base, 404ing the index.
    let listing = std::fs::read_to_string(out.join("directory/Breakfast.html")).unwrap();
    let prefix_idx = listing
        .find("window.__PREFIX__")
        .expect("__PREFIX__ assignment missing");
    let search_idx = listing.find("search.js").expect("search.js tag missing");
    assert!(
        prefix_idx < search_idx,
        "__PREFIX__ must be set before search.js loads"
    );

    // Keyboard-shortcuts JS reads __STATIC_MODE__ to hide dynamic-only entries
    // and skip nav to nonexistent pages.
    assert!(
        listing.contains("window.__STATIC_MODE__ = true"),
        "__STATIC_MODE__ must be set to true in static output"
    );

    let index = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(
        index.contains("github.com/cooklang/cookcli"),
        "static pages should include a 'Built with CookCLI' footer link"
    );
}

#[test]
fn build_writes_menu_pages_without_dotmenu_suffix() {
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

    // Menu files (e.g. "Weekly Plan.menu") must land at "menu/<name>.html",
    // not "menu/<name>.menu.html", so search-index URLs resolve.
    assert!(
        out.join("menu/Weekly Plan.html").is_file(),
        "menu page should be at menu/<name>.html (no .menu suffix)"
    );
    assert!(
        !out.join("menu/Weekly Plan.menu.html").exists(),
        "menu page should not have a .menu.html suffix"
    );
}

#[test]
fn static_output_omits_dynamic_ui() {
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

    let index = std::fs::read_to_string(out.join("index.html")).unwrap();

    // Dynamic nav links to dynamic-only pages should be gone.
    // We look for the rendered <a href=...> form to avoid matching unrelated
    // CSS selectors that mention the same paths.
    assert!(
        !index.contains("href=\"./shopping-list\""),
        "shopping-list nav link still present in static index"
    );
    assert!(
        !index.contains("href=\"./pantry\""),
        "pantry nav link still present in static index"
    );
    assert!(
        !index.contains("href=\"./preferences\""),
        "preferences nav link still present in static index"
    );

    // The dynamic server search fetch should be gone; the static search.js
    // link should be in its place.
    assert!(
        !index.contains("/api/search"),
        "api search reference remains in static index"
    );
    assert!(
        index.contains("/static/js/search.js"),
        "static search.js link missing"
    );
}

#[test]
fn build_internal_links_resolve_to_existing_files() {
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

    // Parse a directory listing and verify every anchor href that points
    // into recipe/ menu/ or directory/ resolves to an actual file.
    let listing = std::fs::read_to_string(out.join("directory/Breakfast.html")).unwrap();
    let re = regex::Regex::new(r##"href="\.\./([^"#?]+)""##).unwrap();
    let prefixes = ["recipe/", "menu/", "directory/"];
    let mut checked = 0;
    for cap in re.captures_iter(&listing) {
        let rel = &cap[1];
        if !prefixes.iter().any(|p| rel.starts_with(p)) {
            continue;
        }
        let target = out.join(rel);
        assert!(
            target.is_file(),
            "broken link in directory/Breakfast.html: '{rel}' -> {target:?}"
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no recipe/menu/directory links found in listing"
    );
}
