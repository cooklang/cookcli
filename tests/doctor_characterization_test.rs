//! Characterization tests for `cook doctor aisle` and `cook doctor pantry`.
//!
//! Both subcommands had no test coverage at all before this file: the only
//! `doctor` tests were for `validate`. They are pinned here so that moving
//! their logic into `cookcli-core` cannot change what a user sees.

#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// A `cook` invocation rooted at `dir`, so that configuration discovery finds
/// `dir/config/` exactly as it would for a user standing in that directory.
fn cook(dir: &Path) -> Command {
    let mut command = Command::cargo_bin("cook").unwrap();
    command.current_dir(dir);
    command
}

/// A collection of one recipe, with whatever configuration the caller wants.
fn collection(recipe: &str, aisle: Option<&str>, pantry: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("dish.cook"), recipe).unwrap();
    if aisle.is_some() || pantry.is_some() {
        fs::create_dir(dir.path().join("config")).unwrap();
    }
    if let Some(aisle) = aisle {
        fs::write(dir.path().join("config").join("aisle.conf"), aisle).unwrap();
    }
    if let Some(pantry) = pantry {
        fs::write(dir.path().join("config").join("pantry.conf"), pantry).unwrap();
    }
    dir
}

// ---------------------------------------------------------------------------
// doctor aisle
// ---------------------------------------------------------------------------

/// The shared fixture has eleven distinct ingredients across five recipes, of
/// which `water` and the oddly named `ingredient` are not in `aisle.conf`.
#[test]
fn aisle_reports_the_ingredients_missing_from_the_configuration() {
    let dir = common::setup_test_recipes().unwrap();

    cook(dir.path())
        .args(["doctor", "aisle"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Scanned 5 recipes, found 11 unique ingredients",
        ))
        .stdout(predicate::str::contains(
            "2 ingredients not found in aisle configuration:",
        ))
        .stdout(predicate::str::contains("  - ingredient"))
        .stdout(predicate::str::contains("  - water"))
        .stdout(predicate::str::contains(
            "Consider adding these ingredients to your aisle.conf file.",
        ))
        // Everything else is categorised, and must not be reported.
        .stdout(predicate::str::contains("  - salt").not())
        .stdout(predicate::str::contains("  - flour").not());
}

#[test]
fn aisle_says_so_when_every_ingredient_is_categorised() {
    let dir = collection("Boil @water{1%l}.\n", Some("[pantry]\nwater\n"), None);

    cook(dir.path())
        .args(["doctor", "aisle"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Scanned 1 recipes, found 1 unique ingredients",
        ))
        .stdout(predicate::str::contains(
            "✓ All ingredients are present in aisle configuration",
        ));
}

/// A name spelled differently in the recipe and the configuration is still the
/// same ingredient.
#[test]
fn aisle_matches_names_ignoring_case() {
    let dir = collection(
        "Add @Salt{1%tsp} and @PEPPER{}.\n",
        Some("[pantry]\nsalt\npepper\n"),
        None,
    );

    cook(dir.path())
        .args(["doctor", "aisle"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "✓ All ingredients are present in aisle configuration",
        ));
}

/// A reference to another recipe is something to cook, not something to buy,
/// so it is never reported as an uncategorised ingredient.
#[test]
fn aisle_ignores_references_to_other_recipes() {
    let dir = collection(
        "Make @./sauce{} and add @water{1%l}.\n",
        Some("[pantry]\nwater\n"),
        None,
    );
    fs::write(dir.path().join("sauce.cook"), "Heat @water{1%l}.\n").unwrap();

    cook(dir.path())
        .args(["doctor", "aisle"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Scanned 2 recipes, found 1 unique ingredients",
        ))
        .stdout(predicate::str::contains(
            "✓ All ingredients are present in aisle configuration",
        ));
}

#[test]
fn aisle_scans_recipes_in_subdirectories() {
    let dir = collection("Boil @water{1%l}.\n", Some("[pantry]\nwater\n"), None);
    fs::create_dir(dir.path().join("Breakfast")).unwrap();
    fs::write(
        dir.path().join("Breakfast").join("porridge.cook"),
        "Simmer @oats{50%g}.\n",
    )
    .unwrap();

    cook(dir.path())
        .args(["doctor", "aisle"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Scanned 2 recipes, found 2 unique ingredients",
        ))
        .stdout(predicate::str::contains(
            "1 ingredients not found in aisle configuration:",
        ))
        .stdout(predicate::str::contains("  - oats"));
}

/// With nothing to check against, the collection is still scanned and reported
/// on before the missing configuration is explained.
#[test]
fn aisle_without_a_configuration_still_scans_and_explains_itself() {
    let dir = collection("Boil @water{1%l}.\n", None, None);

    cook(dir.path())
        .args(["doctor", "aisle"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Scanned 1 recipes, found 1 unique ingredients",
        ))
        .stdout(predicate::str::contains("No aisle configuration found."))
        .stdout(predicate::str::contains(
            "  - ./config/aisle.conf (project-specific)",
        ))
        .stdout(predicate::str::contains(
            "  - ~/.config/cook/aisle.conf (global)",
        ));
}

// ---------------------------------------------------------------------------
// doctor pantry
// ---------------------------------------------------------------------------

/// Ten of the fixture's eleven ingredients are in the pantry; the eleventh,
/// `ingredient`, is not, and is not listed.
#[test]
fn pantry_lists_the_recipe_ingredients_that_are_in_stock() {
    let dir = common::setup_test_recipes().unwrap();

    cook(dir.path())
        .args(["doctor", "pantry"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Scanned 5 recipes, found 11 unique ingredients",
        ))
        .stdout(predicate::str::contains(
            "10 ingredients from recipes are in your pantry:",
        ))
        .stdout(predicate::str::contains("  ✓ water"))
        .stdout(predicate::str::contains("  ✓ flour"))
        .stdout(predicate::str::contains(
            "These ingredients will be excluded from shopping lists.",
        ))
        // In no recipe, so not reported however much else is in stock.
        .stdout(predicate::str::contains("  ✓ butter").not())
        // In a recipe, but not in the pantry.
        .stdout(predicate::str::contains("  ✓ ingredient").not());
}

#[test]
fn pantry_says_so_when_no_ingredient_is_in_stock() {
    let dir = collection(
        "Boil @water{1%l}.\n",
        None,
        Some("[pantry]\nflour = \"1%kg\"\n"),
    );

    cook(dir.path())
        .args(["doctor", "pantry"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Scanned 1 recipes, found 1 unique ingredients",
        ))
        .stdout(predicate::str::contains(
            "✓ No recipe ingredients are currently in your pantry",
        ));
}

#[test]
fn pantry_matches_names_ignoring_case() {
    let dir = collection(
        "Add @Salt{1%tsp}.\n",
        None,
        Some("[pantry]\nsalt = \"1%kg\"\n"),
    );

    cook(dir.path())
        .args(["doctor", "pantry"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "1 ingredients from recipes are in your pantry:",
        ))
        .stdout(predicate::str::contains("  ✓ Salt"));
}

/// A referenced recipe is not an ingredient, even when the pantry happens to
/// hold something of the same name.
#[test]
fn pantry_ignores_references_to_other_recipes() {
    let dir = collection(
        "Make @./sauce{} and add @water{1%l}.\n",
        None,
        Some("[pantry]\nsauce = \"1%l\"\nwater = \"1%l\"\n"),
    );
    fs::write(dir.path().join("sauce.cook"), "Heat @water{1%l}.\n").unwrap();

    cook(dir.path())
        .args(["doctor", "pantry"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "1 ingredients from recipes are in your pantry:",
        ))
        .stdout(predicate::str::contains("  ✓ water"))
        .stdout(predicate::str::contains("  ✓ sauce").not());
}

#[test]
fn pantry_scans_recipes_in_subdirectories() {
    let dir = collection(
        "Boil @water{1%l}.\n",
        None,
        Some("[pantry]\noats = \"500%g\"\n"),
    );
    fs::create_dir(dir.path().join("Breakfast")).unwrap();
    fs::write(
        dir.path().join("Breakfast").join("porridge.cook"),
        "Simmer @oats{50%g}.\n",
    )
    .unwrap();

    cook(dir.path())
        .args(["doctor", "pantry"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Scanned 2 recipes, found 2 unique ingredients",
        ))
        .stdout(predicate::str::contains("  ✓ oats"));
}

/// Unlike the aisle check, this one explains itself instead of scanning: with
/// no pantry there is nothing for a scan to be compared against.
#[test]
fn pantry_without_a_configuration_explains_itself_instead_of_scanning() {
    let dir = collection("Boil @water{1%l}.\n", None, None);

    cook(dir.path())
        .args(["doctor", "pantry"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No pantry configuration found."))
        .stdout(predicate::str::contains(
            "  - ./config/pantry.conf (project-specific)",
        ))
        .stdout(predicate::str::contains(
            "  - ~/.config/cook/pantry.conf (global)",
        ))
        .stdout(predicate::str::contains(
            "Example pantry.conf (TOML format):",
        ))
        .stdout(predicate::str::contains("Scanned").not());
}

// ---------------------------------------------------------------------------
// doctor, with no subcommand
// ---------------------------------------------------------------------------

/// A configuration too broken to parse fails its own check, and the checks
/// after it still run. `cook doctor`'s whole job is reporting problems, so one
/// broken file must not hide what the rest would have said.
#[test]
fn doctor_with_no_subcommand_carries_on_past_a_check_it_cannot_run() {
    let dir = collection(
        "Boil @water{1%l}.\n",
        Some("[pantry]\nwater\n"),
        Some("this is not toml ["),
    );

    cook(dir.path())
        .arg("doctor")
        .assert()
        // The run as a whole still succeeds: `cook doctor` has no --strict, and
        // reporting the problem is the point.
        .success()
        .stdout(predicate::str::contains("=== Aisle Check ==="))
        .stdout(predicate::str::contains(
            "✓ All ingredients are present in aisle configuration",
        ))
        .stdout(predicate::str::contains("=== Pantry Check ==="))
        .stdout(predicate::str::contains("This check could not run:"))
        // The parser's own cause must survive as far as the user.
        .stdout(predicate::str::contains("TOML parse error"));
}

/// The same broken pantry asked for on its own is a failure, because a script
/// asking one question wants to know it got no answer.
#[test]
fn doctor_pantry_on_its_own_fails_on_a_configuration_it_cannot_parse() {
    let dir = collection("Boil @water{1%l}.\n", None, Some("this is not toml ["));

    cook(dir.path())
        .args(["doctor", "pantry"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TOML parse error"));
}

/// Pins a **known oddity**, so that fixing it has to be deliberate: a broken
/// recipe reference is counted into the error total but belongs to no recipe,
/// so the summary reads "in 0 recipe(s)". Reported as
/// <https://github.com/cooklang/cookcli/issues> rather than fixed here — this
/// test is the tripwire, not an endorsement.
#[test]
fn a_broken_reference_counts_as_an_error_belonging_to_no_recipe() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("dish.cook"),
        "---\ntitle: Dish\n---\n\nMake @./nonexistent{}.\n",
    )
    .unwrap();

    cook(dir.path())
        .args(["doctor", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Recipe References ==="))
        .stdout(predicate::str::contains(
            "  ❌ Missing reference: ./nonexistent",
        ))
        .stdout(predicate::str::contains(
            "❌ 1 error(s) found in 0 recipe(s)",
        ));
}

/// ...and it is an error for `--strict`, which is what CI gates on.
#[test]
fn a_broken_reference_fails_a_strict_validation() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("dish.cook"),
        "---\ntitle: Dish\n---\n\nMake @./nonexistent{}.\n",
    )
    .unwrap();

    cook(dir.path())
        .args(["doctor", "validate", "--strict"])
        .assert()
        .failure();
}

/// Every check runs, in order, off one invocation.
#[test]
fn doctor_with_no_subcommand_runs_every_check() {
    let dir = common::setup_test_recipes().unwrap();

    cook(dir.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Running all doctor checks..."))
        .stdout(predicate::str::contains("=== Version Check ==="))
        .stdout(predicate::str::contains("=== Recipe Validation ==="))
        .stdout(predicate::str::contains("Total recipes scanned: 5"))
        .stdout(predicate::str::contains("=== Aisle Check ==="))
        .stdout(predicate::str::contains(
            "2 ingredients not found in aisle configuration:",
        ))
        .stdout(predicate::str::contains("=== Pantry Check ==="))
        .stdout(predicate::str::contains(
            "10 ingredients from recipes are in your pantry:",
        ));
}
