use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, Subcommand};
use cookcli_core::{
    doctor::{aisle_coverage, broken_references, pantry_coverage, CoverageRequest},
    Diagnostic, Severity,
};
use std::collections::BTreeSet;
use tracing::warn;

use crate::{util::cli_error, Context};

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[command(subcommand)]
    command: Option<DoctorCommand>,
}

#[derive(Debug, Subcommand)]
enum DoctorCommand {
    /// Check for ingredients missing from aisle configuration
    ///
    /// Scans all recipes in your collection and identifies ingredients
    /// that are not categorized in your aisle.conf file. This helps
    /// maintain a complete shopping list categorization.
    ///
    /// The aisle.conf file groups ingredients by store section (produce,
    /// dairy, etc.) for better organized shopping lists.
    ///
    /// Example:
    ///   cook doctor aisle              # Check current directory
    ///   cook doctor aisle -b ~/recipes # Check specific directory
    Aisle(AisleArgs),

    /// Check which recipe ingredients are in your pantry
    ///
    /// Scans all recipes and shows which ingredients are already in your
    /// pantry inventory. This helps identify what you don't need to buy.
    ///
    /// The pantry.conf file (TOML format) tracks your ingredient inventory
    /// with quantities and can be used to exclude items from shopping lists.
    ///
    /// Example:
    ///   cook doctor pantry             # Check current directory
    ///   cook doctor pantry -b ~/recipes # Check specific directory
    Pantry(PantryArgs),

    /// Validate all recipes for syntax errors and warnings
    ///
    /// Scans all recipes in your collection and reports:
    /// - Syntax errors that prevent parsing
    /// - Warnings about potential issues
    /// - Missing recipe references (when one recipe includes another)
    /// - Invalid units or quantities
    ///
    /// Example:
    ///   cook doctor validate           # Validate current directory
    ///   cook doctor validate -b ~/recipes # Validate specific directory
    ///   cook doctor validate --strict  # Exit with error code if issues found
    Validate(ValidateArgs),
}

#[derive(Debug, Args)]
struct AisleArgs {
    /// Directory to scan for recipe files
    ///
    /// The command will recursively search this directory for .cook files
    /// and check all ingredients against the aisle configuration.
    /// Defaults to the current directory.
    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    base_path: Option<Utf8PathBuf>,
}

#[derive(Debug, Args)]
struct PantryArgs {
    /// Directory to scan for recipe files
    ///
    /// The command will recursively search this directory for .cook files
    /// and check all ingredients against the pantry configuration.
    /// Defaults to the current directory.
    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    base_path: Option<Utf8PathBuf>,
}

#[derive(Debug, Args)]
struct ValidateArgs {
    /// Directory to scan for recipe files
    ///
    /// The command will recursively search this directory for .cook files
    /// and validate their syntax and references.
    /// Defaults to the current directory.
    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    base_path: Option<Utf8PathBuf>,

    /// Exit with error code if any issues are found
    ///
    /// By default, the command reports issues but exits successfully.
    /// Use this flag in CI/CD pipelines to fail on validation errors.
    #[arg(long)]
    strict: bool,
}

pub fn run(ctx: &Context, args: DoctorArgs) -> Result<()> {
    match args.command {
        Some(DoctorCommand::Aisle(aisle_args)) => run_aisle(ctx, aisle_args),
        Some(DoctorCommand::Pantry(pantry_args)) => run_pantry(ctx, pantry_args),
        Some(DoctorCommand::Validate(validate_args)) => run_validate(ctx, validate_args),
        None => {
            // Run all doctor checks
            println!("Running all doctor checks...\n");

            // Check for updates
            #[cfg(feature = "self-update")]
            {
                println!("=== Version Check ===");
                check_for_updates();
            }
            #[cfg(not(feature = "self-update"))]
            {
                println!("=== Version Check ===");
                println!("ℹ️  Self-update is disabled in this build.");
                println!("   Please update through your package manager or build from source.");
            }

            println!("\n=== Recipe Validation ===");
            report_check(run_validate(
                ctx,
                ValidateArgs {
                    base_path: None,
                    strict: false,
                },
            ));

            println!("\n=== Aisle Check ===");
            report_check(run_aisle(ctx, AisleArgs { base_path: None }));

            println!("\n=== Pantry Check ===");
            report_check(run_pantry(ctx, PantryArgs { base_path: None }));

            Ok(())
        }
    }
}

#[cfg(feature = "self-update")]
fn check_for_updates() {
    match crate::update::check_for_updates() {
        Ok(Some(new_version)) => {
            println!("🆕 A new version ({new_version}) is available!");
            println!("   Run 'cook update' to install the latest version.");
        }
        Ok(None) => {
            println!("✅ You are running the latest version.");
        }
        Err(e) => {
            println!("⚠️  Unable to check for updates: {e}");
        }
    }
}

/// Print a check's failure and carry on to the next one.
///
/// Only the aggregate `cook doctor` goes through here. Each subcommand run on
/// its own still exits non-zero on a failure, because a script asking for one
/// answer wants to know it did not get it — but `cook doctor` is asked for
/// several, and a command whose whole job is reporting problems must not stop
/// at the first one and leave the remaining checks unrun. An unreadable
/// `aisle.conf` should not be able to hide what the pantry check would have
/// said.
fn report_check(result: Result<()>) {
    if let Err(e) = result {
        // `{e:#}` so the chain — core's line and the underlying cause — comes
        // out on the one line, matching the version check just above.
        println!("⚠️  This check could not run: {e:#}");
    }
}

/// Core returns its warnings instead of logging them, so that a library
/// consumer can show them its own way. Logging them is this boundary's job.
fn log_diagnostics(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        match diagnostic.location.as_ref().and_then(|l| l.file.as_ref()) {
            Some(file) => warn!("{file}: {}", diagnostic.message),
            None => warn!("{}", diagnostic.message),
        }
    }
}

fn run_pantry(ctx: &Context, args: PantryArgs) -> Result<()> {
    if ctx.pantry().is_unset() {
        println!("No pantry configuration found.");
        println!("To track your ingredient inventory, create a pantry.conf file in:");
        println!("  - ./config/pantry.conf (project-specific)");
        println!("  - ~/.config/cook/pantry.conf (global)");
        println!("\nExample pantry.conf (TOML format):");
        println!("[pantry]");
        println!("rice = \"5%kg\"");
        println!("pasta = \"1%kg\"");
        println!("\n[freezer]");
        println!("ice_cream = \"1%L\"");
        return Ok(());
    }

    let outcome = pantry_coverage(
        ctx,
        CoverageRequest {
            base_dir: args.base_path,
        },
    )
    .map_err(cli_error)?;
    log_diagnostics(&outcome.diagnostics);
    let coverage = outcome.value;

    println!(
        "Scanned {} recipes, found {} unique ingredients",
        coverage.total_recipes,
        coverage.total_ingredients()
    );

    let in_pantry: Vec<&str> = coverage.known().collect();
    if in_pantry.is_empty() {
        println!("\n✓ No recipe ingredients are currently in your pantry");
    } else {
        println!(
            "\n{} ingredients from recipes are in your pantry:",
            in_pantry.len()
        );
        for ingredient in in_pantry {
            println!("  ✓ {ingredient}");
        }
        println!("\nThese ingredients will be excluded from shopping lists.");
    }

    Ok(())
}

fn run_aisle(ctx: &Context, args: AisleArgs) -> Result<()> {
    // Unlike the pantry check, this one scans first and reports a missing
    // configuration afterwards, so the count is printed either way.
    let configured = !ctx.aisle().is_unset();
    if !configured {
        warn!("No aisle configuration found");
    }

    let outcome = aisle_coverage(
        ctx,
        CoverageRequest {
            base_dir: args.base_path,
        },
    )
    .map_err(cli_error)?;
    log_diagnostics(&outcome.diagnostics);
    let coverage = outcome.value;

    println!(
        "Scanned {} recipes, found {} unique ingredients",
        coverage.total_recipes,
        coverage.total_ingredients()
    );

    if !configured {
        println!("\nNo aisle configuration found.");
        println!("To organize ingredients by store section, create an aisle.conf file in:");
        println!("  - ./config/aisle.conf (project-specific)");
        println!("  - ~/.config/cook/aisle.conf (global)");
        return Ok(());
    }

    let missing: Vec<&str> = coverage.unknown().collect();
    if missing.is_empty() {
        println!("✓ All ingredients are present in aisle configuration");
    } else {
        println!(
            "\n{} ingredients not found in aisle configuration:",
            missing.len()
        );
        for ingredient in missing {
            println!("  - {ingredient}");
        }
        println!("\nConsider adding these ingredients to your aisle.conf file.");
    }

    Ok(())
}

fn run_validate(ctx: &Context, args: ValidateArgs) -> Result<()> {
    let report = cookcli_core::doctor::validate(
        ctx,
        cookcli_core::doctor::ValidateRequest {
            base_dir: args.base_path,
            // This report is going straight to a terminal.
            style: cookcli_core::Style::Ansi,
        },
    )
    .map_err(cli_error)?
    .into_value();

    for recipe in &report.recipes {
        if recipe.diagnostics.is_empty() {
            continue;
        }
        println!("\n📄 {}", recipe.path);

        if recipe.rendered.is_empty() {
            // A recipe core could not read has no source to quote, so there is
            // nothing rendered for it. Print what it did say instead.
            for diagnostic in &recipe.diagnostics {
                println!("  ❌ Error: {}", diagnostic.message);
            }
        } else {
            print!("{}", recipe.rendered);
        }
    }

    let total_recipes = report.total_recipes();
    let recipes_with_warnings = report.recipes_with_warnings();
    let mut total_errors = report.total_errors();
    let total_warnings = report.total_warnings();

    // Recipe references are resolved separately from validation, so a broken
    // one reaches neither the report's error count nor its recipe count. Both
    // have to be topped up here, and the recipe count as a *set* — a recipe
    // that both fails to parse and references something missing is still one
    // recipe, and one that carries two broken references is too.
    let mut failing_recipes: BTreeSet<&Utf8Path> = report
        .recipes
        .iter()
        .filter(|r| r.diagnostics.iter().any(|d| d.severity == Severity::Error))
        .map(|r| r.path.as_path())
        .collect();

    if !report.references().is_empty() {
        println!("\n=== Recipe References ===");
        let broken = broken_references(&report);

        if broken.is_empty() {
            println!("✓ All recipe references are valid");
        } else {
            for (recipe_path, missing) in broken {
                println!("\n📄 {recipe_path}");
                failing_recipes.insert(recipe_path);
                for missing_ref in missing {
                    println!("  ❌ Missing reference: {missing_ref}");
                    total_errors += 1;
                }
            }
        }
    }

    let recipes_with_errors = failing_recipes.len();

    // Print summary
    println!("\n=== Validation Summary ===");
    println!("Total recipes scanned: {total_recipes}");

    if total_errors == 0 && total_warnings == 0 {
        println!("✅ All recipes are valid!");
    } else {
        if total_errors > 0 {
            println!("❌ {total_errors} error(s) found in {recipes_with_errors} recipe(s)");
        }
        if total_warnings > 0 {
            println!("⚠️  {total_warnings} warning(s) found in {recipes_with_warnings} recipe(s)");
        }

        if args.strict {
            anyhow::bail!(
                "Recipe validation failed with {} errors and {} warnings",
                total_errors,
                total_warnings
            );
        }
    }

    Ok(())
}
