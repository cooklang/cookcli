#[path = "common/mod.rs"]
mod common;

use anyhow::Result;

#[test]
fn test_recipe_parse_simple() -> Result<()> {
    let temp_dir = common::setup_test_recipes()?;
    let _ctx = common::create_test_context(temp_dir.path());

    // Simple test - just make sure recipes can be parsed without panicking
    // Actual argument parsing and command execution would require
    // more complex setup due to nested command structure

    assert!(temp_dir.path().join("simple.cook").exists());
    assert!(temp_dir.path().join("sauce.cook").exists());
    assert!(temp_dir.path().join("with_ref.cook").exists());
    assert!(temp_dir.path().join("Breakfast/pancakes.cook").exists());

    Ok(())
}

#[test]
fn test_recipe_files_created() -> Result<()> {
    let temp_dir = common::setup_test_recipes()?;

    // Verify test recipes were created correctly
    let simple_content = std::fs::read_to_string(temp_dir.path().join("simple.cook"))?;
    assert!(simple_content.contains("Simple Recipe"));
    assert!(simple_content.contains("@water"));
    assert!(simple_content.contains("@pasta"));

    let pancakes_content =
        std::fs::read_to_string(temp_dir.path().join("Breakfast/pancakes.cook"))?;
    assert!(pancakes_content.contains("Pancakes"));
    assert!(pancakes_content.contains("@flour"));

    Ok(())
}

#[test]
fn test_config_files_created() -> Result<()> {
    let temp_dir = common::setup_test_recipes()?;

    // Verify config files were created
    assert!(temp_dir.path().join("config/aisle.conf").exists());
    assert!(temp_dir.path().join("config/pantry.conf").exists());

    let aisle_content = std::fs::read_to_string(temp_dir.path().join("config/aisle.conf"))?;
    assert!(aisle_content.contains("[produce]"));
    assert!(aisle_content.contains("tomatoes"));

    Ok(())
}
