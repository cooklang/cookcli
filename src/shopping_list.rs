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

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};
use tracing::warn;

use cookcli_core::{
    format::shopping_list as fmt,
    shopping_list::{generate, GenerateRequest, ScaledRecipe},
    ConfigSource,
};

use crate::{
    util::{cli_error, format::Style, split_recipe_name_and_scaling_factor, write_to_output},
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

    /// Load aisle configuration file
    ///
    /// The aisle file groups ingredients into categories (produce, dairy, etc.)
    /// so the shopping list is organized by store section.
    ///
    /// If not specified, the tool looks for `aisle.conf` in `./config/` and then
    /// in the global config directory (`~/.config/cook/` or the platform
    /// equivalent).
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    aisle: Option<Utf8PathBuf>,

    /// Load pantry configuration file
    ///
    /// Ingredients you already have on hand are subtracted from the shopping
    /// list. The pantry file is TOML, with optional quantities and dates.
    ///
    /// If not specified, the tool looks for `pantry.conf` in `./config/` and then
    /// in the global config directory (`~/.config/cook/` or the platform
    /// equivalent). Use --ignore-pantry to skip pantry subtraction entirely.
    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    pantry: Option<Utf8PathBuf>,

    /// Don't expand referenced recipes
    ///
    /// By default, recipes referenced from within a recipe (via @./other.cook)
    /// are expanded and their ingredients included. This flag treats each recipe
    /// in isolation.
    #[arg(short, long)]
    ignore_references: bool,

    /// Don't subtract pantry items from the shopping list
    ///
    /// By default, ingredients found in the pantry configuration are subtracted
    /// from the shopping list. This flag includes every ingredient regardless of
    /// what's in the pantry, and skips loading the pantry file altogether.
    #[arg(long)]
    ignore_pantry: bool,

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

    // Aisle and pantry resolution: an explicit flag wins, and otherwise the
    // context's own search order (local `config/`, then the global config
    // directory) stands. `--ignore-pantry` skips the pantry altogether, which
    // core spells as "no pantry configuration".
    let mut core_ctx = ctx.to_core();
    if let Some(path) = args.aisle {
        core_ctx = core_ctx.with_aisle(ConfigSource::Path(path));
    }
    if args.ignore_pantry {
        tracing::debug!("Pantry ignored via --ignore-pantry");
        core_ctx = core_ctx.with_pantry(ConfigSource::None);
    } else if let Some(path) = args.pantry {
        // A pantry the user named by hand that cannot be read is fatal: they
        // asked for it, so silently shopping for things they already own would
        // be the wrong answer.
        core_ctx = core_ctx.with_pantry(ConfigSource::Path(path));
    } else if let Err(e) = core_ctx.pantry().read() {
        // A merely discovered pantry is different: nobody asked for it, so an
        // unreadable one is a warning and the list is built without it. The
        // probe costs one extra read of a small file, and is what keeps the
        // distinction honest — checking that the path exists is not enough,
        // because a file can exist and still refuse to be read.
        warn!("Failed to read pantry file: {e}");
        core_ctx = core_ctx.with_pantry(ConfigSource::None);
    }

    let format = args.format.unwrap_or_else(|| match &args.output {
        Some(p) => match p.extension() {
            Some("json") => OutputFormat::Json,
            _ => OutputFormat::Human,
        },
        None => OutputFormat::Human,
    });

    // `name:factor` is this CLI's argument spelling, so it is unpicked here
    // rather than in core, which takes the factor as its own field.
    let recipes = expanded_recipes
        .iter()
        .map(|entry| match split_recipe_name_and_scaling_factor(entry) {
            Some((name, scale)) => ScaledRecipe::scaled(name, scale),
            None => ScaledRecipe::new(entry.as_str()),
        })
        .collect();

    let outcome = generate(
        &core_ctx,
        GenerateRequest {
            recipes,
            ignore_references: args.ignore_references,
        },
    )
    .map_err(cli_error)?;

    // Core returns its warnings instead of logging them, so that a library
    // consumer can show them its own way. Naming the file they came from is
    // this boundary's job — one list can draw on many recipes.
    for diagnostic in &outcome.diagnostics {
        match diagnostic.location.as_ref().and_then(|l| l.file.as_ref()) {
            Some(file) => warn!("{file}: {}", diagnostic.message),
            None => warn!("{}", diagnostic.message),
        }
    }

    let list = outcome.value;

    write_to_output(args.output.as_deref(), |w| {
        if args.ingredients_only {
            match format {
                OutputFormat::Human => {
                    // Simple output: one ingredient per line, no amounts
                    for item in &list.items {
                        writeln!(w, "{}", item.name)?;
                    }
                }
                OutputFormat::Json => {
                    // Output as a JSON array of strings
                    let ingredients: Vec<&str> =
                        list.items.iter().map(|i| i.name.as_str()).collect();
                    if args.pretty {
                        serde_json::to_writer_pretty(w, &ingredients)?;
                    } else {
                        serde_json::to_writer(w, &ingredients)?;
                    }
                }
                OutputFormat::Yaml => {
                    // Output as a YAML array of strings
                    let ingredients: Vec<&str> =
                        list.items.iter().map(|i| i.name.as_str()).collect();
                    serde_yaml::to_writer(w, &ingredients)?;
                }
                OutputFormat::Markdown => {
                    let value = fmt::build_md_value(list, args.plain, args.ingredients_only);
                    write!(w, "{value}")?;
                }
            }
        } else {
            match format {
                OutputFormat::Human => {
                    // `Style::Ansi` keeps the green category headings the CLI
                    // has always printed; `write_to_output` strips them again
                    // for file output.
                    let table = fmt::build_human_table(list, args.plain, Style::Ansi);
                    write!(w, "{table}")?;
                }
                OutputFormat::Json => {
                    let value = fmt::build_json_value(list, args.plain);
                    if args.pretty {
                        serde_json::to_writer_pretty(w, &value)?;
                    } else {
                        serde_json::to_writer(w, &value)?;
                    }
                }
                OutputFormat::Yaml => {
                    // No `plain`: see the note on `build_yaml_value`.
                    let value = fmt::build_yaml_value(list);

                    serde_yaml::to_writer(w, &value)?;
                }
                OutputFormat::Markdown => {
                    let value = fmt::build_md_value(list, args.plain, args.ingredients_only);
                    write!(w, "{value}")?;
                }
            }
        }
        Ok(())
    })
}
