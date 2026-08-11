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
fn test_recipe_latex_default_paper_size_and_margin() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("latex")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r"\documentclass[11pt,a4paper]{article}",
        ))
        .stdout(predicate::str::contains(
            r"\geometry{left=2.5cm,right=2.5cm,top=2.5cm,bottom=2.5cm}",
        ));
}

#[test]
fn test_recipe_latex_custom_paper_size() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("latex")
        .arg("--paper-size")
        .arg("letter")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r"\documentclass[11pt,letterpaper]{article}",
        ));
}

#[test]
fn test_recipe_latex_custom_margin() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("latex")
        .arg("--margin")
        .arg("3")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r"\geometry{left=3cm,right=3cm,top=3cm,bottom=3cm}",
        ));
}

#[test]
fn test_recipe_typst_default_paper_size_and_margin() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("typst")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#"#set page(paper: "a4", margin: (left: 2.5cm, right: 2.5cm, top: 2.5cm, bottom: 2.5cm))"#,
        ));
}

#[test]
fn test_recipe_typst_custom_paper_size_and_margin() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("typst")
        .arg("--paper-size")
        .arg("letter")
        .arg("--margin")
        .arg("3")
        .arg("simple.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#"#set page(paper: "us-letter", margin: (left: 3cm, right: 3cm, top: 3cm, bottom: 3cm))"#,
        ));
}

#[test]
fn test_recipe_paper_size_warns_for_unrelated_format() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("--paper-size")
        .arg("letter")
        .arg("simple.cook")
        .assert()
        .success()
        .stderr(predicate::str::contains("--paper-size"));
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

#[ignore]
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

#[ignore]
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

#[ignore]
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

/// Recipe text piped in on stdin.
///
/// The recipe is written to a file only so the test source stays readable; the
/// command receives it on stdin and never learns the path.
const STDIN_RECIPE_WITH_TITLE: &str = "---\ntitle: Fancy Pasta\nservings: 4\n---\n\
                                       Boil @water{2%cups} and add @pasta{200%g}.\n";

/// Characterization test: `cook recipe` with no path reads stdin, and the
/// recipe's own metadata title — not the internal "stdin" placeholder — is
/// what gets displayed.
#[test]
fn test_stdin_recipe_uses_its_metadata_title() {
    Command::cargo_bin("cook")
        .unwrap()
        .arg("recipe")
        .arg("read")
        .write_stdin(STDIN_RECIPE_WITH_TITLE)
        .assert()
        .success()
        .stdout(predicate::str::contains("Fancy Pasta"))
        .stdout(predicate::str::contains("stdin").not());
}

/// The same title has to reach structured output, where a placeholder would
/// silently corrupt the document rather than just look wrong.
#[test]
fn test_stdin_recipe_title_reaches_structured_output() {
    let schema = Command::cargo_bin("cook")
        .unwrap()
        .args(["recipe", "read", "-f", "jsonld", "--pretty"])
        .write_stdin(STDIN_RECIPE_WITH_TITLE)
        .output()
        .unwrap();
    assert!(schema.status.success());
    let json: JsonValue = serde_json::from_slice(&schema.stdout).expect("Invalid JSON-LD output");
    assert_eq!(json["name"], JsonValue::from("Fancy Pasta"));

    Command::cargo_bin("cook")
        .unwrap()
        .args(["recipe", "read", "-f", "md"])
        .write_stdin(STDIN_RECIPE_WITH_TITLE)
        .assert()
        .success()
        .stdout(predicate::str::contains("# Fancy Pasta"))
        .stdout(predicate::str::contains("# stdin").not());
}

/// Only when the recipe declares no title does the placeholder show through.
#[test]
fn test_stdin_recipe_without_a_title_falls_back_to_stdin() {
    Command::cargo_bin("cook")
        .unwrap()
        .arg("recipe")
        .arg("read")
        .write_stdin("Boil @water{2%cups}.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("stdin"))
        .stdout(predicate::str::contains("water"));
}

/// The `name:factor` suffix is reported back in the human header. Only the
/// JSON form of scaling was covered, which serialises the recipe alone and so
/// pins neither the title nor the `@ N` label.
#[test]
fn test_human_output_labels_inline_scaling() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["recipe", "read", "simple.cook:3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Simple Recipe @ 3"));
}
