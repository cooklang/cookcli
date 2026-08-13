//! End-to-end tests for `cook shopping-list`.
//!
//! # Why these were all `#[ignore]`d
//!
//! Every test in this file carried a bare `#[ignore]` with no reason, and ten
//! of the eighteen failed when run, so the command had no regression coverage
//! at all (<https://github.com/cooklang/cookcli/issues/415>). Three things had
//! gone stale under them:
//!
//! - The shared fixture gained a `config/pantry.conf`, and pantry items are
//!   subtracted by default. Most of what these tests look for — `water`,
//!   `salt`, `pasta`, `flour`, `tomatoes` — is in it, so the assertions were
//!   looking for ingredients the command was right to leave out. Those tests
//!   pass `--ignore-pantry` now, except where the pantry *is* the subject.
//! - `--exclude-pantry` was renamed `--ignore-pantry`.
//! - The `Shopping List` header is no longer printed.
//!
//! These are intent-based assertions — that references are expanded, that
//! quantities combine, that a modifier is honoured — and they complement
//! `shopping_list_characterization_test`, which pins exact output. Where the
//! two overlapped exactly, the copy here was dropped rather than repaired.

#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;

#[test]
fn test_shopping_list_basic() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--ignore-pantry")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("water"))
        .stdout(predicate::str::contains("salt"))
        .stdout(predicate::str::contains("pasta"));
}

#[test]
fn test_shopping_list_multiple_recipes() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("simple.cook")
        .arg("sauce.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("water"))
        .stdout(predicate::str::contains("oil"))
        .stdout(predicate::str::contains("garlic"));
}

#[test]
fn test_shopping_list_with_scaling() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("simple.cook:2")
        .assert()
        .success()
        .stdout(predicate::str::contains("water"))
        .stdout(predicate::str::contains("4 cups")); // Doubled from 2 cups
}

#[test]
fn test_shopping_list_plain_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--plain")
        .arg("simple.cook")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();

    // Plain format should not have headers or sections
    assert!(!stdout.contains("Shopping List"));
    assert!(!stdout.contains("==="));

    // Should contain ingredients
    assert!(stdout.contains("water"));
    assert!(stdout.contains("pasta"));
    assert!(stdout.contains("salt"));
}

#[test]
fn test_shopping_list_json_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("-f")
        .arg("json")
        .arg("simple.cook")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("Valid JSON output");

    // An array of categories, each with its items — not an object with an
    // `ingredients` or `sections` key, which is what this used to look for.
    let categories = json.as_array().expect("an array of categories");
    let first = categories.first().expect("at least one category");
    assert!(first.get("category").is_some(), "{json}");
    assert!(
        first["items"][0].get("name").is_some(),
        "every item is named: {json}"
    );
}

#[test]
fn test_shopping_list_yaml_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("-f")
        .arg("yaml")
        .arg("--ignore-pantry")
        .arg("simple.cook")
        .assert()
        .success()
        // The same shape as the JSON writer: a sequence of categories, each
        // with its items.
        .stdout(predicate::str::contains("- category:"))
        .stdout(predicate::str::contains("items:"))
        .stdout(predicate::str::contains("name: water"));
}

#[test]
fn test_shopping_list_output_to_file() {
    let temp_dir = common::setup_test_recipes().unwrap();
    let output_file = temp_dir.path().join("shopping.txt");

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("-o")
        .arg(&output_file)
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::is_empty()); // Output goes to file, not stdout

    // Verify file was created and contains expected content
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("water"));
    assert!(content.contains("pasta"));
}

#[test]
fn test_shopping_list_with_aisle_categorization() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--ignore-pantry")
        .arg("simple.cook")
        .arg("Breakfast/pancakes.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("DAIRY").or(predicate::str::contains("dairy")))
        .stdout(predicate::str::contains("PANTRY").or(predicate::str::contains("pantry")));
}

#[test]
fn test_shopping_list_with_recipe_reference() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--ignore-pantry")
        .arg("with_ref.cook")
        .assert()
        .success()
        // `tomatoes` is named by `with_ref` itself; `oil` and `garlic` can only
        // be here if `@./sauce{}` was expanded, which is the point.
        .stdout(predicate::str::contains("tomatoes"))
        .stdout(predicate::str::contains("oil"))
        .stdout(predicate::str::contains("garlic"));
}

#[test]
fn test_shopping_list_exclude_pantry() {
    let temp_dir = common::setup_test_recipes().unwrap();

    // `tomatoes` is the ingredient this can be shown with. The fixture's pantry
    // has 5 of them and `with_ref` asks for 3, both unitless, so the
    // subtraction actually happens — unlike `salt`, where the pantry says `1%kg`
    // and the recipe says `1%tsp`, and no conversion between them exists.
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("with_ref.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("tomatoes").not());

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--ignore-pantry")
        .arg("with_ref.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("tomatoes"));
}

#[test]
fn test_shopping_list_menu_file() {
    let temp_dir = common::setup_test_recipes().unwrap();
    let _menu_path = common::create_test_menu(temp_dir.path()).unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--ignore-pantry")
        .arg("weekly.menu")
        .assert()
        .success()
        .stdout(predicate::str::contains("flour")) // From pancakes
        .stdout(predicate::str::contains("water")) // From simple recipe
        .stdout(predicate::str::contains("tomatoes")); // From with_ref recipe
}

#[test]
fn test_shopping_list_combine_quantities() {
    let temp_dir = common::setup_test_recipes().unwrap();

    // Create two recipes that use the same ingredient
    fs::write(
        temp_dir.path().join("recipe1.cook"),
        r#"
Add @flour{2%cups}.
Add @butter{100%g}.
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("recipe2.cook"),
        r#"
Add @flour{3%cups}.
Add @butter{50%g}.
"#,
    )
    .unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--plain")
        .arg("--ignore-pantry")
        .arg("recipe1.cook")
        .arg("recipe2.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("5 cups")) // Combined flour
        .stdout(predicate::str::contains("150 g")); // Combined butter
}

#[test]
fn test_shopping_list_invalid_recipe() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("nonexistent.cook")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Error")));
}

#[test]
fn test_shopping_list_empty_recipe() {
    let temp_dir = common::setup_test_recipes().unwrap();

    // Create an empty recipe
    fs::write(
        temp_dir.path().join("empty.cook"),
        r#"---
title: Empty Recipe
---

No ingredients here, just instructions.
"#,
    )
    .unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("empty.cook")
        .assert()
        .success();
}

#[test]
fn test_shopping_list_pretty_json() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("-f")
        .arg("json")
        .arg("--pretty")
        .arg("simple.cook")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();

    // Pretty JSON should have indentation
    assert!(stdout.contains("  ") || stdout.contains("\n"));

    // Should be valid JSON
    let _json: Value = serde_json::from_str(&stdout).expect("Valid JSON");
}

#[test]
fn test_shopping_list_help() {
    Command::cargo_bin("cook")
        .unwrap()
        .arg("shopping-list")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Create shopping lists"))
        .stdout(predicate::str::contains("--plain"))
        .stdout(predicate::str::contains("--ignore-pantry"));
}

#[test]
fn test_shopping_list_base_path() {
    let temp_dir = common::setup_test_recipes().unwrap();
    let another_dir = tempfile::TempDir::new().unwrap();

    // Run from another directory with base path
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(another_dir.path())
        .arg("shopping-list")
        .arg("-b")
        .arg(temp_dir.path())
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("water"))
        .stdout(predicate::str::contains("pasta"));
}

#[test]
fn test_shopping_list_lists_an_aliased_ingredient_under_its_name() {
    let temp_dir = common::setup_test_recipes().unwrap();

    // `/x` is Cooklang's *alias* separator, not a modifier. This test used to
    // read `@pepper{}/hidden` as a hidden-ingredient marker and assert that
    // `pepper` was left out; there is no such thing. CookCLI parses with
    // `Extensions::empty()`, so the modifier syntax that would hide an
    // ingredient is not available at all, and every ingredient named is listed.
    fs::write(
        temp_dir.path().join("modifiers.cook"),
        r#"
Add @flour{2%cups}/plain flour.
Add @salt{}/sea salt.
Add @pepper{}/black pepper.
"#,
    )
    .unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--plain")
        .arg("--ignore-pantry")
        .arg("modifiers.cook")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();

    // Each is listed once, under the name rather than the alias.
    assert!(stdout.contains("flour"), "{stdout}");
    assert!(stdout.contains("salt"), "{stdout}");
    assert!(stdout.contains("pepper"), "{stdout}");
    assert!(
        !stdout.contains("plain flour"),
        "the alias is not what a shopping list shows: {stdout}"
    );
}
