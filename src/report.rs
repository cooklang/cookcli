use crate::util::split_recipe_name_and_scaling_factor;
use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::{CommandFactory, Parser};
use cooklang_reports::{config::Config, render_template_with_config};
use std::{fs, path::PathBuf};

#[derive(Parser, Debug)]
pub struct ReportArgs {
    /// Path to the Jinja2 template file
    #[arg(short, long)]
    template: Utf8PathBuf,

    /// Path to the recipe file (can include scaling factor with :N suffix)
    #[arg()]
    recipe: String,

    /// Path to the datastore directory (optional)
    #[arg(short, long)]
    datastore: Option<Utf8PathBuf>,
}

pub fn run(_ctx: &crate::Context, args: ReportArgs) -> Result<()> {
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
        .with_context(|| format!("Failed to read recipe file: {}", recipe_name))?;

    // Read the template file
    let template = fs::read_to_string(&args.template)
        .with_context(|| format!("Failed to read template file: {}", args.template))?;

    // Create config with scale and optional datastore
    let mut builder = Config::builder();
    builder.scale(scaling_factor);

    if let Some(datastore) = args.datastore {
        builder.datastore_path(PathBuf::from(datastore));
    }

    let config = builder.build();

    // Render the report
    let report = render_template_with_config(&recipe, &template, &config)
        .with_context(|| "Failed to render report from template")?;

    // Print the report
    println!("{}", report);

    Ok(())
}
