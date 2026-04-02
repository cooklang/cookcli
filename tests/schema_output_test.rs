#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use serde_json::Value as JsonValue;
use std::fs;
use tempfile::TempDir;

/// Helper to create a temp dir with a single .cook file and run schema output
fn schema_output(recipe_content: &str) -> JsonValue {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("test.cook"), recipe_content).unwrap();

    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["recipe", "read", "-f", "schema", "test.cook"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "cook failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    serde_json::from_str(&stdout).expect("Invalid JSON-LD output")
}

#[test]
fn test_schema_step_has_position_not_name() {
    let json = schema_output("Add @salt{1%tsp} to @water{2%cups}.");

    let instructions = json["recipeInstructions"].as_array().unwrap();
    assert_eq!(instructions.len(), 1);

    let step = &instructions[0];
    assert_eq!(step["@type"], "HowToStep");
    assert_eq!(step["position"], 1);
    assert!(
        step.get("name").is_none(),
        "HowToStep should use position, not name"
    );
}

#[test]
fn test_schema_step_position_increments() {
    let json =
        schema_output("Boil @water{2%cups}.\n\nAdd @pasta{200%g} and cook.\n\nDrain and serve.");

    let instructions = json["recipeInstructions"].as_array().unwrap();
    assert_eq!(instructions.len(), 3);
    assert_eq!(instructions[0]["position"], 1);
    assert_eq!(instructions[1]["position"], 2);
    assert_eq!(instructions[2]["position"], 3);
}

#[test]
fn test_schema_step_time_required_from_timer() {
    let json = schema_output("Bake for ~{40%minutes}.");

    let step = &json["recipeInstructions"][0];
    assert_eq!(step["timeRequired"], "PT40M");
}

#[test]
fn test_schema_step_time_required_hours() {
    let json = schema_output("Slow cook for ~{2%hours}.");

    let step = &json["recipeInstructions"][0];
    assert_eq!(step["timeRequired"], "PT2H");
}

#[test]
fn test_schema_step_time_required_seconds() {
    let json = schema_output("Microwave for ~{30%seconds}.");

    let step = &json["recipeInstructions"][0];
    assert_eq!(step["timeRequired"], "PT30S");
}

#[test]
fn test_schema_step_multiple_timers_summed() {
    let json = schema_output("Cook for ~{10%minutes}, then rest for ~{5%minutes}.");

    let step = &json["recipeInstructions"][0];
    assert_eq!(step["timeRequired"], "PT15M");
}

#[test]
fn test_schema_step_no_timer_no_time_required() {
    let json = schema_output("Mix @flour{2%cups} with @sugar{1%cup}.");

    let step = &json["recipeInstructions"][0];
    assert!(
        step.get("timeRequired").is_none(),
        "Steps without timers should not have timeRequired"
    );
}

#[test]
fn test_schema_sections_produce_how_to_section() {
    let json = schema_output(
        "\
== Prep ==

Chop @onion{1}.

== Cook ==

Fry @onion in a #pan{} for ~{5%minutes}.
",
    );

    let instructions = json["recipeInstructions"].as_array().unwrap();
    assert_eq!(instructions.len(), 2);

    // First section
    assert_eq!(instructions[0]["@type"], "HowToSection");
    assert_eq!(instructions[0]["name"], "Prep");
    let items0 = instructions[0]["itemListElement"].as_array().unwrap();
    assert_eq!(items0.len(), 1);
    assert_eq!(items0[0]["@type"], "HowToStep");
    assert_eq!(items0[0]["position"], 1);

    // Second section
    assert_eq!(instructions[1]["@type"], "HowToSection");
    assert_eq!(instructions[1]["name"], "Cook");
    let items1 = instructions[1]["itemListElement"].as_array().unwrap();
    assert_eq!(items1.len(), 1);
    assert_eq!(items1[0]["@type"], "HowToStep");
    assert_eq!(items1[0]["position"], 2);
    assert_eq!(items1[0]["timeRequired"], "PT5M");
}

#[test]
fn test_schema_no_sections_flat_list() {
    let json = schema_output("Step one.\n\nStep two.");

    let instructions = json["recipeInstructions"].as_array().unwrap();
    assert_eq!(instructions.len(), 2);

    // Should be flat HowToStep, not wrapped in HowToSection
    assert_eq!(instructions[0]["@type"], "HowToStep");
    assert_eq!(instructions[1]["@type"], "HowToStep");
}

#[test]
fn test_schema_context_and_type() {
    let json = schema_output("Just a step.");

    assert_eq!(json["@context"], "https://schema.org");
    assert_eq!(json["@type"], "Recipe");
}

#[test]
fn test_schema_issue_291_example() {
    let json = schema_output(
        "\
---
title: Maple Chili Sweet Potatoes
cuisine: mediterranean
tags:
  - vegan
image: https://example.com/image.jpg
---

== Preparation ==

Whisk @olive oil{2%tbsp}, @maple syrup{2%tbsp}, @chili powder{1%tsp}, @cayenne pepper{1/4%tsp}, and @salt{1/4%tsp} together.

== Cooking ==

Toss the glaze with @sweet potatoes{2%medium}(cut into 1.5 cm chunks) and spread on a #baking sheet{}.
Roast at 220°C, stirring halfway through, until golden brown and tender, ~{40%minutes}.
",
    );

    // Metadata
    assert_eq!(json["name"], "Maple Chili Sweet Potatoes");
    assert_eq!(json["recipeCuisine"], "mediterranean");
    assert_eq!(json["keywords"], "vegan");
    assert_eq!(json["image"], "https://example.com/image.jpg");

    // Ingredients
    let ingredients = json["recipeIngredient"].as_array().unwrap();
    assert_eq!(ingredients.len(), 6);

    // Tools
    let tools = json["tool"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0], "baking sheet");

    // Sections
    let instructions = json["recipeInstructions"].as_array().unwrap();
    assert_eq!(instructions.len(), 2);
    assert_eq!(instructions[0]["@type"], "HowToSection");
    assert_eq!(instructions[0]["name"], "Preparation");
    assert_eq!(instructions[1]["@type"], "HowToSection");
    assert_eq!(instructions[1]["name"], "Cooking");

    // Timer on cooking step
    let cooking_steps = instructions[1]["itemListElement"].as_array().unwrap();
    assert_eq!(cooking_steps[0]["timeRequired"], "PT40M");
}

#[test]
fn test_schema_mixed_named_unnamed_sections() {
    // Leading unnamed section followed by a named section
    let json = schema_output(
        "\
Boil @water{2%cups}.

== Sauce ==

Simmer @tomatoes{3} for ~{20%minutes}.
",
    );

    let instructions = json["recipeInstructions"].as_array().unwrap();
    // Unnamed section steps should be flat HowToStep, named section wrapped
    assert_eq!(instructions.len(), 2);
    assert_eq!(instructions[0]["@type"], "HowToStep");
    assert_eq!(instructions[0]["position"], 1);
    assert_eq!(instructions[1]["@type"], "HowToSection");
    assert_eq!(instructions[1]["name"], "Sauce");
}

#[test]
fn test_schema_multi_unit_timers_summed() {
    // 1 hour + 30 minutes = PT1H30M
    let json = schema_output("Cook for ~{1%hour} then rest ~{30%minutes}.");

    let step = &json["recipeInstructions"][0];
    assert_eq!(step["timeRequired"], "PT1H30M");
}

#[test]
fn test_schema_unknown_timer_unit_skipped() {
    // Unknown unit "days" should not contribute to timeRequired
    let json = schema_output("Marinate for ~{2%days}.");

    let step = &json["recipeInstructions"][0];
    assert!(
        step.get("timeRequired").is_none(),
        "Unknown timer units should not produce timeRequired"
    );
}

#[test]
fn test_schema_hours_and_minutes_formatting() {
    // 90 minutes should be PT1H30M, not PT90M
    let json = schema_output("Bake for ~{90%minutes}.");

    let step = &json["recipeInstructions"][0];
    assert_eq!(step["timeRequired"], "PT1H30M");
}
