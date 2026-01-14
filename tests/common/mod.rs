use anyhow::Result;
use camino::Utf8PathBuf;
use chrono::{Duration, Local};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Creates a temporary directory with test recipes
#[allow(dead_code)]
pub fn setup_test_recipes() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let recipes_dir = temp_dir.path();

    // Create a simple recipe
    fs::write(
        recipes_dir.join("simple.cook"),
        r#"---
title: Simple Recipe
servings: 2
---

Boil @water{2%cups} for ~{5%minutes}.
Add @salt{1%tsp} and @pasta{200%g}.
Cook in a #pot for another ~{10%minutes}.
"#,
    )?;

    // Create a recipe with references
    fs::write(
        recipes_dir.join("with_ref.cook"),
        r#"---
title: Recipe with Reference
---

Make @./sauce{}.
Add @tomatoes{3} to the sauce.
"#,
    )?;

    fs::write(
        recipes_dir.join("sauce.cook"),
        r#"---
title: Basic Sauce
---

Heat @oil{2%tbsp} in a #pan.
Add @garlic{2%cloves} and cook for ~{2%minutes}.
"#,
    )?;

    // Create a recipe with errors for doctor testing
    fs::write(
        recipes_dir.join("with_errors.cook"),
        r#">> title: Recipe with Errors

This has a missing reference @./nonexistent{}.
Add @ingredient with no quantity.
Use deprecated >> metadata.
"#,
    )?;

    // Create a subdirectory with recipes
    let breakfast_dir = recipes_dir.join("Breakfast");
    fs::create_dir(&breakfast_dir)?;

    fs::write(
        breakfast_dir.join("pancakes.cook"),
        r#"---
title: Pancakes
servings: 4
---

Mix @flour{2%cups} with @milk{1.5%cups}.
Add @eggs{2} and @sugar{2%tbsp}.
Cook on a #griddle for ~{3%minutes} per side.
"#,
    )?;

    // Create config directory
    let config_dir = recipes_dir.join("config");
    fs::create_dir(&config_dir)?;

    // Create aisle.conf
    fs::write(
        config_dir.join("aisle.conf"),
        r#"[produce]
tomatoes

[dairy]
milk
eggs

[pantry]
flour
sugar
salt
oil
pasta

[spices]
garlic
"#,
    )?;

    // Create pantry.conf with quantities, low thresholds, and expiry dates
    // Use dynamic dates relative to today
    let today = Local::now().date_naive();
    let tomorrow = today + Duration::days(1);
    let in_2_days = today + Duration::days(2);
    let in_3_days = today + Duration::days(3);
    let in_4_days = today + Duration::days(4);
    let in_10_days = today + Duration::days(10);
    let yesterday = today - Duration::days(1);

    let pantry_config = format!(
        r#"[pantry]
salt = {{ quantity = "1%kg", low = "500%g" }}
oil = {{ quantity = "500%ml", low = "200%ml" }}
flour = {{ quantity = "5%kg", low = "1%kg" }}
sugar = {{ quantity = "2%kg", low = "500%g" }}
pasta = {{ quantity = "1%kg", low = "200%g" }}
water = "always available"

[dairy]
milk = {{ quantity = "1%l", expire = "{}", low = "500%ml" }}
eggs = {{ quantity = "12", expire = "{}", low = "6" }}
butter = {{ quantity = "200%g", low = "50%g" }}
yogurt = {{ quantity = "500%g", expire = "{}" }}

[produce]
tomatoes = {{ quantity = "5", expire = "{}", low = "2" }}
garlic = {{ quantity = "10", low = "3" }}
lettuce = {{ quantity = "1", expire = "{}" }}

[spices]
"black pepper" = {{ quantity = "100%g", low = "20%g" }}
oregano = {{ quantity = "50%g" }}

[depleted]
honey = {{ quantity = "0", low = "100%g" }}
vinegar = {{ quantity = "50%ml", low = "200%ml" }}
"expired item" = {{ quantity = "1", expire = "{}" }}
"#,
        in_4_days.format("%Y-%m-%d"),  // milk expires in 4 days
        in_2_days.format("%Y-%m-%d"),  // eggs expire in 2 days
        tomorrow.format("%Y-%m-%d"),   // yogurt expires tomorrow
        in_3_days.format("%Y-%m-%d"),  // tomatoes expire in 3 days
        in_10_days.format("%Y-%m-%d"), // lettuce expires in 10 days
        yesterday.format("%Y-%m-%d"),  // expired item expired yesterday
    );

    fs::write(config_dir.join("pantry.conf"), pantry_config)?;

    Ok(temp_dir)
}

/// Creates a test Context for commands
#[allow(dead_code)]
pub fn create_test_context(base_path: &Path) -> cookcli::Context {
    let utf8_path =
        Utf8PathBuf::from_path_buf(base_path.to_path_buf()).expect("Path should be UTF-8");
    let parser = std::sync::Arc::new(cooklang::CooklangParser::new(
        cooklang::Extensions::empty(),
        cooklang::Converter::default(),
    ));
    cookcli::Context::new(utf8_path, parser)
}

/// Helper to run a command and capture output
#[allow(dead_code)]
pub fn run_command_with_args<T, F>(temp_dir: &TempDir, args: T, command_fn: F) -> Result<String>
where
    F: FnOnce(&cookcli::Context, T) -> Result<()>,
{
    let ctx = create_test_context(temp_dir.path());
    command_fn(&ctx, args)?;
    Ok(String::new()) // Commands typically print to stdout
}

/// Creates a simple .menu file for testing
#[allow(dead_code)]
pub fn create_test_menu(dir: &Path) -> Result<Utf8PathBuf> {
    let menu_path = dir.join("weekly.menu");
    fs::write(
        &menu_path,
        r#"---
title: Weekly Menu
---

Monday:
- @./Breakfast/pancakes{}

Tuesday:
- @./simple{2}

# Wednesday
- @./with_ref
"#,
    )?;
    Ok(Utf8PathBuf::from_path_buf(menu_path).expect("Path should be UTF-8"))
}

/// Creates test report templates
#[allow(dead_code)]
pub fn create_test_reports(dir: &Path) -> Result<()> {
    let reports_dir = dir.join("Reports");
    fs::create_dir(&reports_dir)?;

    fs::write(
        reports_dir.join("simple.jinja"),
        r#"# Recipe List

{% for recipe in recipes %}
- {{ recipe.name }}
{% endfor %}
"#,
    )?;

    fs::write(
        reports_dir.join("config.yaml"),
        r#"scale: 2
aisle: "../config/aisle.conf"
pantry: "../config/pantry.conf"
"#,
    )?;

    Ok(())
}
