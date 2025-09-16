#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;

#[ignore]
#[test]
fn test_shopping_list_basic() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("Shopping List"))
        .stdout(predicate::str::contains("water"))
        .stdout(predicate::str::contains("salt"))
        .stdout(predicate::str::contains("pasta"));
}

#[ignore]
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

#[ignore]
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

#[ignore]
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

#[ignore]
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

    // Check JSON structure
    assert!(json.get("ingredients").is_some() || json.get("sections").is_some());
}

#[ignore]
#[test]
fn test_shopping_list_yaml_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("-f")
        .arg("yaml")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("ingredients:").or(predicate::str::contains("sections:")));
}

#[ignore]
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

#[ignore]
#[test]
fn test_shopping_list_with_aisle_categorization() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("simple.cook")
        .arg("Breakfast/pancakes.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("PRODUCE").or(predicate::str::contains("produce")))
        .stdout(predicate::str::contains("DAIRY").or(predicate::str::contains("dairy")))
        .stdout(predicate::str::contains("PANTRY").or(predicate::str::contains("pantry")));
}

#[ignore]
#[test]
fn test_shopping_list_with_recipe_reference() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("with_ref.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("tomatoes"))
        .stdout(predicate::str::contains("oil"))
        .stdout(predicate::str::contains("garlic"));
}

#[ignore]
#[test]
fn test_shopping_list_exclude_pantry() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output_with_pantry = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("simple.cook")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_without_pantry = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--exclude-pantry")
        .arg("simple.cook")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Without --exclude-pantry should have more items
    assert!(output_with_pantry.len() >= output_without_pantry.len());
}

#[ignore]
#[test]
fn test_shopping_list_menu_file() {
    let temp_dir = common::setup_test_recipes().unwrap();
    let _menu_path = common::create_test_menu(temp_dir.path()).unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("weekly.menu")
        .assert()
        .success()
        .stdout(predicate::str::contains("flour")) // From pancakes
        .stdout(predicate::str::contains("water")) // From simple recipe
        .stdout(predicate::str::contains("tomatoes")); // From with_ref recipe
}

#[ignore]
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
        .arg("recipe1.cook")
        .arg("recipe2.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("5 cups")) // Combined flour
        .stdout(predicate::str::contains("150 g")); // Combined butter
}

#[ignore]
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

#[ignore]
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

#[ignore]
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

#[ignore]
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
        .stdout(predicate::str::contains("--exclude-pantry"));
}

#[ignore]
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

#[ignore]
#[test]
fn test_shopping_list_ingredient_modifiers() {
    let temp_dir = common::setup_test_recipes().unwrap();

    // Create a recipe with ingredient modifiers
    fs::write(
        temp_dir.path().join("modifiers.cook"),
        r#"
Add @flour{2%cups}/optional.
Add @salt{}/to taste.
Add @pepper{}/hidden.
"#,
    )
    .unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--plain")
        .arg("modifiers.cook")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();

    // Optional ingredients should be included
    assert!(stdout.contains("flour"));

    // "to taste" ingredients should be included
    assert!(stdout.contains("salt"));

    // Hidden ingredients should not be included
    assert!(!stdout.contains("pepper"));
}
