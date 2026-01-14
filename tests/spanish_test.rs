use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_spanish_units() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();
    let config_dir = base_path.join("config");
    fs::create_dir(&config_dir)?;

    // Create units.toml with Spanish units
    fs::write(
        config_dir.join("units.toml"),
        r#"
[[quantity]]
quantity = "mass"
[quantity.units]
metric = [
    { names = ["gramo", "gramos"], symbols = ["g"], ratio = 1 },
]

[[quantity]]
quantity = "volume"
[quantity.units]
metric = [
    { names = ["litro", "litros"], symbols = ["l"], ratio = 1 },
]
"#,
    )?;

    // Create a recipe using Spanish units
    fs::write(
        base_path.join("paella.cook"),
        r#"Agrega @arroz{500%gramos} y @caldo{1%litro}."#,
    )?;

    let mut cmd = Command::cargo_bin("cook")?;
    cmd.current_dir(base_path)
        .arg("recipe")
        .arg("paella.cook")
        .assert()
        .success()
        .stdout(predicate::str::contains("arroz"))
        .stdout(predicate::str::contains("500 g")) // Normalized to symbol 'g'
        .stdout(predicate::str::contains("litro"))
        .stdout(predicate::str::contains("1 l")); // Normalized to symbol 'l'

    Ok(())
}
