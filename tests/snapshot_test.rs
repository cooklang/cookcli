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

/// Every optional command, with the feature that compiles it in.
///
/// Five independent features means thirty-two possible command lists, so a
/// snapshot of the raw help text only ever holds for the combination it was
/// recorded under. It used to branch on `self-update` alone, which left it
/// asserting a list with no `login`/`logout` against a build that has them —
/// so `--features sync,server,import,lsp` failed
/// (<https://github.com/cooklang/cookcli/issues/440>).
///
/// The two tests below split that apart: one snapshots the invariant part of
/// the help text, the other checks the gating. Neither depends on which
/// features this build happens to have.
const OPTIONAL_COMMANDS: &[(&str, bool)] = &[
    ("server", cfg!(feature = "server")),
    ("import", cfg!(feature = "import")),
    ("lsp", cfg!(feature = "lsp")),
    ("login", cfg!(feature = "sync")),
    ("logout", cfg!(feature = "sync")),
    ("update", cfg!(feature = "self-update")),
];

fn help_text() -> String {
    let output = Command::cargo_bin("cook")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

/// True when `line` is the command-list entry for `command`.
///
/// Anchored on the two-space indent and the following whitespace so that
/// `server` does not also match a description mentioning a server.
fn is_command_line(line: &str, command: &str) -> bool {
    line.strip_prefix("  ")
        .and_then(|rest| rest.strip_prefix(command))
        .is_some_and(|rest| rest.starts_with(' '))
}

/// The help text with the feature-gated commands removed, so that one snapshot
/// holds for every feature combination. What the gated lines *should* be is
/// [`help_lists_exactly_the_commands_that_were_compiled_in`]'s job.
#[test]
fn test_help_output() {
    let stdout = help_text();
    let invariant: String = stdout
        .lines()
        .filter(|line| {
            !OPTIONAL_COMMANDS
                .iter()
                .any(|(command, _)| is_command_line(line, command))
        })
        .map(|line| format!("{line}\n"))
        .collect();

    // Filter version numbers for stable snapshots
    // On Windows, the executable name is cook.exe, normalize it to cook
    with_settings!({filters => vec![
        (r"cookcli \d+\.\d+\.\d+", "cookcli [VERSION]"),
        (r"Usage: cook\.exe", "Usage: cook"),  // Normalize Windows executable name in usage line
    ]}, {
        assert_snapshot!("help_output", invariant);
    });
}

/// Each optional command is listed exactly when its feature is enabled.
///
/// This is what the old snapshot was really asserting, minus the coupling to
/// one particular build. It also catches the opposite mistake — a command
/// listed by a build that cannot run it.
#[test]
fn help_lists_exactly_the_commands_that_were_compiled_in() {
    let stdout = help_text();
    for (command, enabled) in OPTIONAL_COMMANDS {
        let listed = stdout.lines().any(|line| is_command_line(line, command));
        assert_eq!(
            listed,
            *enabled,
            "`{command}` is {} in the help text but its feature is {}\n{stdout}",
            if listed { "listed" } else { "missing" },
            if *enabled { "on" } else { "off" },
        );
    }
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
