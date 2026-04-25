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

#[ignore]
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

#[ignore]
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

#[test]
fn test_cli_recipe_extension_default() {
    let temp_dir = common::setup_test_recipes().unwrap();

    // Default is no extensions, so inline references should not be resolved
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("extensions.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("&salt"));
}

#[test]
fn test_cli_recipe_extension_no_modifiers() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("none")
        .assert()
        .success()
        .stdout(predicate::str::contains("&salt"));
}

#[test]
fn test_cli_recipe_extension_all() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("all")
        .assert()
        .success()
        .stdout(predicate::str::contains("&salt").not());
}

#[test]
fn test_cli_recipe_extension_multiple() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("component-alias")
        .arg("--extension")
        .arg("intermediate-preparations")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("white wine")
                .not()
                .and(predicate::str::contains("@&(1)dough").not()),
        );
}

#[test]
fn test_cli_recipe_extension_compat() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("compat")
        .assert()
        .success()
        .stdout(predicate::str::contains("&salt").not());
}

#[test]
fn test_cli_recipe_extension_modifiers() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("modifiers")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""relation":{"relation":{"type":"reference","references_to":0},"reference_target":"ingredient"}"#));
}

#[test]
fn test_cli_recipe_extension_component_alias() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("component-alias")
        .assert()
        .success()
        .stdout(predicate::str::contains("white wine").not());
}

#[test]
fn test_cli_recipe_extension_range_values() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("range-values")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#""value":{"type":"range","value":{"start":{"type":"regular","value":200.0}"#,
        ));
}

#[test]
fn test_cli_recipe_extension_intermediate_preparations() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("intermediate-preparations")
        .assert()
        .success()
        .stdout(predicate::str::contains("@&(1)dough").not());
}

#[test]
fn test_cli_recipe_extension_advanced_units() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("advanced-units")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#"unit":"g""#));
}

#[test]
fn test_cli_recipe_extension_inline_quantities() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("inline-quantities")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#"{"type":"inlineQuantity","index":0}"#,
        ));
}

#[test]
fn test_cli_recipe_extension_modes() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("extensions.cook")
        .arg("--extension")
        .arg("modes")
        .assert()
        .success()
        .stdout(predicate::str::contains("sugar eggs vanilla").not());
}
