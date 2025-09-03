#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_malformed_recipe_missing_closing_bracket() {
    let temp_dir = TempDir::new().unwrap();

    // Create a malformed recipe with missing closing bracket
    fs::write(
        temp_dir.path().join("malformed.cook"),
        r#">> title: Malformed Recipe

Add @water{2%cups and mix well.
Boil for ~{5%minutes}."#,
    )
    .unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("malformed.cook")
        .assert()
        .success(); // Parser should be lenient
}

#[test]
fn test_recipe_with_invalid_quantities() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("invalid_qty.cook"),
        r#"---
title: Invalid Quantities
---

Add @flour{abc%cups} to bowl.
Mix with @water{-5%ml}."#,
    )
    .unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("invalid_qty.cook")
        .assert()
        .success(); // Should handle gracefully
}

#[test]
fn test_empty_recipe_file() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("empty.cook"), "").unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("empty.cook")
        .assert()
        .success();
}

#[test]
fn test_recipe_with_unicode_characters() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("unicode.cook"),
        r#"---
title: Crème Brûlée 🍮
---

Add @crème{200%ml} and @sucre{50%g}.
Heat in #poêle for ~{5%minutes}.
Garnish with 🍓."#,
    )
    .unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("unicode.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("crème"));
}

#[test]
fn test_very_long_recipe_name() {
    let temp_dir = TempDir::new().unwrap();

    // Use a long but valid filename (most filesystems support 255 chars total)
    let long_name = "a".repeat(240) + ".cook"; // Leave room for .cook extension
    fs::write(temp_dir.path().join(&long_name), "@water{1%cup}").unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg(&long_name)
        .assert()
        .success();
}

#[test]
fn test_recipe_with_circular_reference() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("a.cook"), "Make @./b{}.").unwrap();

    fs::write(temp_dir.path().join("b.cook"), "Make @./a{}.").unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("a.cook")
        .assert()
        .success(); // Should handle circular references
}

#[test]
fn test_recipe_with_missing_reference() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("with_missing.cook"),
        "Make @./nonexistent{}.",
    )
    .unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("with_missing.cook")
        .assert()
        .success(); // Should handle missing references gracefully
}

#[test]
fn test_recipe_with_extreme_scaling() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("normal.cook"), "@water{1%cup}").unwrap();

    // Test with very large scaling factor
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("normal.cook:1000000")
        .assert()
        .success();

    // Test with zero scaling factor - actually succeeds but with 0 quantities
    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("normal.cook:0")
        .assert()
        .success(); // Zero scaling actually works, just shows 0 quantities
}

#[test]
fn test_recipe_with_special_characters_in_path() {
    let temp_dir = TempDir::new().unwrap();

    let special_dir = temp_dir.path().join("special & chars (test)");
    fs::create_dir(&special_dir).unwrap();

    fs::write(special_dir.join("recipe.cook"), "@water{1%cup}").unwrap();

    Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("recipe")
        .arg("read")
        .arg("special & chars (test)/recipe.cook")
        .assert()
        .success();
}
