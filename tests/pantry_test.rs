#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn test_pantry_depleted_human_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("depleted")
        .assert()
        .success()
        .stdout(predicate::str::contains("Depleted or Low Stock Items"))
        .stdout(predicate::str::contains("honey"))
        .stdout(predicate::str::contains("vinegar"));
}

#[test]
fn test_pantry_depleted_with_all() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("depleted")
        .arg("--all")
        .assert()
        .success()
        .stdout(predicate::str::contains("oregano"))
        .stdout(predicate::str::contains("black pepper"));
}

#[test]
fn test_pantry_depleted_json_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("-f")
        .arg("json")
        .arg("depleted")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("Valid JSON output");
    assert!(json.get("items").is_some());

    let items = json.get("items").unwrap().as_array().unwrap();
    assert!(items
        .iter()
        .any(|item| { item.get("name").unwrap().as_str().unwrap() == "honey" }));
}

#[test]
fn test_pantry_depleted_yaml_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("-f")
        .arg("yaml")
        .arg("depleted")
        .assert()
        .success()
        .stdout(predicate::str::contains("items:"))
        .stdout(predicate::str::contains("name: honey"))
        .stdout(predicate::str::contains("is_low: true"));
}

#[test]
fn test_pantry_expiring_human_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("expiring")
        .assert()
        .success()
        .stdout(predicate::str::contains("Items Expiring Within"))
        .stdout(predicate::str::contains("eggs"))
        .stdout(predicate::str::contains("expires in 2 days"));
}

#[test]
fn test_pantry_expiring_with_days() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("expiring")
        .arg("-d")
        .arg("10")
        .assert()
        .success()
        .stdout(predicate::str::contains("milk"))
        .stdout(predicate::str::contains("tomatoes"))
        .stdout(predicate::str::contains("lettuce"));
}

#[test]
fn test_pantry_expiring_expired_items() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("expiring")
        .arg("-d")
        .arg("30")
        .assert()
        .success()
        .stdout(predicate::str::contains("expired item"))
        .stdout(predicate::str::contains("EXPIRED"));
}

#[test]
fn test_pantry_expiring_json_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("-f")
        .arg("json")
        .arg("expiring")
        .arg("-d")
        .arg("30")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("Valid JSON output");
    assert!(json.get("items").is_some());

    let items = json.get("items").unwrap().as_array().unwrap();
    assert!(items
        .iter()
        .any(|item| { item.get("name").unwrap().as_str().unwrap() == "milk" }));
}

#[test]
fn test_pantry_expiring_include_unknown() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("expiring")
        .arg("--include-unknown")
        .assert()
        .success()
        .stdout(predicate::str::contains("No Expiry Date Set"))
        .stdout(predicate::str::contains("oregano"));
}

#[test]
fn test_pantry_recipes_human_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("recipes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Recipes You Can Make"));
}

#[test]
fn test_pantry_recipes_with_partial() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("recipes")
        .arg("--partial")
        .arg("--threshold")
        .arg("50")
        .assert()
        .success()
        .stdout(predicate::str::contains("Recipes You Can Make"));
}

#[test]
fn test_pantry_recipes_json_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("-f")
        .arg("json")
        .arg("recipes")
        .arg("--partial")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("Valid JSON output");
    assert!(json.get("full_matches").is_some());
    assert!(json.get("partial_matches").is_some());
}

#[test]
fn test_pantry_recipes_yaml_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("-f")
        .arg("yaml")
        .arg("recipes")
        .assert()
        .success()
        .stdout(predicate::str::contains("full_matches:"))
        .stdout(predicate::str::contains("partial_matches:"));
}

#[test]
fn test_pantry_with_base_path() {
    let temp_dir = common::setup_test_recipes().unwrap();
    let another_dir = tempfile::TempDir::new().unwrap();

    // Run from another directory with base path pointing to temp_dir
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(another_dir.path())
        .arg("pantry")
        .arg("-b")
        .arg(temp_dir.path())
        .arg("depleted")
        .assert()
        .success()
        .stdout(predicate::str::contains("honey"));
}

#[test]
fn test_pantry_no_config_error() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("depleted")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No pantry configuration found"));
}

#[test]
fn test_pantry_help() {
    Command::cargo_bin("cook")
        .unwrap()
        .arg("pantry")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage pantry inventory"))
        .stdout(predicate::str::contains("depleted"))
        .stdout(predicate::str::contains("expiring"))
        .stdout(predicate::str::contains("recipes"));
}

#[test]
fn test_pantry_subcommand_help() {
    Command::cargo_bin("cook")
        .unwrap()
        .arg("pantry")
        .arg("depleted")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Show items that are out of stock"));
}

#[test]
fn test_pantry_low_quantity_detection() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("depleted")
        .assert()
        .success()
        .stdout(predicate::str::contains("vinegar"))
        .stdout(predicate::str::contains("50%ml"))
        .stdout(predicate::str::contains("low when < 200%ml"));
}

#[test]
fn test_pantry_format_aliases() {
    let temp_dir = common::setup_test_recipes().unwrap();

    // Test short alias 'd' for depleted
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("d")
        .assert()
        .success()
        .stdout(predicate::str::contains("Depleted"));

    // Test short alias 'e' for expiring
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("e")
        .assert()
        .success()
        .stdout(predicate::str::contains("Expiring"));

    // Test short alias 'r' for recipes
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("r")
        .assert()
        .success()
        .stdout(predicate::str::contains("Recipes"));
}
