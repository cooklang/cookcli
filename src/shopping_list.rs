// This file includes a substantial portion of code from
// https://github.com/Zheoni/cooklang-chef
//
// The original code is licensed under the MIT License, a copy of which
// is provided below in addition to our project's license.
//
//

// MIT License

// Copyright (c) 2023 Francisco J. Sanchez

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use anyhow::{Context as _, Result};
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};
use std::collections::BTreeMap;
use tracing::warn;
use yansi::Paint;

use cooklang::{
    aisle::AisleConf,
    ingredient_list::IngredientList,
    quantity::{GroupedQuantity, Quantity, Value},
};
use serde::Serialize;

use crate::{
    util::{extract_ingredients, write_to_output, PARSER},
    Context,
};

#[derive(Debug, Args)]
#[command()]
pub struct ShoppingListArgs {
    /// Recipe files to include in the shopping list
    ///
    /// Specify one or more recipe files by name or path. Each recipe can include
    /// an optional scaling factor using the :N syntax (e.g., "recipe.cook:2" to double).
    /// Glob patterns are supported (e.g., "*.cook" for all recipes in a directory).
    ///
    /// Examples:
    ///   pasta.cook              # Single recipe at default scale
    ///   "Pasta.cook:3"          # Triple the pasta recipe
    ///   recipe1.cook recipe2.cook  # Multiple recipes
    ///   desserts/*.cook         # All recipes in desserts folder
    recipes: Vec<String>,

    /// Base directory to search for recipe files
    ///
    /// When recipe names (not full paths) are provided, the tool will search
    /// for them in this directory. Defaults to the current directory.
    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    base_path: Option<Utf8PathBuf>,

    /// Output file path (stdout if not specified)
    ///
    /// The output format can be inferred from the file extension
    /// (.json, .yaml, .txt, .md)
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    output: Option<Utf8PathBuf>,

    /// Display ingredients without aisle categories
    ///
    /// By default, ingredients are grouped by their aisle category
    /// (produce, dairy, etc.). This flag displays them as a simple list.
    #[arg(short, long)]
    plain: bool,

    /// Output format for the shopping list
    ///
    /// Available formats: human (default), json, yaml, markdown
    /// If not specified, format is inferred from output file extension.
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// Pretty output format, if available
    #[arg(long)]
    pretty: bool,

    /// Load aisle conf file
    #[arg(short, long)]
    aisle: Option<Utf8PathBuf>,

    /// Don't expand referenced recipes
    #[arg(short, long)]
    ignore_references: bool,

    /// Display only ingredient names, one per line, without amounts
    #[arg(long)]
    ingredients_only: bool,
}

impl ShoppingListArgs {
    pub fn get_base_path(&self) -> Option<Utf8PathBuf> {
        self.base_path.clone()
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
    Yaml,
    Markdown,
}

pub fn run(ctx: &Context, args: ShoppingListArgs) -> Result<()> {
    // Expand directories to .cook files
    let mut expanded_recipes = Vec::new();
    for entry in &args.recipes {
        let path = if entry.contains(':') {
            // Handle recipe:scaling syntax
            let (recipe_path, _) = entry.split_once(':').unwrap();
            Utf8PathBuf::from(recipe_path)
        } else {
            Utf8PathBuf::from(entry)
        };

        // Check if it's a directory
        if path.is_dir() {
            // Find all .cook files in the directory
            for dir_entry in std::fs::read_dir(&path)? {
                let dir_entry = dir_entry?;
                let file_path = dir_entry.path();
                if let Some(ext) = file_path.extension() {
                    if ext == "cook" {
                        if let Ok(utf8_path) = Utf8PathBuf::from_path_buf(file_path) {
                            // Preserve the scaling factor if it was specified
                            if entry.contains(':') {
                                let scaling = entry.split_once(':').unwrap().1;
                                expanded_recipes.push(format!("{utf8_path}:{scaling}"));
                            } else {
                                expanded_recipes.push(utf8_path.to_string());
                            }
                        }
                    }
                }
            }
        } else {
            // Not a directory, use as-is
            expanded_recipes.push(entry.clone());
        }
    }

    // If no recipes were expanded (empty directory or no directories), use original list
    if expanded_recipes.is_empty() && !args.recipes.is_empty() {
        expanded_recipes = args.recipes.clone();
    }

    let aile_path = args
        .aisle
        .or_else(|| ctx.aisle())
        .map(|path| -> Result<(_, _)> {
            let content = std::fs::read_to_string(&path).context("Failed to read aisle file")?;
            Ok((path, content))
        })
        .transpose()?;

    let aisle = if let Some((_path, content)) = &aile_path {
        // Use parse_lenient to be more forgiving with aisle configuration
        let result = cooklang::aisle::parse_lenient(content);

        // Check if there are any warnings to display
        if result.report().has_warnings() {
            for warning in result.report().warnings() {
                warn!("Aisle configuration warning: {}", warning);
            }
        }

        // Get the output - parse_lenient should always return something
        result.output().cloned().unwrap_or_else(|| {
            warn!("Aisle file parsing failed, using default configuration");
            Default::default()
        })
    } else {
        warn!("No aisle file found. Docs https://cooklang.org/docs/spec/#shopping-lists");
        Default::default()
    };

    // Load pantry configuration if available
    let pantry_path = ctx.pantry();
    let pantry = if let Some(path) = &pantry_path {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                tracing::debug!("Loading pantry from: {}", path);
                let result = cooklang::pantry::parse_lenient(&content);

                // Check if there are any warnings to display
                if result.report().has_warnings() {
                    for warning in result.report().warnings() {
                        warn!("Pantry configuration warning: {}", warning);
                    }
                }

                let mut pantry_conf = result.output().cloned();
                if let Some(ref mut pantry) = pantry_conf {
                    pantry.rebuild_index();
                    tracing::debug!(
                        "Pantry loaded successfully with {} sections",
                        pantry.sections.len()
                    );
                } else {
                    tracing::warn!("Failed to parse pantry file");
                }
                pantry_conf
            }
            Err(e) => {
                warn!("Failed to read pantry file: {}", e);
                None
            }
        }
    } else {
        tracing::debug!("No pantry file found");
        None
    };

    let format = args.format.unwrap_or_else(|| match &args.output {
        Some(p) => match p.extension() {
            Some("json") => OutputFormat::Json,
            _ => OutputFormat::Human,
        },
        None => OutputFormat::Human,
    });

    // retrieve, scale and merge ingredients
    let mut list = IngredientList::new();
    let mut seen = BTreeMap::new();

    let ignore_references = args.ignore_references;

    for entry in expanded_recipes {
        // Determine the base path for this entry
        // If the entry is an absolute path or relative path to a file,
        // use its parent directory as the base for resolving references
        let entry_without_scaling = entry.split(':').next().unwrap_or(&entry);
        let entry_path = Utf8PathBuf::from(entry_without_scaling);

        // Check if this is a file path (contains '/' or starts with './')
        let (actual_entry, base_path) = if entry_without_scaling.contains('/') {
            // This looks like a file path, not just a recipe name
            let full_path = if entry_path.is_absolute() {
                entry_path.clone()
            } else {
                // Clean up the path by removing ./ prefix if present
                let clean_entry = entry_without_scaling
                    .strip_prefix("./")
                    .unwrap_or(entry_without_scaling);
                ctx.base_path().join(clean_entry)
            };

            if full_path.exists() && full_path.is_file() {
                // File exists, use its parent directory as base
                let base = full_path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| ctx.base_path().clone());

                // Convert to just the filename for the recipe lookup
                let filename = full_path
                    .file_name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| entry_without_scaling.to_string());

                // Preserve scaling if present
                let actual = if entry.contains(':') {
                    format!("{}:{}", filename, entry.split(':').nth(1).unwrap())
                } else {
                    filename
                };

                (actual, base)
            } else {
                // File doesn't exist, but still treat as path
                // This will fail with a better error message
                (entry.to_string(), ctx.base_path().clone())
            }
        } else {
            // This is just a recipe name, use as-is
            (entry.to_string(), ctx.base_path().clone())
        };

        extract_ingredients(
            &actual_entry,
            &mut list,
            &mut seen,
            &base_path,
            PARSER.converter(),
            ignore_references,
        )?;
    }

    // Use common names from aisle configuration
    list = list.use_common_names(&aisle, PARSER.converter());

    // Subtract pantry quantities from shopping list
    if let Some(pantry_conf) = &pantry {
        list = list.subtract_pantry(pantry_conf, PARSER.converter());
    }

    write_to_output(args.output.as_deref(), |w| {
        if args.ingredients_only {
            match format {
                OutputFormat::Human => {
                    // Simple output: one ingredient per line, no amounts
                    for (ingredient, _quantity) in list {
                        writeln!(w, "{ingredient}")?;
                    }
                }
                OutputFormat::Json => {
                    // Output as a JSON array of strings
                    let ingredients: Vec<String> =
                        list.into_iter().map(|(ingredient, _)| ingredient).collect();
                    if args.pretty {
                        serde_json::to_writer_pretty(w, &ingredients)?;
                    } else {
                        serde_json::to_writer(w, &ingredients)?;
                    }
                }
                OutputFormat::Yaml => {
                    // Output as a YAML array of strings
                    let ingredients: Vec<String> =
                        list.into_iter().map(|(ingredient, _)| ingredient).collect();
                    serde_yaml::to_writer(w, &ingredients)?;
                }
                OutputFormat::Markdown => {
                    let value = build_md_value(list, &aisle, args.plain, args.ingredients_only);
                    write!(w, "{value}")?;
                }
            }
        } else {
            match format {
                OutputFormat::Human => {
                    let table = build_human_table(list, &aisle, args.plain);
                    write!(w, "{table}")?;
                }
                OutputFormat::Json => {
                    let value = build_json_value(list, &aisle, args.plain);
                    if args.pretty {
                        serde_json::to_writer_pretty(w, &value)?;
                    } else {
                        serde_json::to_writer(w, &value)?;
                    }
                }
                OutputFormat::Yaml => {
                    let value = build_yaml_value(list, &aisle);

                    serde_yaml::to_writer(w, &value)?;
                }
                OutputFormat::Markdown => {
                    let value = build_md_value(list, &aisle, args.plain, args.ingredients_only);
                    write!(w, "{value}")?;
                }
            }
        }
        Ok(())
    })
}

fn total_quantity_fmt(qty: &GroupedQuantity, row: &mut tabular::Row) {
    let content = qty
        .iter()
        .map(quantity_fmt)
        .reduce(|s, q| format!("{s}, {q}"))
        .unwrap_or_default();
    row.add_ansi_cell(content);
}

fn quantity_fmt(qty: &Quantity) -> String {
    if let Some(unit) = qty.unit() {
        format!("{} {}", qty.value(), unit)
    } else {
        format!("{}", qty.value())
    }
}

fn build_human_table(list: IngredientList, aisle: &AisleConf, plain: bool) -> tabular::Table {
    let mut table = tabular::Table::new("{:<} {:<}");
    if plain {
        for (igr, q) in list {
            let mut row = tabular::Row::new().with_cell(igr);
            total_quantity_fmt(&q, &mut row);
            table.add_row(row);
        }
    } else {
        let categories = list.categorize(aisle);
        for (cat, items) in categories {
            table.add_heading(format!("[{}]", cat.green()));
            for (igr, q) in items {
                let mut row = tabular::Row::new().with_cell(igr);
                total_quantity_fmt(&q, &mut row);
                table.add_row(row);
            }
        }
    }
    table
}

fn build_md_value(
    list: IngredientList,
    aisle: &AisleConf,
    plain: bool,
    ingredients_only: bool,
) -> String {
    let mut output = String::new();

    let format_ingredient = |ingredient: &str, quantity: &GroupedQuantity| {
        if ingredients_only {
            format!("- {ingredient}\n")
        } else {
            let quantity_string = quantity
                .iter()
                .map(quantity_fmt)
                .collect::<Vec<_>>()
                .join(", ");
            format!("- *{quantity_string}* {ingredient}\n")
        }
    };
    if plain {
        // no categories, simple list
        for (ingredient, quantity) in list {
            output.push_str(&format_ingredient(&ingredient, &quantity));
        }
    } else {
        let categories = list.categorize(aisle);
        for (i, (category, items)) in categories.into_iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            output.push_str(&format!("# {category}\n"));
            for (ingredient, quantity) in items {
                output.push_str(&format_ingredient(&ingredient, &quantity));
            }
        }
    }
    output
}

fn build_json_value<'a>(
    list: IngredientList,
    aisle: &'a AisleConf<'a>,
    plain: bool,
) -> serde_json::Value {
    #[derive(Serialize)]
    struct Quantity {
        value: Value,
        unit: Option<String>,
    }
    impl From<cooklang::quantity::Quantity> for Quantity {
        fn from(qty: cooklang::quantity::Quantity) -> Self {
            let unit = qty.unit().map(|s| s.to_owned());
            let value = qty.value().clone();
            Self { value, unit }
        }
    }
    #[derive(Serialize)]
    struct Ingredient {
        name: String,
        quantity: Vec<Quantity>,
    }
    impl From<(String, GroupedQuantity)> for Ingredient {
        fn from((name, qty): (String, GroupedQuantity)) -> Self {
            Ingredient {
                name,
                quantity: qty.into_vec().into_iter().map(Quantity::from).collect(),
            }
        }
    }
    #[derive(Serialize)]
    struct Category {
        category: String,
        items: Vec<Ingredient>,
    }

    if plain {
        serde_json::to_value(list.into_iter().map(Ingredient::from).collect::<Vec<_>>()).unwrap()
    } else {
        serde_json::to_value(
            list.categorize(aisle)
                .into_iter()
                .map(|(category, items)| Category {
                    category,
                    items: items.into_iter().map(Ingredient::from).collect(),
                })
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }
}

fn build_yaml_value<'a>(list: IngredientList, aisle: &'a AisleConf<'a>) -> serde_yaml::Value {
    #[derive(Serialize)]
    struct Quantity {
        value: Value,
        unit: Option<String>,
    }
    impl From<cooklang::quantity::Quantity> for Quantity {
        fn from(qty: cooklang::quantity::Quantity) -> Self {
            let unit = qty.unit().map(|s| s.to_owned());
            let value = qty.value().clone();
            Self { value, unit }
        }
    }
    #[derive(Serialize)]
    struct Ingredient {
        name: String,
        quantity: Vec<Quantity>,
    }
    impl From<(String, GroupedQuantity)> for Ingredient {
        fn from((name, qty): (String, GroupedQuantity)) -> Self {
            Ingredient {
                name,
                quantity: qty.into_vec().into_iter().map(Quantity::from).collect(),
            }
        }
    }
    #[derive(Serialize)]
    struct Category {
        category: String,
        items: Vec<Ingredient>,
    }

    // Convert to categorized list and serialize to YAML
    serde_yaml::to_value(
        list.categorize(aisle)
            .into_iter()
            .map(|(category, items)| Category {
                category,
                items: items.into_iter().map(Ingredient::from).collect(),
            })
            .collect::<Vec<_>>(),
    )
    .unwrap()
}
