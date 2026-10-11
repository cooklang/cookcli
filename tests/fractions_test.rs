//! Fractions read the same in every human-readable output (#672).
//!
//! The web UI showed `5/8 cup` where `cook recipe` printed `0.625 cup`, and
//! both printed `1.625 cup` for `1 5/8 cup`. Every text output now goes
//! through `cooklang_format::number`, so these are the numbers each shows; the
//! JSON keeps the plain number. Grams and litres stay in decimals.

#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use std::fs;

const RECIPE: &str = "Mix @milk{1 5/8%cup}, @cocoa{5/8%cup} and @sugar{3/4%cup}.\n";

fn cook_in(dir: &std::path::Path) -> Command {
    let mut cmd = std::process::Command::new(assert_cmd::cargo::cargo_bin("cook"));
    common::with_isolated_config(&mut cmd, dir);
    cmd.current_dir(dir);
    Command::from_std(cmd)
}

fn recipe_dir() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("units.cook"), RECIPE).unwrap();
    dir
}

fn stdout(cmd: &mut Command) -> String {
    let output = cmd.output().unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn the_terminal_shows_exact_fractions() {
    let dir = recipe_dir();
    let out = stdout(cook_in(dir.path()).args(["recipe", "read", "units.cook"]));

    for expected in ["1 5/8 cup", "5/8 cup", "3/4 cup"] {
        assert!(out.contains(expected), "missing {expected:?} in:\n{out}");
    }
    assert!(!out.contains("0.625"), "{out}");
    assert!(!out.contains("1.625"), "{out}");
}

#[test]
fn markdown_shows_exact_fractions() {
    let dir = recipe_dir();
    let out = stdout(cook_in(dir.path()).args(["recipe", "read", "-f", "markdown", "units.cook"]));

    assert!(out.contains("*1 5/8 cup* milk"), "{out}");
    assert!(out.contains("*5/8 cup* cocoa"), "{out}");
}

/// Scaled by 1.5, `5/8` becomes `15/16`, which is not one of the fractions: it
/// stays a decimal rather than being shown as the nearest one.
#[test]
fn a_value_between_fractions_stays_a_decimal() {
    let dir = recipe_dir();
    let out = stdout(cook_in(dir.path()).args(["recipe", "read", "units.cook:1.5"]));

    assert!(out.contains("0.938 cup"), "{out}");
    assert!(out.contains("2.438 cup"), "{out}");
    assert!(!out.contains("2 7/16"), "{out}");
}

#[test]
fn the_shopping_list_shows_exact_fractions() {
    let dir = recipe_dir();
    let out = stdout(cook_in(dir.path()).args([
        "shopping-list",
        "--ignore-pantry",
        "--plain",
        "units.cook",
    ]));

    assert!(out.contains("1 5/8 cup"), "{out}");
    assert!(out.contains("5/8 cup"), "{out}");
}

/// Fractions are for cups and spoons: grams and litres read in decimals, in
/// the terminal and on the shopping list alike.
#[test]
fn grams_and_litres_stay_decimals() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(
        dir.path().join("metric.cook"),
        "Mix @flour{100.625%g}, @butter{0.5%kg} and @milk{1.5%l}.\n",
    )
    .unwrap();

    let recipe = stdout(cook_in(dir.path()).args(["recipe", "read", "metric.cook"]));
    let list = stdout(cook_in(dir.path()).args([
        "shopping-list",
        "--ignore-pantry",
        "--plain",
        "metric.cook",
    ]));
    for out in [recipe, list] {
        for expected in ["100.625 g", "0.5 kg", "1.5 l"] {
            assert!(out.contains(expected), "missing {expected:?} in:\n{out}");
        }
        assert!(!out.contains("5/8"), "{out}");
        assert!(!out.contains("1/2"), "{out}");
    }
}

#[test]
fn json_keeps_the_plain_number() {
    let dir = recipe_dir();
    let out = stdout(cook_in(dir.path()).args(["recipe", "read", "-f", "json", "units.cook"]));

    assert!(out.contains("1.625"), "{out}");
    assert!(!out.contains("5/8"), "{out}");
}
