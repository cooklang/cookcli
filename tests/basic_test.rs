#[path = "common/mod.rs"]
mod common;

use anyhow::Result;

#[test]
fn test_setup_recipes() -> Result<()> {
    let temp_dir = common::setup_test_recipes()?;

    // Test that basic recipe files are created
    assert!(temp_dir.path().join("simple.cook").exists());
    assert!(temp_dir.path().join("sauce.cook").exists());
    assert!(temp_dir.path().join("with_ref.cook").exists());
    assert!(temp_dir.path().join("with_errors.cook").exists());
    assert!(temp_dir.path().join("Breakfast/pancakes.cook").exists());

    // Test that config files are created
    assert!(temp_dir.path().join("config/aisle.conf").exists());
    assert!(temp_dir.path().join("config/pantry.conf").exists());

    Ok(())
}

#[test]
fn test_context_creation() -> Result<()> {
    let temp_dir = common::setup_test_recipes()?;
    let ctx = common::create_test_context(temp_dir.path());

    // Test that context has correct base path
    assert_eq!(ctx.base_path().as_str(), temp_dir.path().to_str().unwrap());

    // Test that aisle and pantry are found
    assert!(ctx.aisle().is_some());
    assert!(ctx.pantry().is_some());

    Ok(())
}

#[test]
fn test_menu_creation() -> Result<()> {
    let temp_dir = common::setup_test_recipes()?;
    let menu_path = common::create_test_menu(temp_dir.path())?;

    assert!(menu_path.exists());

    let content = std::fs::read_to_string(&menu_path)?;
    assert!(content.contains("Weekly Menu"));
    assert!(content.contains("pancakes"));
    assert!(content.contains("simple{2}"));

    Ok(())
}

#[test]
fn test_report_templates() -> Result<()> {
    let temp_dir = common::setup_test_recipes()?;
    common::create_test_reports(temp_dir.path())?;

    assert!(temp_dir.path().join("Reports/simple.jinja").exists());
    assert!(temp_dir.path().join("Reports/config.yaml").exists());

    Ok(())
}
