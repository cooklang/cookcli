use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};
use cookcli_core::pantry as core;
use serde::Serialize;

use crate::{util::cli_error, Context as AppContext};

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

    /// List all items in the pantry
    ///
    /// Displays all pantry items organized by section, showing quantities,
    /// expiry dates, and low-stock thresholds.
    ///
    /// Examples:
    ///   cook pantry list                     # List all items in human format
    ///   cook pantry list -f json             # Output as JSON
    ///   cook pantry list --section dairy     # Show only the dairy section
    #[command(alias = "ls")]
    List(ListArgs),

    /// Add an item to the pantry
    ///
    /// Adds a new ingredient to the specified section of your pantry
    /// configuration. Creates the pantry file and section if they do not
    /// exist yet.
    ///
    /// Examples:
    ///   cook pantry add pantry flour                                # Simple item
    ///   cook pantry add dairy milk --quantity "2%l"                 # With quantity
    ///   cook pantry add dairy yogurt --quantity "500%g" --low "200%g" --expire 2025-06-01
    #[command(alias = "a")]
    Add(AddArgs),

    /// Remove an item from the pantry
    ///
    /// Removes an ingredient from the specified section. If the section
    /// becomes empty after removal it is also deleted from the file.
    ///
    /// Examples:
    ///   cook pantry remove dairy milk        # Remove milk from the dairy section
    #[command(alias = "rm")]
    Remove(RemoveArgs),

    /// Update an item already in the pantry
    ///
    /// Updates one or more attributes of an existing pantry item.
    /// Only the flags you provide are changed; everything else is kept.
    ///
    /// Examples:
    ///   cook pantry update dairy milk --quantity "1%l"
    ///   cook pantry update dairy milk --expire 2025-06-15 --low "500%ml"
    #[command(alias = "up")]
    Update(UpdateArgs),
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

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Only show items from this section
    #[arg(long)]
    pub section: Option<String>,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Section to add the item into (e.g. dairy, produce, pantry)
    pub section: String,

    /// Name of the ingredient to add
    pub name: String,

    /// Quantity on hand (e.g. "2%kg", "500%ml", "12")
    #[arg(long)]
    pub quantity: Option<String>,

    /// Date the item was bought (e.g. 2025-01-01)
    #[arg(long)]
    pub bought: Option<String>,

    /// Expiry date (e.g. 2025-06-01)
    #[arg(long)]
    pub expire: Option<String>,

    /// Quantity considered "low" (e.g. "200%g")
    #[arg(long)]
    pub low: Option<String>,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Section containing the item
    pub section: String,

    /// Name of the ingredient to remove
    pub name: String,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Section containing the item
    pub section: String,

    /// Name of the ingredient to update
    pub name: String,

    /// New quantity value (e.g. "2%kg")
    #[arg(long)]
    pub quantity: Option<String>,

    /// New bought date
    #[arg(long)]
    pub bought: Option<String>,

    /// New expiry date
    #[arg(long)]
    pub expire: Option<String>,

    /// New low-stock threshold
    #[arg(long)]
    pub low: Option<String>,
}

// Output structures for JSON/YAML formats
#[derive(Debug, Serialize)]
struct ListSection {
    name: String,
    items: Vec<ListItem>,
}

#[derive(Debug, Serialize)]
struct ListItem {
    name: String,
    quantity: Option<String>,
    bought: Option<String>,
    expire: Option<String>,
    low: Option<String>,
}

#[derive(Debug, Serialize)]
struct ListOutput {
    sections: Vec<ListSection>,
}

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
        new_ctx = AppContext::discover(absolute_base_path);
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
        PantryCommand::List(list_args) => run_list(ctx, list_args, format),
        PantryCommand::Add(add_args) => run_add(ctx, add_args),
        PantryCommand::Remove(remove_args) => run_remove(ctx, remove_args),
        PantryCommand::Update(update_args) => run_update(ctx, update_args),
    }
}

fn run_depleted(ctx: &AppContext, args: DepletedArgs, format: OutputFormat) -> Result<()> {
    let depleted_items: Vec<DepletedItem> =
        core::depleted(ctx, core::DepletedRequest { all: args.all })
            .map_err(cli_error)?
            .into_value()
            .into_iter()
            .map(|item| DepletedItem {
                is_low: item.is_low(),
                name: item.name,
                section: item.section,
                quantity: item.quantity,
                low_threshold: item.low,
            })
            .collect();

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
    let expiring_list: Vec<ExpiringItem> = core::expiring(
        ctx,
        core::ExpiringRequest {
            days: args.days,
            include_unknown: args.include_unknown,
        },
    )
    .map_err(cli_error)?
    .into_value()
    .into_iter()
    .map(|expiring| ExpiringItem {
        status: expiry_status(expiring.days_until_expiry),
        expire_date: expiring.expire_date,
        days_until_expiry: expiring.days_until_expiry,
        name: expiring.item.name,
        section: expiring.item.section,
    })
    .collect();

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
    let matches = core::recipes(
        ctx,
        core::RecipesRequest {
            threshold: args.threshold,
        },
    )
    .map_err(cli_error)?
    .into_value();

    let full_matches = matches.full;
    // Core reports partial matches whether or not they were asked for; showing
    // them is what `--partial` decides.
    let partial_matches_raw: Vec<(String, usize, Vec<String>)> = if args.partial {
        matches
            .partial
            .into_iter()
            .map(|m| (m.name, m.percentage, m.missing))
            .collect()
    } else {
        Vec::new()
    };

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
    let plan = core::plan(
        ctx,
        core::PlanRequest {
            max_ingredients: args.max_ingredients,
            allow_missing: args.allow_missing,
        },
    )
    .map_err(cli_error)?
    .into_value();

    let total_recipes = plan.total_recipes;
    let cookable_count = plan.cookable_recipes();
    let coverage_percentage = plan.coverage_percentage();
    let selected_ingredients: Vec<IngredientStep> = plan
        .steps
        .into_iter()
        .map(|step| IngredientStep {
            name: step.name,
            new_recipes_unlocked: step.new_recipes_unlocked,
            total_cookable: step.total_cookable,
        })
        .collect();

    if total_recipes == 0 {
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

// ---------------------------------------------------------------------------
// CRUD run functions
// ---------------------------------------------------------------------------

fn run_list(ctx: &AppContext, args: ListArgs, format: OutputFormat) -> Result<()> {
    let sections: Vec<ListSection> = core::list(
        ctx,
        core::ListRequest {
            section: args.section.clone(),
        },
    )
    .map_err(cli_error)?
    .into_value()
    .sections
    .into_iter()
    .map(|section| ListSection {
        name: section.name,
        items: section
            .items
            .into_iter()
            .map(|item| ListItem {
                name: item.name,
                quantity: item.quantity,
                bought: item.bought,
                expire: item.expire,
                low: item.low,
            })
            .collect(),
    })
    .collect();

    // Core reports what is there and leaves this to the caller: asking for a
    // section that is not in the file is an error here, and always has been.
    if let Some(ref filter) = args.section {
        if sections.is_empty() {
            anyhow::bail!("Section '{}' not found in pantry", filter);
        }
    }

    match format {
        OutputFormat::Human => {
            if sections.is_empty() {
                println!("Pantry is empty.");
                return Ok(());
            }
            println!("Pantry Items:");
            println!("=============");
            for section in &sections {
                println!("\n{}:", section.name.to_uppercase());
                for item in &section.items {
                    print!("  • {}", item.name);
                    if let Some(ref qty) = item.quantity {
                        print!(" - {qty}");
                    }
                    let mut extras = Vec::new();
                    if let Some(ref expire) = item.expire {
                        extras.push(format!("expires: {expire}"));
                    }
                    if let Some(ref low) = item.low {
                        extras.push(format!("low: {low}"));
                    }
                    if let Some(ref bought) = item.bought {
                        extras.push(format!("bought: {bought}"));
                    }
                    if !extras.is_empty() {
                        print!(" ({})", extras.join(", "));
                    }
                    println!();
                }
            }
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&ListOutput { sections })?
            );
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&ListOutput { sections })?);
        }
    }

    Ok(())
}

fn run_add(ctx: &AppContext, args: AddArgs) -> Result<()> {
    core::add(
        ctx,
        core::AddRequest {
            section: args.section.clone(),
            name: args.name.clone(),
            quantity: args.quantity,
            bought: args.bought,
            expire: args.expire,
            low: args.low,
        },
    )
    .map_err(cli_error)?;

    println!("Added '{}' to section '{}'.", args.name, args.section);
    Ok(())
}

fn run_remove(ctx: &AppContext, args: RemoveArgs) -> Result<()> {
    core::remove(
        ctx,
        core::RemoveRequest {
            section: args.section.clone(),
            name: args.name.clone(),
        },
    )
    .map_err(cli_error)?;

    println!("Removed '{}' from section '{}'.", args.name, args.section);
    Ok(())
}

fn run_update(ctx: &AppContext, args: UpdateArgs) -> Result<()> {
    // Core refuses an update that sets nothing too, in the wording a library
    // can use. Checked again here so that the message names the flags the user
    // actually typed, which is the only thing this adds.
    if args.quantity.is_none()
        && args.bought.is_none()
        && args.expire.is_none()
        && args.low.is_none()
    {
        anyhow::bail!("No attributes specified. Provide at least one of --quantity, --bought, --expire, --low.");
    }

    core::update(
        ctx,
        core::UpdateRequest {
            section: args.section.clone(),
            name: args.name.clone(),
            quantity: args.quantity,
            bought: args.bought,
            expire: args.expire,
            low: args.low,
        },
    )
    .map_err(cli_error)?;

    println!("Updated '{}' in section '{}'.", args.name, args.section);
    Ok(())
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

/// How an expiry is worded for the user.
///
/// The arithmetic is `cookcli-core`'s; only the wording is here, because it is
/// what `cook pantry expiring` prints. `None` is an item that has no readable
/// expiry date, included only by `--include-unknown`.
fn expiry_status(days_until: Option<i64>) -> String {
    match days_until {
        None => "No expiry date".to_string(),
        Some(days) if days < 0 => format!("EXPIRED {} days ago", -days),
        Some(0) => "EXPIRES TODAY".to_string(),
        Some(1) => "expires tomorrow".to_string(),
        Some(days) => format!("expires in {days} days"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every branch of the wording, in the order a day counts down through
    /// them.
    #[test]
    fn expiry_is_worded_by_how_far_off_it_is() {
        assert_eq!(expiry_status(Some(-2)), "EXPIRED 2 days ago");
        assert_eq!(expiry_status(Some(-1)), "EXPIRED 1 days ago");
        assert_eq!(expiry_status(Some(0)), "EXPIRES TODAY");
        assert_eq!(expiry_status(Some(1)), "expires tomorrow");
        assert_eq!(expiry_status(Some(2)), "expires in 2 days");
        assert_eq!(expiry_status(None), "No expiry date");
    }
}
