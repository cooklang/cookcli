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

use anyhow::{Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::CommandFactory;
use cooklang::{ingredient_list::IngredientList, quantity::Value, Converter};
use cooklang_find::RecipeEntry;
use std::collections::BTreeMap;

pub const RECIPE_SCALING_DELIMITER: char = ':';

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
    let recipe = recipe_entry.recipe(scaling_factor);
    let ref_indices = list.add_recipe(&recipe, converter, ignore_references);

    if !ignore_references {
        for ref_index in ref_indices {
            let ingredient = &recipe.ingredients[ref_index];
            let reference = ingredient.reference.as_ref().unwrap();

            let suffix = match ingredient.quantity.as_ref() {
                Some(quantity) => {
                    if quantity.unit().is_some() {
                        return Err(anyhow::anyhow!(
                            "Unit not supported for referenced ingredients: {}({}). See https://github.com/cooklang/cookcli/issues/137",
                            ingredient.name,
                            quantity
                        ));
                    } else {
                        match quantity.value() {
                            Value::Number(value) => value.to_string(),
                            _ => String::from(""),
                        }
                    }
                }
                None => scaling_factor.to_string(),
            };

            let path = reference.path("/") + ":" + &suffix;

            extract_ingredients(
                path.as_str(),
                list,
                seen,
                base_path,
                converter,
                ignore_references,
            )?;
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
