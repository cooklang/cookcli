use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Args, Subcommand};
use cooklang_find::build_tree;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use tracing::warn;

use crate::{util::parse_recipe_from_entry, Context};

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
            run_validate(
                ctx,
                ValidateArgs {
                    base_path: None,
                    strict: false,
                },
            )?;

            println!("\n=== Aisle Check ===");
            run_aisle(ctx, AisleArgs { base_path: None })?;

            println!("\n=== Pantry Check ===");
            run_pantry(ctx, PantryArgs { base_path: None })?;

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

fn run_pantry(ctx: &Context, args: PantryArgs) -> Result<()> {
    let base_path = args.base_path.as_ref().unwrap_or(ctx.base_path());

    // Load pantry configuration
    let pantry_path = ctx.pantry();
    let pantry = pantry_path.as_ref().and_then(|path| {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let result = cooklang::pantry::parse_lenient(&content);

                // Display warnings if any
                if result.report().has_warnings() {
                    warn!("Warnings in pantry configuration:");
                    for warning in result.report().warnings() {
                        warn!("  - {warning}");
                    }
                }

                result.output().cloned().map(|mut p| {
                    p.rebuild_index();
                    p
                })
            }
            Err(e) => {
                warn!("Failed to read pantry file: {e}");
                None
            }
        }
    });

    if pantry.is_none() && pantry_path.is_none() {
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

    // Find all recipes
    let tree = build_tree(base_path)?;

    // Collect all unique ingredients from all recipes and track which are in pantry
    let mut all_ingredients = BTreeSet::new();
    let mut pantry_ingredients = BTreeSet::new();
    let mut recipe_count = 0;

    // Walk through the tree to find and process all recipes
    fn process_recipes(
        tree: &cooklang_find::RecipeTree,
        all_ingredients: &mut BTreeSet<String>,
        pantry_ingredients: &mut BTreeSet<String>,
        pantry: &Option<cooklang::pantry::PantryConf>,
        recipe_count: &mut usize,
        parser: &cooklang::CooklangParser,
    ) {
        // Check if this node has a recipe
        if let Some(entry) = &tree.recipe {
            *recipe_count += 1;

            // Parse the recipe
            let recipe = match parse_recipe_from_entry(entry, 1.0, parser) {
                Ok(r) => r,
                Err(e) => {
                    let name = entry.name().as_deref().unwrap_or("unknown");
                    warn!("Failed to parse recipe '{name}': {e}");
                    return;
                }
            };

            // Collect ingredients (excluding recipe references)
            for ingredient in recipe.ingredients.iter() {
                // Skip recipe references - they shouldn't be in pantry
                if ingredient.reference.is_some() {
                    continue;
                }

                if ingredient.modifiers().should_be_listed() {
                    let name = ingredient.display_name();
                    let name_str = name.to_string();
                    all_ingredients.insert(name_str.clone());

                    // Check if this ingredient is in pantry
                    if let Some(pantry_conf) = pantry {
                        if pantry_conf.has_ingredient(&name_str) {
                            pantry_ingredients.insert(name_str);
                        }
                    }
                }
            }
        }

        // Recursively check children
        for subtree in tree.children.values() {
            process_recipes(
                subtree,
                all_ingredients,
                pantry_ingredients,
                pantry,
                recipe_count,
                parser,
            );
        }
    }

    process_recipes(
        &tree,
        &mut all_ingredients,
        &mut pantry_ingredients,
        &pantry,
        &mut recipe_count,
        &ctx.parser,
    );

    println!(
        "Scanned {} recipes, found {} unique ingredients",
        recipe_count,
        all_ingredients.len()
    );

    if pantry.is_some() {
        if pantry_ingredients.is_empty() {
            println!("\n✓ No recipe ingredients are currently in your pantry");
        } else {
            println!(
                "\n{} ingredients from recipes are in your pantry:",
                pantry_ingredients.len()
            );
            for ingredient in pantry_ingredients {
                println!("  ✓ {ingredient}");
            }
            println!("\nThese ingredients will be excluded from shopping lists.");
        }
    }

    Ok(())
}

fn run_aisle(ctx: &Context, args: AisleArgs) -> Result<()> {
    let base_path = args.base_path.as_ref().unwrap_or(ctx.base_path());

    // Load aisle configuration
    let aisle_path = ctx.aisle();
    let aisle_data = aisle_path.as_ref().and_then(|path| {
        std::fs::read_to_string(path)
            .map(|content| (path.clone(), content))
            .ok()
    });

    let aisle = if let Some((_path, content)) = aisle_data.as_ref() {
        let result = cooklang::aisle::parse_lenient(content);

        // Display warnings if any
        if result.report().has_warnings() {
            eprintln!("Warnings in aisle configuration:");
            for warning in result.report().warnings() {
                eprintln!("  - {warning}");
            }
        }

        result.output().cloned()
    } else {
        if aisle_data.is_none() && aisle_path.is_some() {
            warn!("Failed to read aisle file");
        } else if aisle_path.is_none() {
            warn!("No aisle configuration found");
        }
        None
    };

    // Find all recipes
    let tree = build_tree(base_path)?;

    // Collect all unique ingredients from all recipes
    let mut all_ingredients = BTreeSet::new();
    let mut recipe_count = 0;

    // Walk through the tree to find and process all recipes
    fn process_recipes(
        tree: &cooklang_find::RecipeTree,
        all_ingredients: &mut BTreeSet<String>,
        recipe_count: &mut usize,
        parser: &cooklang::CooklangParser,
    ) {
        // Check if this node has a recipe
        if let Some(entry) = &tree.recipe {
            *recipe_count += 1;

            // Parse the recipe
            let recipe = match parse_recipe_from_entry(entry, 1.0, parser) {
                Ok(r) => r,
                Err(e) => {
                    let name = entry.name().as_deref().unwrap_or("unknown");
                    warn!("Failed to parse recipe '{name}': {e}");
                    return;
                }
            };

            // Collect ingredients (excluding recipe references)
            for ingredient in recipe.ingredients.iter() {
                // Skip recipe references - they shouldn't be in aisle
                if ingredient.reference.is_some() {
                    continue;
                }

                if ingredient.modifiers().should_be_listed() {
                    let name = ingredient.display_name();
                    all_ingredients.insert(name.to_string());
                }
            }
        }

        // Recursively check children
        for subtree in tree.children.values() {
            process_recipes(subtree, all_ingredients, recipe_count, parser);
        }
    }

    process_recipes(&tree, &mut all_ingredients, &mut recipe_count, &ctx.parser);

    println!(
        "Scanned {} recipes, found {} unique ingredients",
        recipe_count,
        all_ingredients.len()
    );

    // Check which ingredients are missing from aisle
    if let Some(aisle_conf) = aisle {
        let aisle_info = aisle_conf.ingredients_info();

        let missing_ingredients: Vec<_> = all_ingredients
            .into_iter()
            .filter(|ingredient| {
                // Check if ingredient is in aisle (case-insensitive)
                !aisle_info
                    .iter()
                    .any(|(aisle_name, _)| aisle_name.eq_ignore_ascii_case(ingredient))
            })
            .collect();

        // Output results
        if missing_ingredients.is_empty() {
            println!("✓ All ingredients are present in aisle configuration");
        } else {
            println!(
                "\n{} ingredients not found in aisle configuration:",
                missing_ingredients.len()
            );
            for ingredient in missing_ingredients {
                println!("  - {ingredient}");
            }
            println!("\nConsider adding these ingredients to your aisle.conf file.");
        }
    } else {
        // No aisle config found - just inform the user
        println!("\nNo aisle configuration found.");
        println!("To organize ingredients by store section, create an aisle.conf file in:");
        println!("  - ./config/aisle.conf (project-specific)");
        println!("  - ~/.config/cook/aisle.conf (global)");
    }

    Ok(())
}

fn run_validate(ctx: &Context, args: ValidateArgs) -> Result<()> {
    let base_path = args.base_path.as_ref().unwrap_or(ctx.base_path());

    // Build the recipe tree
    let tree = build_tree(base_path)?;

    // Statistics
    let mut total_recipes = 0;
    let mut recipes_with_errors = 0;
    let mut recipes_with_warnings = 0;
    let mut total_errors = 0;
    let mut total_warnings = 0;

    // Track recipe references for validation
    let mut recipe_references = BTreeMap::new();

    // Validate recipes and collect references
    fn validate_recipes(
        tree: &cooklang_find::RecipeTree,
        base_path: &Utf8PathBuf,
        stats: &mut (usize, usize, usize, usize, usize),
        recipe_refs: &mut BTreeMap<String, Vec<String>>,
        parser: &cooklang::CooklangParser,
    ) {
        if let Some(entry) = &tree.recipe {
            stats.0 += 1; // total_recipes

            let recipe_name = entry.name().as_deref().unwrap_or("unknown");
            let recipe_path = entry
                .path()
                .cloned()
                .unwrap_or_else(|| base_path.join(recipe_name));
            let relative_path = if let Ok(stripped) = recipe_path.strip_prefix(base_path) {
                stripped
            } else {
                &recipe_path
            };

            // Try to read and parse the recipe
            match fs::read_to_string(&recipe_path) {
                Ok(content) => {
                    // Parse with our configured parser to get all errors and warnings
                    let parsed = parser.parse(&content);

                    let errors: Vec<_> = parsed.report().errors().collect();
                    let warnings: Vec<_> = parsed.report().warnings().collect();

                    let has_errors = !errors.is_empty();
                    let has_warnings = !warnings.is_empty();

                    if has_errors || has_warnings {
                        println!("\n📄 {relative_path}");

                        if has_errors {
                            stats.1 += 1; // recipes_with_errors
                            stats.3 += errors.len(); // total_errors

                            for error in errors {
                                println!("  ❌ Error: {error}");
                            }
                        }

                        if has_warnings {
                            stats.2 += 1; // recipes_with_warnings
                            stats.4 += warnings.len(); // total_warnings

                            for warning in warnings {
                                println!("  ⚠️  Warning: {warning}");
                            }
                        }
                    }

                    // Collect recipe references
                    if let Some(recipe) = parsed.output() {
                        let mut refs = Vec::new();
                        for ingredient in &recipe.ingredients {
                            if let Some(reference) = &ingredient.reference {
                                // Get the full path of the reference
                                let ref_path = if reference.components.is_empty() {
                                    reference.name.clone()
                                } else {
                                    reference.path("/")
                                };
                                refs.push(ref_path);
                            }
                        }
                        if !refs.is_empty() {
                            recipe_refs.insert(relative_path.to_string(), refs);
                        }
                    }
                }
                Err(e) => {
                    println!("\n📄 {relative_path}");
                    println!("  ❌ Error: Failed to read file: {e}");
                    stats.1 += 1; // recipes_with_errors
                    stats.3 += 1; // total_errors
                }
            }
        }

        // Recursively check children
        for subtree in tree.children.values() {
            validate_recipes(subtree, base_path, stats, recipe_refs, parser);
        }
    }

    let mut stats = (
        total_recipes,
        recipes_with_errors,
        recipes_with_warnings,
        total_errors,
        total_warnings,
    );
    validate_recipes(
        &tree,
        base_path,
        &mut stats,
        &mut recipe_references,
        &ctx.parser,
    );
    (
        total_recipes,
        recipes_with_errors,
        recipes_with_warnings,
        total_errors,
        total_warnings,
    ) = stats;

    // Check recipe references using cooklang_find::get_recipe
    if !recipe_references.is_empty() {
        println!("\n=== Recipe References ===");
        let mut missing_refs = false;

        for (recipe_path, refs) in &recipe_references {
            let mut missing_in_recipe = Vec::new();

            for reference in refs {
                // Try to resolve the recipe using cooklang_find::get_recipe
                // This handles relative paths and recipe discovery properly
                match cooklang_find::get_recipe(vec![base_path.clone()], reference.into()) {
                    Ok(_) => {
                        // Recipe found, reference is valid
                    }
                    Err(_) => {
                        // Recipe not found
                        missing_in_recipe.push(reference.clone());
                    }
                }
            }

            if !missing_in_recipe.is_empty() {
                missing_refs = true;
                println!("\n📄 {recipe_path}");
                for missing_ref in missing_in_recipe {
                    println!("  ❌ Missing reference: {missing_ref}");
                    total_errors += 1;
                }
            }
        }

        if !missing_refs {
            println!("✓ All recipe references are valid");
        }
    }

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
