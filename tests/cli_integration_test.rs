#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_cli_recipe_command() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("water"));
}

#[test]
fn test_cli_recipe_with_scaling() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("simple.cook:2")
        .assert()
        .success()
        .stdout(predicate::str::contains("water"));
}

#[test]
fn test_cli_shopping_list() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("water"))
        .stdout(predicate::str::contains("pasta"));
}

#[test]
fn test_cli_shopping_list_multiple_recipes() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("simple.cook")
        .arg("sauce.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("water")); // Check for ingredient from first recipe
}

#[test]
fn test_cli_shopping_list_plain() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--plain")
        .arg("simple.cook")
        .assert()
        .success();
}

#[test]
fn test_cli_search() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("search")
        .arg("water")
        .assert()
        .success()
        .stdout(predicate::str::contains("simple.cook"));
}

#[test]
fn test_cli_search_multiple_terms() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("search")
        .arg("oil")
        .arg("garlic")
        .assert()
        .success()
        .stdout(predicate::str::contains("sauce.cook"));
}

#[test]
fn test_cli_doctor_validate() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .arg("doctor")
        .arg("validate")
        .arg("-b")
        .arg(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn test_cli_doctor_validate_with_errors() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .arg("doctor")
        .arg("validate")
        .arg("-b")
        .arg(temp_dir.path())
        .arg("-v")
        .assert()
        .success(); // Non-strict mode succeeds even with errors
}

#[test]
fn test_cli_doctor_validate_strict() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .arg("doctor")
        .arg("validate")
        .arg("-b")
        .arg(temp_dir.path())
        .arg("--strict")
        .assert()
        .failure(); // Strict mode fails with errors
}

#[test]
fn test_cli_seed() {
    let temp_dir = TempDir::new().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("seed")
        .arg("test_seed") // Specify explicit path
        .assert()
        .success();

    // Check that seed directory was created
    assert!(temp_dir.path().join("test_seed").exists());
}

#[test]
fn test_cli_seed_custom_path() {
    let temp_dir = TempDir::new().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("seed")
        .arg("my_recipes")
        .assert()
        .success();

    // Check that custom directory was created
    assert!(temp_dir.path().join("my_recipes").exists());
}

#[test]
fn test_cli_help() {
    Command::cargo_bin("cook")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "command-line interface for managing and working with Cooklang recipes",
        ));
}

#[test]
fn test_cli_recipe_help() {
    Command::cargo_bin("cook")
        .unwrap()
        .arg("recipe")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Parse and display"));
}

#[test]
fn test_cli_shopping_list_help() {
    Command::cargo_bin("cook")
        .unwrap()
        .arg("shopping-list")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Create shopping lists"));
}

#[test]
fn test_cli_nonexistent_recipe() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("nonexistent.cook")
        .assert()
        .failure();
}

#[test]
fn test_cli_recipe_from_subdirectory() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("Breakfast/pancakes.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("Pancakes"));
}
