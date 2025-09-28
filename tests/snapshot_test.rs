#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use insta::{assert_snapshot, with_settings};

#[test]
fn test_recipe_human_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("simple.cook")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Snapshot the human-readable recipe output
    assert_snapshot!(stdout);
}

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
        .arg("--pretty")
        .arg("simple.cook")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Parse and re-serialize to ensure consistent formatting
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let formatted = serde_json::to_string_pretty(&json).unwrap();

    assert_snapshot!(formatted);
}

#[test]
fn test_recipe_yaml_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("yaml")
        .arg("simple.cook")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_snapshot!(stdout);
}

#[test]
fn test_recipe_markdown_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("markdown")
        .arg("Breakfast/pancakes.cook")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_snapshot!(stdout);
}

#[ignore]
#[test]
fn test_shopping_list_plain() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("--plain")
        .arg("simple.cook")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Sort lines for consistent output
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    let sorted = lines.join("\n");

    assert_snapshot!(sorted);
}

#[ignore]
#[test]
fn test_shopping_list_categorized() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("simple.cook")
        .arg("Breakfast/pancakes.cook")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Filter out ANSI color codes for snapshot testing
    let cleaned = strip_ansi_escapes::strip(&stdout);
    let cleaned_str = String::from_utf8(cleaned).unwrap();

    assert_snapshot!(cleaned_str);
}

#[ignore]
#[test]
fn test_shopping_list_json() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("shopping-list")
        .arg("-f")
        .arg("json")
        .arg("--pretty")
        .arg("simple.cook")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Parse and re-serialize for consistent formatting
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let formatted = serde_json::to_string_pretty(&json).unwrap();

    assert_snapshot!(formatted);
}

#[test]
fn test_doctor_validate_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .arg("doctor")
        .arg("validate")
        .arg("-b")
        .arg(temp_dir.path())
        .arg("-v")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Filter out ANSI codes and normalize paths
    let cleaned = strip_ansi_escapes::strip(&stdout);
    let mut cleaned_str = String::from_utf8(cleaned).unwrap();

    // Replace temp directory paths with a placeholder for consistent snapshots
    let temp_path = temp_dir.path().to_string_lossy();
    cleaned_str = cleaned_str.replace(temp_path.as_ref(), "[TEMP_DIR]");

    // Split into recipe blocks and summary
    let mut current_recipe = Vec::new();
    let mut recipes = Vec::new();
    let mut summary_lines = Vec::new();
    let mut in_summary = false;

    for line in cleaned_str.lines() {
        if line.contains("Validation Summary") {
            in_summary = true;
        }

        if in_summary {
            summary_lines.push(line);
        } else if line.starts_with("📄") {
            // Start of a new recipe
            if !current_recipe.is_empty() {
                recipes.push(current_recipe.join("\n"));
                current_recipe = Vec::new();
            }
            current_recipe.push(line);
        } else if !line.is_empty() && !current_recipe.is_empty() {
            // This is a warning line for the current recipe
            current_recipe.push(line);
        }
    }

    // Add the last recipe if any
    if !current_recipe.is_empty() {
        recipes.push(current_recipe.join("\n"));
    }

    // Sort recipes by filename for consistency
    recipes.sort();

    // Reconstruct the output
    let mut sorted_output = recipes.join("\n\n");
    if !summary_lines.is_empty() {
        sorted_output.push_str("\n\n");
        sorted_output.push_str(&summary_lines.join("\n"));
    }

    assert_snapshot!(sorted_output);
}

#[test]
fn test_search_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("search")
        .arg("flour")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Sort lines for consistent output
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    let sorted = lines.join("\n");

    // Use platform-specific snapshots
    #[cfg(target_os = "windows")]
    assert_snapshot!("search_output_windows", sorted);
    #[cfg(not(target_os = "windows"))]
    assert_snapshot!("search_output", sorted);
}

#[test]
fn test_scaled_recipe_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("--pretty")
        .arg("simple.cook:3")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Parse and re-serialize for consistent formatting
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let formatted = serde_json::to_string_pretty(&json).unwrap();

    assert_snapshot!(formatted);
}

#[test]
fn test_help_output() {
    let output = Command::cargo_bin("cook")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Filter version numbers for stable snapshots
    // On Windows, the executable name is cook.exe, normalize it to cook
    with_settings!({filters => vec![
        (r"cookcli \d+\.\d+\.\d+", "cookcli [VERSION]"),
        (r"Usage: cook\.exe", "Usage: cook"),  // Normalize Windows executable name in usage line
    ]}, {
        // Use different snapshots based on whether self-update feature is enabled
        #[cfg(feature = "self-update")]
        assert_snapshot!("help_output", stdout);
        #[cfg(not(feature = "self-update"))]
        assert_snapshot!("help_output_no_update", stdout);
    });
}

#[test]
fn test_recipe_with_references_output() {
    let temp_dir = common::setup_test_recipes().unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("-f")
        .arg("json")
        .arg("--pretty")
        .arg("with_ref.cook")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Parse and re-serialize for consistent formatting
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let formatted = serde_json::to_string_pretty(&json).unwrap();

    assert_snapshot!(formatted);
}
