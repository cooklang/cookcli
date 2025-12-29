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

#[test]
fn test_pantry_depleted_respects_explicit_threshold() {
    // Test for GitHub issue #228: Explicit low thresholds should take precedence over heuristics
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    std::fs::create_dir(&config_dir).unwrap();

    // Create a pantry.conf with test cases for the bug fix
    let pantry_config = r#"[test]
# Should appear as depleted (15 < 20)
"item_below_threshold" = { quantity = "15%g", low = "20%g" }

# Should NOT appear as depleted (85 > 20) - This was the reported bug
"item_above_threshold" = { quantity = "85%g", low = "20%g" }

# Should appear using heuristic (50 <= 100)
"item_no_threshold" = "50%g"

# Should NOT appear (200 > 100 heuristic)
"item_high_no_threshold" = "200%g"
"#;
    std::fs::write(config_dir.join("pantry.conf"), pantry_config).unwrap();

    // Test default depleted output
    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("depleted")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();

    // Item below explicit threshold should appear
    assert!(
        stdout.contains("item_below_threshold"),
        "Item below explicit threshold (15g < 20g) should appear in depleted list"
    );

    // Item above explicit threshold should NOT appear (this is the bug fix)
    assert!(
        !stdout.contains("item_above_threshold"),
        "Item above explicit threshold (85g > 20g) should NOT appear in depleted list"
    );

    // Item without threshold but low by heuristic should appear
    assert!(
        stdout.contains("item_no_threshold"),
        "Item without explicit threshold (50g <= 100g heuristic) should appear"
    );

    // Item without threshold and high by heuristic should NOT appear
    assert!(
        !stdout.contains("item_high_no_threshold"),
        "Item without threshold (200g > 100g heuristic) should NOT appear"
    );

    // Test with --all flag to verify item_above_threshold can appear when requested
    let output_all = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("pantry")
        .arg("depleted")
        .arg("--all")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout_all = String::from_utf8(output_all).unwrap();

    // With --all flag, item_above_threshold should now appear
    assert!(
        stdout_all.contains("item_above_threshold"),
        "Item above threshold should appear with --all flag"
    );
}
