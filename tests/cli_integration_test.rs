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

/// Every term is searched, not just the first.
///
/// The terms above both appear in `sauce.cook`, so that test still passes if
/// the trailing ones are dropped on the way to the search. Here the first term
/// matches nothing, so the recipe can only be found by the second.
#[test]
fn test_cli_search_uses_every_term_not_just_the_first() {
    let temp_dir = common::setup_test_recipes().unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("search")
        .arg("kohlrabi")
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
        .failure()
        // The wording is user-facing text, pinned so a refactor cannot quietly
        // reword it.
        .stderr(predicate::str::contains(
            "Recipe not found: nonexistent.cook",
        ));
}

/// A recipe that is present but cannot be read is *not* reported as missing —
/// that would send the user looking for the wrong problem — and the underlying
/// cause survives to the terminal.
///
/// A directory standing in for the file reproduces this on every platform,
/// unlike permission bits, which root ignores and Windows does not have.
#[test]
fn test_cli_unreadable_recipe_reports_the_read_failure() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::create_dir(temp_dir.path().join("unreadable.cook")).unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("unreadable.cook")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read 'unreadable.cook'"))
        .stderr(predicate::str::contains("Caused by"))
        .stderr(predicate::str::contains("Recipe not found").not());
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

/// A pantry the user named with `--pantry` that cannot be read is fatal — they
/// asked for that file by name, and quietly shopping as if it were empty could
/// send them to the shop for things they already own. A pantry merely
/// *discovered* in `config/` is different: nobody asked for it, so the command
/// warns and builds the list without it.
///
/// Nothing else covers this distinction, and the two halves are one line apart
/// in `shopping_list::run`, so a refactor could collapse them unnoticed.
///
/// Permission bits are the only way to make a file that `is_file()` accepts but
/// `read_to_string` rejects — checking mere existence is what missed this case
/// originally. They are not enforced for root, and some filesystems ignore them
/// entirely, so the test probes whether they bite and skips if they do not,
/// rather than asserting something untrue.
#[cfg(unix)]
#[test]
fn test_cli_unreadable_pantry_is_fatal_only_when_named_explicitly() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    std::fs::write(root.join("a.cook"), "Add @tomatoes{3}.\n").unwrap();
    std::fs::create_dir(root.join("config")).unwrap();
    // A local aisle config so the output does not depend on the developer's
    // global `~/.config/cook/aisle.conf`.
    std::fs::write(root.join("config/aisle.conf"), "[produce]\ntomatoes\n").unwrap();
    let pantry = root.join("config/pantry.conf");
    std::fs::write(&pantry, "[cupboard]\ntomatoes = \"1\"\n").unwrap();

    let unreadable = std::fs::Permissions::from_mode(0o000);
    let readable = std::fs::Permissions::from_mode(0o644);
    std::fs::set_permissions(&pantry, unreadable).unwrap();
    if std::fs::read_to_string(&pantry).is_ok() {
        // Running as root, or on a filesystem that ignores the mode.
        std::fs::set_permissions(&pantry, readable).unwrap();
        return;
    }

    let run = |args: &[&str]| {
        Command::cargo_bin("cook")
            .unwrap()
            .current_dir(root)
            .arg("shopping-list")
            .args(args)
            .output()
            .unwrap()
    };
    let discovered = run(&["a.cook"]);
    let explicit = run(&["--pantry", "config/pantry.conf", "a.cook"]);

    // Restore before asserting: a panic here must not leave a directory the
    // `TempDir` drop cannot clean up.
    std::fs::set_permissions(&pantry, readable).unwrap();

    let stdout = String::from_utf8_lossy(&discovered.stdout);
    let stderr = String::from_utf8_lossy(&discovered.stderr);
    assert!(
        discovered.status.success(),
        "a discovered pantry that cannot be read must not fail the command\n{stderr}"
    );
    assert!(
        stdout.contains("tomatoes 3"),
        "nothing may be subtracted from a pantry that could not be read: {stdout}"
    );
    assert!(
        stderr.contains("Failed to read pantry file"),
        "the user must be told the pantry was skipped: {stderr}"
    );

    let stderr = String::from_utf8_lossy(&explicit.stderr);
    assert!(
        !explicit.status.success(),
        "a pantry named with --pantry that cannot be read must be fatal"
    );
    assert!(
        stderr.contains("pantry.conf"),
        "the error must name the file it could not read: {stderr}"
    );
}
