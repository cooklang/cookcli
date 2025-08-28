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

pub mod cooklang_to_cooklang;
pub mod cooklang_to_human;
pub mod cooklang_to_md;
pub mod format;

use anyhow::{Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::CommandFactory;
use cooklang::{
    ingredient_list::IngredientList, quantity::Value, Converter, CooklangParser, Extensions, Recipe,
};
use cooklang_find::RecipeEntry;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::warn;

pub const RECIPE_SCALING_DELIMITER: char = ':';

pub static PARSER: Lazy<CooklangParser> = Lazy::new(|| {
    // Use no extensions but with default converter for basic unit support
    CooklangParser::new(Extensions::empty(), Converter::default())
});

/// Parse a Recipe from a RecipeEntry with the given scaling factor
pub fn parse_recipe_from_entry(entry: &RecipeEntry, scaling_factor: f64) -> Result<Arc<Recipe>> {
    let content = entry.content().context("Failed to read recipe content")?;
    let parsed = PARSER.parse(&content);

    // Log any warnings
    if parsed.report().has_warnings() {
        let recipe_name = entry.name().as_deref().unwrap_or("unknown");
        for warning in parsed.report().warnings() {
            warn!("Recipe '{}': {}", recipe_name, warning);
        }
    }

    let (mut recipe, _warnings) = parsed.into_result().context("Failed to parse recipe")?;

    // Scale the recipe
    recipe.scale(scaling_factor, PARSER.converter());
    Ok(Arc::new(recipe))
}

pub fn write_to_output<F>(output: Option<&Utf8Path>, f: F) -> Result<()>
where
    F: FnOnce(Box<dyn std::io::Write>) -> Result<()>,
{
    let stream: Box<dyn std::io::Write> = if let Some(path) = output {
        let file = std::fs::File::create(path).context("Failed to create output file")?;
        let stream = anstream::StripStream::new(file);
        Box::new(stream)
    } else {
        Box::new(anstream::stdout().lock())
    };
    f(stream)?;
    Ok(())
}

pub fn split_recipe_name_and_scaling_factor(query: &str) -> Option<(&str, &str)> {
    query.trim().rsplit_once(RECIPE_SCALING_DELIMITER)
}

/// Resolves a path to an absolute path. If the input path is already absolute,
/// it is returned as is. Otherwise, it is resolved relative to the current working directory.
/// The path is normalized to remove any `.` or `..` components.
pub fn resolve_to_absolute_path(path: &Utf8Path) -> anyhow::Result<Utf8PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| {
                tracing::error!("Failed to get current directory: {:?}", e);
                anyhow::anyhow!("Failed to get current directory")
            })?
            .join(path)
            .try_into()
            .map_err(|e| {
                tracing::error!("Failed to convert path to UTF-8: {:?}", e);
                anyhow::anyhow!("Failed to convert path to UTF-8")
            })?
    };

    // Normalize the path by resolving all components
    std::fs::canonicalize(&absolute)
        .map_err(|e| {
            tracing::error!("Failed to canonicalize path: {:?}", e);
            anyhow::anyhow!("Failed to canonicalize path")
        })?
        .try_into()
        .map_err(|e| {
            tracing::error!("Failed to convert canonicalized path to UTF-8: {:?}", e);
            anyhow::anyhow!("Failed to convert canonicalized path to UTF-8")
        })
}

pub fn extract_ingredients(
    entry: &str,
    list: &mut IngredientList,
    seen: &mut BTreeMap<String, usize>,
    base_path: &Utf8PathBuf,
    converter: &Converter,
    ignore_references: bool,
) -> Result<()> {
    if seen.contains_key(entry) {
        return Err(anyhow::anyhow!(
            "Circular dependency found: {} -> {}",
            seen.keys().cloned().collect::<Vec<_>>().join(" -> "),
            entry
        ));
    }

    seen.insert(entry.to_string(), seen.len());

    // split into name and servings
    let (name, scaling_factor) = split_recipe_name_and_scaling_factor(entry)
        .map(|(name, scaling_factor)| {
            let target = scaling_factor.parse::<f64>().unwrap_or_else(|err| {
                let mut cmd = crate::CliArgs::command();
                cmd.error(
                    clap::error::ErrorKind::InvalidValue,
                    format!("Invalid scaling target for '{name}': {err}"),
                )
                .exit()
            });
            (name, target)
        })
        .unwrap_or((entry, 1.0));

    let recipe_entry = get_recipe(base_path, name)?;
    let recipe = parse_recipe_from_entry(&recipe_entry, scaling_factor)?;
    let ref_indices = list.add_recipe(&recipe, converter, ignore_references);

    if !ignore_references {
        // Determine the base path for resolving references
        // If the recipe has a path, use its parent directory as the base
        let ref_base_path = recipe_entry
            .path()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| base_path.clone());

        for ref_index in ref_indices {
            let ingredient = &recipe.ingredients[ref_index];
            let reference = ingredient.reference.as_ref().unwrap();

            // Get the referenced recipe path
            // Handle the case where components might be ["."] or empty
            let ref_path = if reference.components.is_empty() {
                reference.name.clone()
            } else if reference.components.len() == 1 && reference.components[0] == "." {
                format!("./{}", reference.name)
            } else {
                reference.path("/")
            };

            // If the reference starts with ./ or ../, resolve it relative to the recipe's location
            // Otherwise, use the original base_path
            let search_base = if ref_path.starts_with("./") || ref_path.starts_with("../") {
                &ref_base_path
            } else {
                base_path
            };

            let ref_entry = get_recipe(search_base, &ref_path)?;

            // Parse and scale the recipe based on the quantity specification
            let ref_recipe = match ingredient.quantity.as_ref() {
                Some(quantity) => {
                    let target_value = match quantity.value() {
                        Value::Number(num) => num
                            .to_string()
                            .parse::<f64>()
                            .map_err(|_| anyhow::anyhow!("Invalid numeric value: {}", num))?,
                        _ => {
                            return Err(anyhow::anyhow!(
                                "Invalid quantity value for referenced recipe: {}",
                                ingredient.name
                            ));
                        }
                    };

                    let content = ref_entry
                        .content()
                        .context("Failed to read recipe content")?;
                    let (mut recipe, _warnings) = PARSER
                        .parse(&content)
                        .into_result()
                        .context("Failed to parse recipe")?;

                    // Use the new scale_to_target function
                    recipe
                        .scale_to_target(target_value, quantity.unit(), PARSER.converter())
                        .context(format!(
                            "Failed to scale recipe '{}' with target {} {}",
                            ref_path,
                            target_value,
                            quantity.unit().unwrap_or("(no unit)")
                        ))?;

                    // Apply any additional CLI scaling
                    if scaling_factor != 1.0 {
                        recipe.scale(scaling_factor, PARSER.converter());
                    }

                    Arc::new(recipe)
                }
                None => {
                    // No quantity specified, use CLI scaling only
                    parse_recipe_from_entry(&ref_entry, scaling_factor)?
                }
            };

            // Add the scaled recipe's ingredients to the list
            list.add_recipe(&ref_recipe, converter, true);
        }
    }

    seen.remove(entry);

    Ok(())
}

pub fn get_recipe(base_path: &Utf8PathBuf, name: &str) -> Result<RecipeEntry> {
    Ok(cooklang_find::get_recipe(
        vec![base_path.clone()],
        name.into(),
    )?)
}
