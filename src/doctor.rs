use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Args, Subcommand};
use cooklang_find::build_tree;
use std::collections::{BTreeSet, BTreeMap, HashSet};
use std::fs;

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
        Some(DoctorCommand::Validate(validate_args)) => run_validate(ctx, validate_args),
        None => {
            // Run all doctor checks
            println!("Running all doctor checks...\n");
            
            println!("=== Recipe Validation ===");
            run_validate(ctx, ValidateArgs { base_path: None, strict: false })?;
            
            println!("\n=== Aisle Check ===");
            run_aisle(ctx, AisleArgs { base_path: None })?;
            
            Ok(())
        }
    }
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
            eprintln!("Warning: Failed to read aisle file");
        } else if aisle_path.is_none() {
            eprintln!("Warning: No aisle configuration found");
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
    ) {
        // Check if this node has a recipe
        if let Some(entry) = &tree.recipe {
            *recipe_count += 1;

            // Parse the recipe
            let recipe = match parse_recipe_from_entry(entry, 1.0) {
                Ok(r) => r,
                Err(e) => {
                    let name = entry.name().as_deref().unwrap_or("unknown");
                    eprintln!("Warning: Failed to parse recipe '{name}': {e}");
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
            process_recipes(subtree, all_ingredients, recipe_count);
        }
    }

    process_recipes(&tree, &mut all_ingredients, &mut recipe_count);

    println!(
        "Scanned {} recipes, found {} unique ingredients",
        recipe_count,
        all_ingredients.len()
    );

    // Check which ingredients are missing from aisle
    let missing_ingredients = if let Some(aisle_conf) = aisle {
        let aisle_info = aisle_conf.ingredients_info();

        all_ingredients
            .into_iter()
            .filter(|ingredient| {
                // Check if ingredient is in aisle (case-insensitive)
                !aisle_info
                    .iter()
                    .any(|(aisle_name, _)| aisle_name.eq_ignore_ascii_case(ingredient))
            })
            .collect::<Vec<_>>()
    } else {
        // No aisle config, all ingredients are "missing"
        all_ingredients.into_iter().collect()
    };

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

        if aisle_path.is_some() {
            println!("\nConsider adding these ingredients to your aisle.conf file.");
        } else {
            println!("\nConsider creating an aisle.conf file in ./config/aisle.conf");
        }
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
    
    // Track all recipe names for reference validation
    let mut all_recipe_names = HashSet::new();
    let mut recipe_references = BTreeMap::new();
    
    // First pass: collect all recipe names
    fn collect_recipe_names(
        tree: &cooklang_find::RecipeTree,
        names: &mut HashSet<String>,
    ) {
        if let Some(entry) = &tree.recipe {
            if let Some(name) = entry.name() {
                names.insert(name.to_lowercase());
                // Also add without extension
                if let Some(stem) = name.strip_suffix(".cook") {
                    names.insert(stem.to_lowercase());
                }
            }
        }
        
        for subtree in tree.children.values() {
            collect_recipe_names(subtree, names);
        }
    }
    
    collect_recipe_names(&tree, &mut all_recipe_names);
    
    // Second pass: validate recipes and collect references
    fn validate_recipes(
        tree: &cooklang_find::RecipeTree,
        base_path: &Utf8PathBuf,
        stats: &mut (usize, usize, usize, usize, usize),
        recipe_refs: &mut BTreeMap<String, Vec<String>>,
    ) {
        if let Some(entry) = &tree.recipe {
            stats.0 += 1; // total_recipes
            
            let recipe_name = entry.name().as_deref().unwrap_or("unknown");
            let recipe_path = entry.path().cloned().unwrap_or_else(|| base_path.join(recipe_name));
            let relative_path = if let Ok(stripped) = recipe_path.strip_prefix(base_path) {
                stripped
            } else {
                &recipe_path
            };
            
            // Try to read and parse the recipe
            match fs::read_to_string(&recipe_path) {
                Ok(content) => {
                    // Parse with lenient parser to get all errors and warnings
                    let parsed = cooklang::parse(&content);
                    
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
            validate_recipes(subtree, base_path, stats, recipe_refs);
        }
    }
    
    let mut stats = (total_recipes, recipes_with_errors, recipes_with_warnings, total_errors, total_warnings);
    validate_recipes(&tree, base_path, &mut stats, &mut recipe_references);
    (total_recipes, recipes_with_errors, recipes_with_warnings, total_errors, total_warnings) = stats;
    
    // Check recipe references
    if !recipe_references.is_empty() {
        println!("\n=== Recipe References ===");
        let mut missing_refs = false;
        
        for (recipe_path, refs) in &recipe_references {
            let mut missing_in_recipe = Vec::new();
            
            for reference in refs {
                let ref_lower = reference.to_lowercase();
                let ref_without_ext = ref_lower.strip_suffix(".cook").unwrap_or(&ref_lower);
                
                if !all_recipe_names.contains(&ref_lower) && 
                   !all_recipe_names.contains(ref_without_ext) {
                    missing_in_recipe.push(reference.clone());
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
            anyhow::bail!("Recipe validation failed with {} errors and {} warnings", total_errors, total_warnings);
        }
    }
    
    Ok(())
}
