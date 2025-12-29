use anyhow::{Context, Result};
use chrono::prelude::*;
use clap::{Args, Subcommand, ValueEnum};
use cooklang::pantry::PantryItem;
use cooklang_find::build_tree;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::{util::parse_recipe_from_entry, Context as AppContext};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable output (default)
    Human,
    /// JSON output
    Json,
    /// YAML output
    Yaml,
}

#[derive(Debug, Args)]
pub struct PantryArgs {
    /// Base path for recipes and configuration files
    #[arg(short = 'b', long, value_name = "PATH")]
    pub base_path: Option<camino::Utf8PathBuf>,

    /// Output format
    #[arg(short = 'f', long, value_enum, default_value = "human")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: PantryCommand,
}

#[derive(Debug, Subcommand)]
pub enum PantryCommand {
    /// Show items that are out of stock or have low quantities
    #[command(alias = "d")]
    Depleted(DepletedArgs),

    /// Show items that are expiring soon
    #[command(alias = "e")]
    Expiring(ExpiringArgs),

    /// List recipes that can be made with items currently in pantry
    #[command(alias = "r")]
    Recipes(RecipesArgs),

    /// Analyze ingredient usage across recipes to help plan pantry items
    #[command(alias = "pl")]
    Plan(PlanArgs),
}

#[derive(Debug, Args)]
pub struct DepletedArgs {
    /// Show all items including those without quantities
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug, Args)]
pub struct ExpiringArgs {
    /// Number of days to look ahead for expiring items (default: 7)
    #[arg(short = 'd', long, default_value = "7")]
    pub days: u32,

    /// Include items without expiry dates
    #[arg(long)]
    pub include_unknown: bool,
}

#[derive(Debug, Args)]
pub struct RecipesArgs {
    /// Include partial matches (recipes where most ingredients are available)
    #[arg(short = 'p', long)]
    pub partial: bool,

    /// Minimum percentage of ingredients that must be available for partial matches (default: 75)
    #[arg(long, default_value = "75")]
    pub threshold: u8,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    /// Maximum number of ingredients to show (default: show all needed for 100% coverage)
    #[arg(short = 'n', long)]
    pub max_ingredients: Option<usize>,

    /// Skip the first N ingredients (useful if you already have common items)
    #[arg(short = 's', long, default_value = "0")]
    pub skip: usize,

    /// Allow recipes to be considered cookable even if N ingredients are missing
    #[arg(short = 'm', long, default_value = "0")]
    pub allow_missing: usize,
}

// Output structures for JSON/YAML formats
#[derive(Debug, Serialize)]
struct DepletedOutput {
    items: Vec<DepletedItem>,
}

#[derive(Debug, Serialize)]
struct DepletedItem {
    name: String,
    section: String,
    quantity: Option<String>,
    low_threshold: Option<String>,
    is_low: bool,
}

#[derive(Debug, Serialize)]
struct ExpiringOutput {
    items: Vec<ExpiringItem>,
}

#[derive(Debug, Serialize)]
struct ExpiringItem {
    name: String,
    section: String,
    expire_date: Option<String>,
    days_until_expiry: Option<i64>,
    status: String,
}

#[derive(Debug, Serialize)]
struct RecipesOutput {
    full_matches: Vec<String>,
    partial_matches: Vec<PartialMatch>,
}

#[derive(Debug, Serialize)]
struct PartialMatch {
    recipe: String,
    percentage: usize,
    missing_ingredients: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PlanOutput {
    total_recipes: usize,
    cookable_recipes: usize,
    coverage_percentage: usize,
    ingredients: Vec<IngredientStep>,
}

#[derive(Debug, Serialize)]
struct IngredientStep {
    name: String,
    new_recipes_unlocked: usize,
    total_cookable: usize,
}

pub fn run(ctx: &AppContext, args: PantryArgs) -> Result<()> {
    // Create a new context with the provided base path if specified
    let new_ctx;
    let ctx = if let Some(base_path) = args.base_path {
        let absolute_base_path = crate::util::resolve_to_absolute_path(&base_path)?;
        new_ctx = AppContext::new(absolute_base_path);
        &new_ctx
    } else {
        ctx
    };

    let format = args.format;

    match args.command {
        PantryCommand::Depleted(depleted_args) => run_depleted(ctx, depleted_args, format),
        PantryCommand::Expiring(expiring_args) => run_expiring(ctx, expiring_args, format),
        PantryCommand::Recipes(recipes_args) => run_recipes(ctx, recipes_args, format),
        PantryCommand::Plan(plan_args) => run_plan(ctx, plan_args, format),
    }
}

fn run_depleted(ctx: &AppContext, args: DepletedArgs, format: OutputFormat) -> Result<()> {
    let pantry_path = ctx
        .pantry()
        .ok_or_else(|| anyhow::anyhow!("No pantry configuration found"))?;
    let content = std::fs::read_to_string(&pantry_path)
        .with_context(|| format!("Failed to read pantry file at {pantry_path}"))?;

    let result = cooklang::pantry::parse_lenient(&content);
    let pantry_conf = result.output().ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to parse pantry configuration: {:?}",
            result.report()
        )
    })?;

    let mut depleted_items = Vec::new();

    for (section, items) in &pantry_conf.sections {
        for item in items {
            let is_low = item.is_low();
            let should_show = if is_low {
                true
            } else {
                match item {
                    PantryItem::Simple(_) => args.all,
                    PantryItem::WithAttributes(attrs) => {
                        // Check if user set an explicit low threshold
                        if let Some(low) = &attrs.low {
                            // User has set a threshold
                            // Only trust it if the units match (so we can actually compare them)
                            if let Some(quantity) = &attrs.quantity {
                                if units_match(quantity, low) {
                                    // Units match, trust the threshold comparison result
                                    args.all
                                } else {
                                    // Units don't match, use default rules instead
                                    is_low_quantity(quantity)
                                }
                            } else {
                                args.all
                            }
                        } else {
                            // No explicit threshold set, use default rules
                            if let Some(quantity) = &attrs.quantity {
                                is_low_quantity(quantity)
                            } else {
                                args.all
                            }
                        }
                    }
                }
            };

            if should_show {
                let quantity = item.quantity().map(|q| q.to_string());
                let low_threshold = item.low().map(|l| l.to_string());

                depleted_items.push(DepletedItem {
                    name: item.name().to_string(),
                    section: section.clone(),
                    quantity,
                    low_threshold,
                    is_low,
                });
            }
        }
    }

    match format {
        OutputFormat::Human => {
            if depleted_items.is_empty() {
                println!("No depleted items found!");
            } else {
                println!("Depleted or Low Stock Items:");
                println!("============================");

                let mut current_section = String::new();
                for item in &depleted_items {
                    if item.section != current_section {
                        println!("\n{}:", item.section.to_uppercase());
                        current_section = item.section.clone();
                    }
                    print!("  • {}", item.name);
                    if let Some(ref qty) = item.quantity {
                        print!(" ({qty})");
                    }
                    if let Some(ref low) = item.low_threshold {
                        print!(" [low when < {low}]");
                    }
                    println!();
                }
            }
        }
        OutputFormat::Json => {
            let output = DepletedOutput {
                items: depleted_items,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Yaml => {
            let output = DepletedOutput {
                items: depleted_items,
            };
            println!("{}", serde_yaml::to_string(&output)?);
        }
    }

    Ok(())
}

fn run_expiring(ctx: &AppContext, args: ExpiringArgs, format: OutputFormat) -> Result<()> {
    let pantry_path = ctx
        .pantry()
        .ok_or_else(|| anyhow::anyhow!("No pantry configuration found"))?;
    let content = std::fs::read_to_string(&pantry_path)
        .with_context(|| format!("Failed to read pantry file at {pantry_path}"))?;

    let result = cooklang::pantry::parse_lenient(&content);
    let pantry_conf = result.output().ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to parse pantry configuration: {:?}",
            result.report()
        )
    })?;

    let today = Local::now().date_naive();
    let threshold_date = today + chrono::Duration::days(args.days as i64);

    let mut expiring_list = Vec::new();

    for (section, items) in &pantry_conf.sections {
        for item in items {
            let expire_date = item.expire().and_then(parse_date);

            if let Some(date) = expire_date {
                if date <= threshold_date {
                    let days_until = (date - today).num_days();
                    let status = if days_until < 0 {
                        format!("EXPIRED {} days ago", -days_until)
                    } else if days_until == 0 {
                        "EXPIRES TODAY".to_string()
                    } else if days_until == 1 {
                        "expires tomorrow".to_string()
                    } else {
                        format!("expires in {days_until} days")
                    };

                    expiring_list.push(ExpiringItem {
                        name: item.name().to_string(),
                        section: section.clone(),
                        expire_date: Some(date.format("%Y-%m-%d").to_string()),
                        days_until_expiry: Some(days_until),
                        status,
                    });
                }
            } else if args.include_unknown {
                expiring_list.push(ExpiringItem {
                    name: item.name().to_string(),
                    section: section.clone(),
                    expire_date: None,
                    days_until_expiry: None,
                    status: "No expiry date".to_string(),
                });
            }
        }
    }

    // Sort by days until expiry
    expiring_list.sort_by_key(|item| item.days_until_expiry.unwrap_or(i64::MAX));

    match format {
        OutputFormat::Human => {
            println!("Items Expiring Within {} Days:", args.days);
            println!("================================");

            let with_dates: Vec<_> = expiring_list
                .iter()
                .filter(|i| i.expire_date.is_some())
                .collect();
            let without_dates: Vec<_> = expiring_list
                .iter()
                .filter(|i| i.expire_date.is_none())
                .collect();

            if !with_dates.is_empty() {
                println!("\nExpiring Soon:");
                for item in &with_dates {
                    println!(
                        "  • {} - {} ({}) [{}]",
                        item.name,
                        item.expire_date.as_ref().unwrap(),
                        item.status,
                        item.section
                    );
                }
            }

            if !without_dates.is_empty() {
                println!("\nNo Expiry Date Set:");
                for item in &without_dates {
                    println!("  • {} [{}]", item.name, item.section);
                }
            }

            if expiring_list.is_empty() {
                println!("\nNo expiring items found!");
            }
        }
        OutputFormat::Json => {
            let output = ExpiringOutput {
                items: expiring_list,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Yaml => {
            let output = ExpiringOutput {
                items: expiring_list,
            };
            println!("{}", serde_yaml::to_string(&output)?);
        }
    }

    Ok(())
}

fn run_recipes(ctx: &AppContext, args: RecipesArgs, format: OutputFormat) -> Result<()> {
    let pantry_path = ctx
        .pantry()
        .ok_or_else(|| anyhow::anyhow!("No pantry configuration found"))?;
    let content = std::fs::read_to_string(&pantry_path)
        .with_context(|| format!("Failed to read pantry file at {pantry_path}"))?;

    let result = cooklang::pantry::parse_lenient(&content);
    let pantry_conf = result.output().ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to parse pantry configuration: {:?}",
            result.report()
        )
    })?;

    // Build a set of available ingredients (normalized to lowercase)
    let mut pantry_ingredients = HashSet::new();
    for items in pantry_conf.sections.values() {
        for item in items {
            pantry_ingredients.insert(item.name().to_lowercase());
        }
    }

    // Build recipe tree
    let tree = build_tree(ctx.base_path()).context("Failed to build recipe tree")?;

    let mut full_matches = Vec::new();
    let mut partial_matches_raw = Vec::new();

    // Recursively process recipes in the tree
    fn process_tree(
        tree: &cooklang_find::RecipeTree,
        pantry_ingredients: &HashSet<String>,
        full_matches: &mut Vec<String>,
        partial_matches: &mut Vec<(String, usize, Vec<String>)>,
        args: &RecipesArgs,
    ) {
        // Check if this node has a recipe
        if let Some(entry) = &tree.recipe {
            // Parse the recipe
            if let Ok(recipe) = parse_recipe_from_entry(entry, 1.0) {
                // Get all ingredients from the recipe (excluding recipe references)
                let mut recipe_ingredients = HashSet::new();
                for ingredient in &recipe.ingredients {
                    // Skip recipe references
                    if ingredient.reference.is_some() {
                        continue;
                    }
                    if ingredient.modifiers().should_be_listed() {
                        recipe_ingredients
                            .insert(ingredient.display_name().to_string().to_lowercase());
                    }
                }

                if !recipe_ingredients.is_empty() {
                    // Check how many ingredients are available in pantry
                    let available_count = recipe_ingredients
                        .iter()
                        .filter(|ing| pantry_ingredients.contains(*ing))
                        .count();

                    let total_count = recipe_ingredients.len();
                    let percentage = (available_count * 100) / total_count;

                    let recipe_name = entry.name().as_deref().unwrap_or("unknown").to_string();

                    if available_count == total_count {
                        full_matches.push(recipe_name);
                    } else if args.partial && percentage >= args.threshold as usize {
                        let missing: Vec<_> = recipe_ingredients
                            .iter()
                            .filter(|ing| !pantry_ingredients.contains(*ing))
                            .cloned()
                            .collect();
                        partial_matches.push((recipe_name, percentage, missing));
                    }
                }
            }
        }

        // Recursively check children
        for subtree in tree.children.values() {
            process_tree(
                subtree,
                pantry_ingredients,
                full_matches,
                partial_matches,
                args,
            );
        }
    }

    process_tree(
        &tree,
        &pantry_ingredients,
        &mut full_matches,
        &mut partial_matches_raw,
        &args,
    );

    match format {
        OutputFormat::Human => {
            println!("Recipes You Can Make with Pantry Items:");
            println!("========================================");

            if !full_matches.is_empty() {
                println!("\n✓ Complete Matches (all ingredients available):");
                for recipe in &full_matches {
                    println!("  • {recipe}");
                }
            }

            if !partial_matches_raw.is_empty() {
                println!(
                    "\n⚠ Partial Matches ({}%+ ingredients available):",
                    args.threshold
                );
                for (recipe, percentage, missing) in &partial_matches_raw {
                    println!("  • {recipe} ({percentage}% available)");
                    println!("    Missing: {}", missing.join(", "));
                }
            }

            if full_matches.is_empty() && partial_matches_raw.is_empty() {
                if args.partial {
                    println!(
                        "\nNo recipes found with at least {}% of ingredients available.",
                        args.threshold
                    );
                } else {
                    println!("\nNo recipes found with all ingredients available.");
                    println!("Tip: Use --partial to see recipes you can mostly make.");
                }
            }
        }
        OutputFormat::Json => {
            let partial_matches: Vec<PartialMatch> = partial_matches_raw
                .into_iter()
                .map(|(recipe, percentage, missing)| PartialMatch {
                    recipe,
                    percentage,
                    missing_ingredients: missing,
                })
                .collect();

            let output = RecipesOutput {
                full_matches,
                partial_matches,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Yaml => {
            let partial_matches: Vec<PartialMatch> = partial_matches_raw
                .into_iter()
                .map(|(recipe, percentage, missing)| PartialMatch {
                    recipe,
                    percentage,
                    missing_ingredients: missing,
                })
                .collect();

            let output = RecipesOutput {
                full_matches,
                partial_matches,
            };
            println!("{}", serde_yaml::to_string(&output)?);
        }
    }

    Ok(())
}

fn run_plan(ctx: &AppContext, args: PlanArgs, format: OutputFormat) -> Result<()> {
    // Build recipe tree
    let tree = build_tree(ctx.base_path()).context("Failed to build recipe tree")?;

    // Structure to hold recipe information
    struct RecipeInfo {
        ingredients: HashSet<String>,
    }

    let mut recipes: Vec<RecipeInfo> = Vec::new();

    // Recursively process recipes in the tree
    fn process_tree(tree: &cooklang_find::RecipeTree, recipes: &mut Vec<RecipeInfo>) {
        // Check if this node has a recipe
        if let Some(entry) = &tree.recipe {
            // Skip .menu files - only process .cook files
            if entry.is_menu() {
                return;
            }

            // Parse the recipe
            if let Ok(recipe) = parse_recipe_from_entry(entry, 1.0) {
                let mut recipe_ingredients = HashSet::new();

                // Get all ingredients from the recipe (excluding recipe references)
                for ingredient in &recipe.ingredients {
                    // Skip recipe references
                    if ingredient.reference.is_some() {
                        continue;
                    }
                    if ingredient.modifiers().should_be_listed() {
                        let name = ingredient.display_name().to_string();
                        recipe_ingredients.insert(name);
                    }
                }

                if !recipe_ingredients.is_empty() {
                    recipes.push(RecipeInfo {
                        ingredients: recipe_ingredients,
                    });
                }
            }
        }

        // Recursively check children
        for subtree in tree.children.values() {
            process_tree(subtree, recipes);
        }
    }

    process_tree(&tree, &mut recipes);

    if recipes.is_empty() {
        match format {
            OutputFormat::Human => println!("No recipes found in collection."),
            OutputFormat::Json => {
                let output = PlanOutput {
                    total_recipes: 0,
                    cookable_recipes: 0,
                    coverage_percentage: 0,
                    ingredients: vec![],
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
            OutputFormat::Yaml => {
                let output = PlanOutput {
                    total_recipes: 0,
                    cookable_recipes: 0,
                    coverage_percentage: 0,
                    ingredients: vec![],
                };
                println!("{}", serde_yaml::to_string(&output)?);
            }
        }
        return Ok(());
    }

    // Greedy coverage algorithm
    // Track which ingredients are still needed for each recipe
    let mut recipe_missing: Vec<HashSet<String>> =
        recipes.iter().map(|r| r.ingredients.clone()).collect();

    let mut selected_ingredients: Vec<IngredientStep> = Vec::new();
    let mut cookable_count = 0;
    let total_recipes = recipes.len();

    // Continue until all recipes are cookable or we hit the limit
    let max_ingredients = args.max_ingredients.unwrap_or(usize::MAX);

    while cookable_count < total_recipes && selected_ingredients.len() < max_ingredients {
        // Count how many recipes each ingredient appears in (among remaining recipes)
        let mut ingredient_scores: HashMap<String, usize> = HashMap::new();

        for missing_set in &recipe_missing {
            for ingredient in missing_set {
                *ingredient_scores.entry(ingredient.clone()).or_insert(0) += 1;
            }
        }

        if ingredient_scores.is_empty() {
            break; // No more ingredients to select
        }

        // Select the ingredient that appears in the most recipes
        let best_ingredient = ingredient_scores
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(ing, _)| ing.clone())
            .unwrap();

        // Remove this ingredient from all recipe missing lists
        let mut new_recipe_missing = Vec::new();
        let mut newly_cookable = 0;

        for mut missing_set in recipe_missing {
            missing_set.remove(&best_ingredient);

            if missing_set.len() <= args.allow_missing {
                // This recipe is now cookable (with N or fewer missing ingredients)
                newly_cookable += 1;
            } else {
                // Still missing some ingredients
                new_recipe_missing.push(missing_set);
            }
        }

        recipe_missing = new_recipe_missing;
        cookable_count += newly_cookable;

        selected_ingredients.push(IngredientStep {
            name: best_ingredient,
            new_recipes_unlocked: newly_cookable,
            total_cookable: cookable_count,
        });
    }

    let coverage_percentage = (cookable_count * 100) / total_recipes.max(1);

    // Output results
    match format {
        OutputFormat::Human => {
            println!("Optimal Pantry Plan (Greedy Coverage):");
            println!("=======================================");
            if args.allow_missing > 0 {
                println!(
                    "Note: Allowing recipes with up to {} missing ingredient{}",
                    args.allow_missing,
                    if args.allow_missing == 1 { "" } else { "s" }
                );
            }
            println!();

            if args.skip > 0 && args.skip < selected_ingredients.len() {
                // Show summary of skipped ingredients
                let skipped_coverage = selected_ingredients[args.skip - 1].total_cookable;
                let skipped_pct = (skipped_coverage * 100) / total_recipes.max(1);

                println!("Already have (first {} ingredients):", args.skip);
                println!(
                    "  → Can cook {} out of {} recipes ({}% coverage)",
                    skipped_coverage, total_recipes, skipped_pct
                );
                println!();
                println!("Recommended additions:");
                println!();

                // Show remaining ingredients
                for (i, step) in selected_ingredients.iter().enumerate().skip(args.skip) {
                    let new_str = if step.new_recipes_unlocked == 1 {
                        "recipe"
                    } else {
                        "recipes"
                    };
                    println!(
                        "{:3}. {:<40} (+{} {}, {} total)",
                        i + 1,
                        step.name,
                        step.new_recipes_unlocked,
                        new_str,
                        step.total_cookable
                    );
                }
            } else {
                // Normal output without skipping
                println!(
                    "With these {} ingredients, you can cook {} out of {} recipes:",
                    selected_ingredients.len(),
                    cookable_count,
                    total_recipes
                );
                println!();

                for (i, step) in selected_ingredients.iter().enumerate() {
                    let new_str = if step.new_recipes_unlocked == 1 {
                        "recipe"
                    } else {
                        "recipes"
                    };
                    println!(
                        "{:3}. {:<40} (+{} {}, {} total)",
                        i + 1,
                        step.name,
                        step.new_recipes_unlocked,
                        new_str,
                        step.total_cookable
                    );
                }
            }

            println!();
            println!("Final coverage: {}% of recipes", coverage_percentage);
        }
        OutputFormat::Json => {
            let output = PlanOutput {
                total_recipes,
                cookable_recipes: cookable_count,
                coverage_percentage,
                ingredients: selected_ingredients,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Yaml => {
            let output = PlanOutput {
                total_recipes,
                cookable_recipes: cookable_count,
                coverage_percentage,
                ingredients: selected_ingredients,
            };
            println!("{}", serde_yaml::to_string(&output)?);
        }
    }

    Ok(())
}

/// Check if two quantity strings have matching units
fn units_match(quantity: &str, low_threshold: &str) -> bool {
    let parse_unit = |s: &str| -> Option<String> {
        regex::Regex::new(r"^(\d+(?:\.\d+)?)\s*%?\s*(.*)$")
            .ok()
            .and_then(|re| re.captures(s))
            .map(|captures| {
                captures
                    .get(2)
                    .map(|m| m.as_str().to_lowercase())
                    .unwrap_or_default()
            })
    };

    match (parse_unit(quantity), parse_unit(low_threshold)) {
        (Some(q_unit), Some(l_unit)) => q_unit == l_unit,
        _ => false,
    }
}

fn is_low_quantity(quantity: &str) -> bool {
    // Default rules for when no explicit threshold is set
    // Grams/ml: <= 100, Kg/L: < 0.5, Items: <= 1
    if let Some(captures) = regex::Regex::new(r"^(\d+(?:\.\d+)?)\s*%?\s*(.*)$")
        .ok()
        .and_then(|re| re.captures(quantity))
    {
        if let Ok(amount) = captures.get(1).unwrap().as_str().parse::<f64>() {
            let unit = captures.get(2).map(|m| m.as_str()).unwrap_or("");

            // Different thresholds for different units
            match unit.to_lowercase().as_str() {
                "g" | "ml" => amount <= 100.0,
                "kg" | "l" => amount < 0.5,
                "" | "item" | "items" => amount <= 1.0,
                _ => amount <= 1.0,
            }
        } else {
            false
        }
    } else {
        false
    }
}

fn parse_date(date_str: &str) -> Option<NaiveDate> {
    // Try multiple date formats
    let formats = [
        "%Y-%m-%d", "%d.%m.%Y", "%d/%m/%Y", "%m/%d/%Y", "%Y.%m.%d", "%d-%m-%Y",
    ];

    for format in &formats {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
            return Some(date);
        }
    }

    None
}
