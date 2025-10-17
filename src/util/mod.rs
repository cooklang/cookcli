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
pub mod cooklang_to_latex;
pub mod cooklang_to_md;
pub mod cooklang_to_schema;
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
    F: FnOnce(&mut dyn std::io::Write) -> Result<()>,
{
    let mut stream: Box<dyn std::io::Write> = if let Some(path) = output {
        let file = std::fs::File::create(path).context("Failed to create output file")?;
        let stream = anstream::StripStream::new(file);
        Box::new(stream)
    } else {
        Box::new(anstream::stdout().lock())
    };
    f(stream.as_mut())?;
    // Explicitly flush the stream to ensure all output is written
    use std::io::Write;
    stream.flush()?;
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
                let mut cmd = crate::args::CliArgs::command();
                cmd.error(
                    clap::error::ErrorKind::InvalidValue,
                    format!("Invalid scaling target for '{name}': {err}"),
                )
                .exit()
            });
            (name, target)
        })
        .unwrap_or((entry, 1.0));

    let recipe_entry =
        get_recipe(base_path, name).with_context(|| format!("Failed to find recipe '{name}'"))?;
    let recipe = parse_recipe_from_entry(&recipe_entry, scaling_factor)?;
    let ref_indices = list.add_recipe(&recipe, converter, ignore_references);

    tracing::debug!(
        "ignore_references = {}, ref_indices.len() = {}",
        ignore_references,
        ref_indices.len()
    );
    if !ignore_references {
        for ref_index in ref_indices {
            let ingredient = &recipe.ingredients[ref_index];
            let reference = ingredient.reference.as_ref().unwrap();

            let ref_path = reference.path(&std::path::MAIN_SEPARATOR.to_string());

            let ref_entry = get_recipe(&base_path, &ref_path).with_context(|| {
                format!(
                    "Failed to find referenced recipe '{}' from '{}'",
                    ref_path,
                    recipe_entry
                        .path()
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| name.to_string())
                )
            })?;

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
                    tracing::debug!(
                        "Scaling recipe '{}' to target {} {}",
                        ref_path,
                        target_value,
                        quantity.unit().unwrap_or("(no unit)")
                    );
                    recipe
                        .scale_to_target(target_value, quantity.unit(), PARSER.converter())
                        .context(format!(
                            "Failed to scale recipe '{}' with target {} {}",
                            ref_path,
                            target_value,
                            quantity.unit().unwrap_or("(no unit)")
                        ))?;

                    // Don't apply additional CLI scaling when using scale_to_target
                    // The target value already accounts for the scaling

                    Arc::new(recipe)
                }
                None => {
                    // No quantity specified, use CLI scaling only
                    parse_recipe_from_entry(&ref_entry, scaling_factor)?
                }
            };

            // Find ingredients with references that need to be processed
            let mut nested_refs = Vec::new();
            for (index, ingredient) in ref_recipe.ingredients.iter().enumerate() {
                if ingredient.reference.is_some() {
                    nested_refs.push(index);
                }
            }

            // Process nested references recursively
            tracing::debug!("Found {} nested references to process", nested_refs.len());
            for nested_index in nested_refs {
                let nested_ingredient = &ref_recipe.ingredients[nested_index];
                tracing::debug!("Processing nested ingredient: {:?}", nested_ingredient.name);
                if let Some(nested_ref) = &nested_ingredient.reference {
                    // Build the full path for the nested reference
                    let nested_path = if nested_ref.components.is_empty() {
                        nested_ref.name.clone()
                    } else {
                        let sep = std::path::MAIN_SEPARATOR.to_string();
                        format!(".{}{}{}{}", sep, nested_ref.components.join(&sep), sep, nested_ref.name)
                    };

                    // Get the nested recipe to check its servings metadata
                    let nested_entry_path = get_recipe(&base_path, &nested_path)?;
                    let nested_content = nested_entry_path
                        .content()
                        .context("Failed to read nested recipe")?;
                    let (nested_recipe, _) = PARSER
                        .parse(&nested_content)
                        .into_result()
                        .context("Failed to parse nested recipe")?;

                    // For nested references, we need to handle scaling properly based on units
                    if let Some(quantity) = &nested_ingredient.quantity {
                        if quantity.unit() == Some("servings") {
                            // This is a servings-based reference
                            // The quantity value is the target number of servings we need
                            if let Value::Number(target_servings) = quantity.value() {
                                // We need to scale the nested recipe to produce target_servings
                                let mut scaled_nested = nested_recipe;
                                let target = target_servings.to_string().parse().unwrap_or(1.0);
                                tracing::debug!("Scaling nested recipe to {} servings", target);
                                scaled_nested
                                    .scale_to_target(target, Some("servings"), PARSER.converter())
                                    .context("Failed to scale nested recipe")?;

                                // Now add this properly scaled nested recipe's ingredients
                                // Pass false to exclude references - they will be handled recursively
                                list.add_recipe(&Arc::new(scaled_nested), converter, false);
                            }
                        } else {
                            // For non-servings units, treat the quantity as a regular scaling factor
                            // This handles cases like "2 cups" of something
                            if let Value::Number(num) = quantity.value() {
                                let scaling = num.to_string().parse().unwrap_or(1.0);
                                let mut scaled_nested = nested_recipe;
                                scaled_nested.scale(scaling, PARSER.converter());
                                list.add_recipe(&Arc::new(scaled_nested), converter, false);
                            }
                        }
                    } else {
                        // No quantity specified, use scale 1.0
                        list.add_recipe(&Arc::new(nested_recipe), converter, false);
                    }
                }
            }

            // Now add the non-reference ingredients from the recipe
            // We need to do this AFTER processing nested references to avoid duplicates
            // Pass false to exclude references since we've already expanded them
            list.add_recipe(&ref_recipe, converter, false);
        }
    }

    seen.remove(entry);

    Ok(())
}

pub fn get_recipe(base_path: &Utf8PathBuf, name: &str) -> Result<RecipeEntry> {
    // Remove ./ prefix if present before passing to cooklang_find
    // The cooklang-find library doesn't expect the ./ prefix
    let clean_name = name.strip_prefix("./").unwrap_or(name);

    Ok(cooklang_find::get_recipe(
        vec![base_path.clone()],
        clean_name.into(),
    )?)
}
