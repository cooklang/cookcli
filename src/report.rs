use crate::util::split_recipe_name_and_scaling_factor;
use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::{CommandFactory, Parser};
use cooklang_reports::{config::Config, render_template_with_config};
use std::{fs, path::PathBuf};
use tracing::warn;

#[derive(Parser, Debug)]
pub struct ReportArgs {
    /// Path to the Jinja2 template file
    ///
    /// The template receives recipe data including:
    /// - metadata (title, author, tags, etc.)
    /// - ingredients (with quantities and units)
    /// - steps (parsed instructions)
    /// - cookware and timers
    ///
    /// Example template variables:
    ///   {{ recipe.title }}
    ///   {% for ingredient in recipe.ingredients %}
    ///     {{ ingredient.name }}: {{ ingredient.quantity }}
    ///   {% endfor %}
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    template: Utf8PathBuf,

    /// Recipe file to process
    ///
    /// Can include an optional scaling factor using the :N syntax
    /// (e.g., "recipe.cook:2" to double the recipe). The scaling
    /// will be applied to all ingredient quantities in the template.
    #[arg(value_name = "RECIPE")]
    recipe: String,

    /// Path to the datastore directory with additional recipe data
    ///
    /// The datastore can contain nutritional information, costs,
    /// and other data that can be accessed in the template.
    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    datastore: Option<Utf8PathBuf>,

    /// Path to the aisle configuration file
    ///
    /// Used for categorizing ingredients by store section.
    /// The template can access ingredient aisle information.
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    aisle: Option<Utf8PathBuf>,

    /// Path to the pantry configuration file
    ///
    /// Used for filtering out pantry items from shopping lists.
    /// Ingredients marked as pantry items can be accessed in templates.
    #[arg(short = 'p', long, value_hint = clap::ValueHint::FilePath)]
    pantry: Option<Utf8PathBuf>,
}

pub fn run(ctx: &crate::Context, args: ReportArgs) -> Result<()> {
    // Print warning about prototype feature
    warn!("⚠️  The report command is a prototype feature and will change in future versions.");

    // Split recipe name and scaling factor
    let (recipe_name, scaling_factor) = split_recipe_name_and_scaling_factor(&args.recipe)
        .map(|(name, factor)| {
            let scale = factor.parse::<f64>().unwrap_or_else(|err| {
                let mut cmd = crate::CliArgs::command();
                cmd.error(
                    clap::error::ErrorKind::InvalidValue,
                    format!("Invalid scaling factor for '{name}': {err}"),
                )
                .exit()
            });
            (name, scale)
        })
        .unwrap_or((&args.recipe, 1.0));

    // Read the recipe file
    let recipe = fs::read_to_string(recipe_name)
        .with_context(|| format!("Failed to read recipe file: {recipe_name}"))?;

    // Read the template file
    let template = fs::read_to_string(&args.template)
        .with_context(|| format!("Failed to read template file: {}", args.template))?;

    // Create config with scale, optional datastore, and base_path from context
    let mut builder = Config::builder();
    builder.scale(scaling_factor);

    if let Some(datastore) = args.datastore {
        builder.datastore_path(PathBuf::from(datastore));
    }

    // Use the base_path from context
    builder.base_path(PathBuf::from(ctx.base_path()));

    // Add aisle configuration if provided
    if let Some(aisle_path) = args.aisle {
        builder.aisle_path(PathBuf::from(aisle_path));
    } else if let Some(aisle_path) = ctx.aisle() {
        // Use context aisle if not explicitly provided
        builder.aisle_path(PathBuf::from(aisle_path));
    }

    // Add pantry configuration if provided
    if let Some(pantry_path) = args.pantry {
        builder.pantry_path(PathBuf::from(pantry_path));
    } else if let Some(pantry_path) = ctx.pantry() {
        // Use context pantry if not explicitly provided
        builder.pantry_path(PathBuf::from(pantry_path));
    }

    let config = builder.build();

    // Render the report
    let report = match render_template_with_config(&recipe, &template, &config) {
        Ok(report) => report,
        Err(err) => {
            // Use the enhanced error formatting from cooklang-reports
            eprintln!("{}", err.format_with_source());
            std::process::exit(1);
        }
    };

    // Print the report
    println!("{report}");

    Ok(())
}
