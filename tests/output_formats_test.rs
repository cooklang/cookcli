#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value as JsonValue;
use std::fs;

#[test]
fn test_recipe_json_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("simple.cook")
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify it's valid JSON
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: JsonValue = serde_json::from_str(&stdout).expect("Invalid JSON output");

    // Check for expected fields in Cooklang JSON structure
    assert!(json.get("metadata").is_some());
    assert!(json.get("sections").is_some());
}

#[test]
fn test_recipe_yaml_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("yaml")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("metadata:"))
        .stdout(predicate::str::contains("sections:"));
}

#[test]
fn test_recipe_markdown_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("markdown")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("## Ingredients"))
        .stdout(predicate::str::contains("## Steps"));
}

#[test]
fn test_recipe_cooklang_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("cooklang")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("@water"))
        .stdout(predicate::str::contains("#"));
}

#[test]
fn test_shopping_list_json_output() {
    let temp_dir = common::setup_test_recipes().unwrap();
    let output_file = temp_dir.path().join("shopping.json");

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("-f")
        .arg("json")
        .arg("-o")
        .arg(&output_file)
        .arg("simple.cook")
        .assert()
        .success();

    // Verify the file was created and is valid JSON
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    let _json: JsonValue = serde_json::from_str(&content).expect("Invalid JSON in output file");
}

#[test]
fn test_shopping_list_yaml_output() {
    let temp_dir = common::setup_test_recipes().unwrap();
    let output_file = temp_dir.path().join("shopping.yaml");

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("-f")
        .arg("yaml")
        .arg("-o")
        .arg(&output_file)
        .arg("simple.cook")
        .assert()
        .success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("ingredients:") || content.contains("- name:"));
}

#[test]
fn test_shopping_list_human_output_to_file() {
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
        .success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("water") || content.contains("pasta"));
}

#[test]
fn test_output_file_inference_from_extension() {
    let temp_dir = common::setup_test_recipes().unwrap();

    // Test JSON inference
    let json_file = temp_dir.path().join("output.json");
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-o")
        .arg(&json_file)
        .arg("simple.cook")
        .assert()
        .success();

    let content = fs::read_to_string(&json_file).unwrap();
    let _json: JsonValue =
        serde_json::from_str(&content).expect("Should infer JSON from .json extension");

    // Test YAML inference
    let yaml_file = temp_dir.path().join("output.yaml");
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-o")
        .arg(&yaml_file)
        .arg("simple.cook")
        .assert()
        .success();

    let content = fs::read_to_string(&yaml_file).unwrap();
    assert!(content.contains("ingredients:") || content.contains("steps:"));
}

#[test]
fn test_pretty_json_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("--pretty")
        .arg("simple.cook")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Pretty JSON should have newlines and indentation
    assert!(stdout.contains("\n  "));
}

#[test]
fn test_ingredients_only_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--ingredients-only")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("water"))
        .stdout(predicate::str::contains("pasta")); // Should contain ingredient names
}
