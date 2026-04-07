#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;

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

// ---------------------------------------------------------------------------
// CRUD: list
// ---------------------------------------------------------------------------

#[test]
fn test_pantry_list_human_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pantry Items"))
        .stdout(predicate::str::contains("DAIRY"))
        .stdout(predicate::str::contains("milk"))
        .stdout(predicate::str::contains("eggs"));
}

#[test]
fn test_pantry_list_section_filter() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "list", "--section", "dairy"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DAIRY"))
        .stdout(predicate::str::contains("milk"))
        // section heading for "pantry" section should not appear
        .stdout(predicate::str::contains("PANTRY:").not());
}

#[test]
fn test_pantry_list_section_filter_not_found() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "list", "--section", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_pantry_list_json_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "-f", "json", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("valid JSON");
    let sections = json.get("sections").unwrap().as_array().unwrap();
    assert!(!sections.is_empty());
    // Each section has a name and items array
    assert!(sections[0].get("name").is_some());
    assert!(sections[0].get("items").is_some());
}

#[test]
fn test_pantry_list_yaml_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "-f", "yaml", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sections:"))
        .stdout(predicate::str::contains("name:"));
}

#[test]
fn test_pantry_list_no_pantry_error() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No pantry configuration found"));
}

// ---------------------------------------------------------------------------
// CRUD: add
// ---------------------------------------------------------------------------

fn make_minimal_pantry() -> tempfile::TempDir {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir(&config_dir).unwrap();
    fs::write(
        config_dir.join("pantry.conf"),
        "[pantry]\nflour = { quantity = \"1%kg\", low = \"200%g\" }\n",
    )
    .unwrap();
    temp_dir
}

#[test]
fn test_pantry_add_simple_item() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "add", "pantry", "sugar"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added 'sugar'"));

    let content = fs::read_to_string(temp_dir.path().join("config/pantry.conf")).unwrap();
    assert!(content.contains("sugar"));
}

#[test]
fn test_pantry_add_with_attributes() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "pantry",
            "add",
            "dairy",
            "milk",
            "--quantity",
            "2%l",
            "--low",
            "500%ml",
            "--expire",
            "2025-12-01",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added 'milk'"));

    let content = fs::read_to_string(temp_dir.path().join("config/pantry.conf")).unwrap();
    assert!(content.contains("[dairy]"));
    assert!(content.contains("milk"));
    assert!(content.contains("2%l"));
    assert!(content.contains("500%ml"));
    assert!(content.contains("2025-12-01"));
}

#[test]
fn test_pantry_add_creates_new_section() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "add", "spices", "cumin", "--quantity", "50%g"])
        .assert()
        .success();

    let content = fs::read_to_string(temp_dir.path().join("config/pantry.conf")).unwrap();
    assert!(content.contains("[spices]"));
    assert!(content.contains("cumin"));
}

#[test]
fn test_pantry_add_creates_pantry_file_when_missing() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "add", "produce", "apples", "--quantity", "6"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added 'apples'"));

    assert!(temp_dir.path().join("config/pantry.conf").exists());
    let content = fs::read_to_string(temp_dir.path().join("config/pantry.conf")).unwrap();
    assert!(content.contains("apples"));
}

#[test]
fn test_pantry_add_duplicate_errors() {
    let temp_dir = make_minimal_pantry();

    // First add succeeds
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "add", "pantry", "flour"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

// ---------------------------------------------------------------------------
// CRUD: remove
// ---------------------------------------------------------------------------

#[test]
fn test_pantry_remove_item() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "remove", "pantry", "flour"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed 'flour'"));

    let content = fs::read_to_string(temp_dir.path().join("config/pantry.conf")).unwrap();
    assert!(!content.contains("flour"));
}

#[test]
fn test_pantry_remove_empty_section_deleted() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "remove", "pantry", "flour"])
        .assert()
        .success();

    let content = fs::read_to_string(temp_dir.path().join("config/pantry.conf")).unwrap();
    // Section should be removed when it becomes empty
    assert!(!content.contains("[pantry]"));
}

#[test]
fn test_pantry_remove_nonexistent_item_errors() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "remove", "pantry", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_pantry_remove_nonexistent_section_errors() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "remove", "nosuchsection", "flour"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_pantry_remove_no_pantry_error() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "remove", "pantry", "flour"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No pantry configuration found"));
}

// ---------------------------------------------------------------------------
// CRUD: update
// ---------------------------------------------------------------------------

#[test]
fn test_pantry_update_quantity() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "update", "pantry", "flour", "--quantity", "2%kg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated 'flour'"));

    let content = fs::read_to_string(temp_dir.path().join("config/pantry.conf")).unwrap();
    assert!(content.contains("2%kg"));
    // Original low threshold should be preserved
    assert!(content.contains("200%g"));
}

#[test]
fn test_pantry_update_adds_expire() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "pantry",
            "update",
            "pantry",
            "flour",
            "--expire",
            "2025-12-31",
        ])
        .assert()
        .success();

    let content = fs::read_to_string(temp_dir.path().join("config/pantry.conf")).unwrap();
    assert!(content.contains("2025-12-31"));
    // Original attributes should be preserved
    assert!(content.contains("1%kg"));
    assert!(content.contains("200%g"));
}

#[test]
fn test_pantry_update_nonexistent_item_errors() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "pantry",
            "update",
            "pantry",
            "nonexistent",
            "--quantity",
            "1%kg",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_pantry_update_no_attributes_errors() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "update", "pantry", "flour"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No attributes specified"));
}

#[test]
fn test_pantry_update_no_pantry_error() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "update", "pantry", "flour", "--quantity", "1%kg"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No pantry configuration found"));
}

// ---------------------------------------------------------------------------
// CRUD: command aliases
// ---------------------------------------------------------------------------

#[test]
fn test_pantry_list_alias() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pantry Items"));
}

#[test]
fn test_pantry_add_alias() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "a", "pantry", "rice"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added 'rice'"));
}

#[test]
fn test_pantry_remove_alias() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "rm", "pantry", "flour"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed 'flour'"));
}

#[test]
fn test_pantry_update_alias() {
    let temp_dir = make_minimal_pantry();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "up", "pantry", "flour", "--quantity", "3%kg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated 'flour'"));
}

// ---------------------------------------------------------------------------
// CRUD: help text
// ---------------------------------------------------------------------------

#[test]
fn test_pantry_help_shows_crud_commands() {
    Command::cargo_bin("cook")
        .unwrap()
        .args(["pantry", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("remove"))
        .stdout(predicate::str::contains("update"));
}

// ---------------------------------------------------------------------------
// Serialization: general items, ordering, non-ASCII
// ---------------------------------------------------------------------------

/// General (top-level) items must stay at the top level after a round-trip
/// through add (which re-serializes the whole file).
#[test]
fn test_pantry_general_items_stay_top_level() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir(&config_dir).unwrap();
    // Top-level `key = "string"` items are parsed as "general" section items.
    // Inline tables at the top level are treated as sections by the parser.
    fs::write(
        config_dir.join("pantry.conf"),
        "salt = \"1%kg\"\npepper = \"50%g\"\n\n[dairy]\nmilk = { quantity = \"2%l\" }\n",
    )
    .unwrap();

    // Add a new item to the dairy section, triggering a full re-serialize
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "add", "dairy", "cheese"])
        .assert()
        .success();

    let content = fs::read_to_string(config_dir.join("pantry.conf")).unwrap();

    // General items must appear before any [section] header
    let first_section = content.find('[').expect("should have a section header");
    let salt_pos = content.find("salt").expect("salt should be present");
    let pepper_pos = content.find("pepper").expect("pepper should be present");
    assert!(
        salt_pos < first_section,
        "general item 'salt' should appear before the first section header\n{content}"
    );
    assert!(
        pepper_pos < first_section,
        "general item 'pepper' should appear before the first section header\n{content}"
    );

    // Must NOT have a [general] section header
    assert!(
        !content.contains("[general]"),
        "general items should not be wrapped in a [general] section\n{content}"
    );

    // The file must still be parseable and list everything
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("salt"))
        .stdout(predicate::str::contains("pepper"))
        .stdout(predicate::str::contains("milk"))
        .stdout(predicate::str::contains("cheese"));
}

/// Item order within a section must be preserved after a round-trip.
#[test]
fn test_pantry_item_order_preserved() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir(&config_dir).unwrap();
    fs::write(
        config_dir.join("pantry.conf"),
        "[pantry]\nzucchini = { quantity = \"2\" }\napple = { quantity = \"5\" }\nmango = { quantity = \"3\" }\n",
    )
    .unwrap();

    // Add triggers a re-serialize of the entire file
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "add", "pantry", "banana"])
        .assert()
        .success();

    let content = fs::read_to_string(config_dir.join("pantry.conf")).unwrap();

    let pos_z = content.find("zucchini").expect("zucchini present");
    let pos_a = content.find("apple").expect("apple present");
    let pos_m = content.find("mango").expect("mango present");
    let pos_b = content.find("banana").expect("banana present");

    assert!(
        pos_z < pos_a && pos_a < pos_m && pos_m < pos_b,
        "items should keep their original order with new item appended\n{content}"
    );
}

/// Section order must be preserved after a round-trip.
#[test]
fn test_pantry_section_order_preserved() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir(&config_dir).unwrap();
    fs::write(
        config_dir.join("pantry.conf"),
        "[dairy]\nmilk = { quantity = \"1%l\" }\n\n[produce]\napple = { quantity = \"5\" }\n\n[bakery]\nbread = { quantity = \"1 loaf\" }\n",
    )
    .unwrap();

    // Update triggers a re-serialize of the entire file
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "update", "dairy", "milk", "--quantity", "2%l"])
        .assert()
        .success();

    let content = fs::read_to_string(config_dir.join("pantry.conf")).unwrap();

    let pos_dairy = content.find("[dairy]").expect("[dairy] present");
    let pos_produce = content.find("[produce]").expect("[produce] present");
    let pos_bakery = content.find("[bakery]").expect("[bakery] present");

    assert!(
        pos_dairy < pos_produce && pos_produce < pos_bakery,
        "sections should keep their original order\n{content}"
    );
}

/// Non-ASCII characters (e.g. Swedish å ä ö) must round-trip correctly.
#[test]
fn test_pantry_non_ascii_round_trip() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Add an item with non-ASCII characters
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "add", "mejeri", "smörgås"])
        .assert()
        .success();

    let content = fs::read_to_string(temp_dir.path().join("config/pantry.conf")).unwrap();
    assert!(
        content.contains("smörgås"),
        "non-ASCII item name should appear in the file\n{content}"
    );

    // The file must parse successfully and show the item
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("smörgås"));

    // Add a second item to trigger a full re-serialize, then verify again
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "add", "mejeri", "äpple", "--quantity", "3"])
        .assert()
        .success();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["pantry", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("smörgås"))
        .stdout(predicate::str::contains("äpple"));
}
