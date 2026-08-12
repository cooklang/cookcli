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

pub mod menu_scale;

// The formatters and the parser now live in `cookcli-core`. Re-exported here
// so the rest of the CLI keeps reaching them as `crate::util::format::..` and
// `crate::util::PARSER`. Core is the single definition of `PARSER`; the copy
// that used to live here was byte-for-byte the same parser configuration.
pub use cookcli_core::format;
pub use cookcli_core::parser::PARSER;

use anyhow::{Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::Recipe;
use cooklang_find::RecipeEntry;
use std::sync::Arc;
use tracing::warn;

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

    let recipe_path = entry
        .path()
        .map(|p| p.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Check for parsing errors and format them with line context
    if parsed.report().has_errors() {
        let mut error_output = Vec::new();
        parsed
            .report()
            .write(&recipe_path, &content, false, &mut error_output)
            .ok();
        let error_details = String::from_utf8_lossy(&error_output);
        return Err(anyhow::anyhow!(
            "Failed to parse recipe '{}'\n{}",
            recipe_path,
            error_details
        ));
    }

    let (mut recipe, _warnings) = parsed.into_result().expect("already checked for errors");

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

/// Present a `cookcli-core` error in the CLI's own wording.
///
/// Core renders errors as a single lowercase line, by library convention, and
/// keeps its long-form parse report in a field. Capitalising for the terminal
/// is this boundary's job, so every message the user sees reads the same way —
/// and it happens in one place, because every command that reads recipes
/// surfaces the same handful of errors.
///
/// Anything not named here converts as-is. That keeps `source` attached so
/// anyhow prints the underlying cause, at the cost of a lowercase first line.
pub fn cli_error(error: cookcli_core::CoreError) -> anyhow::Error {
    use cookcli_core::CoreError;
    match error {
        CoreError::Parse { name, rendered, .. } => {
            anyhow::anyhow!("Failed to parse recipe '{name}'\n{rendered}")
        }
        CoreError::RecipeNotFound { name } => anyhow::anyhow!("Recipe not found: {name}"),
        // Named here only for the wording `cook pantry` has always used. The
        // variant carries no source, so nothing is lost by converting it to a
        // message.
        CoreError::MissingConfig { kind } => anyhow::anyhow!("No {kind} configuration found"),
        // Named here only for the capital letter: the variant carries no
        // source, so nothing is lost by converting it to a message. `cook
        // doctor validate` is what makes this reachable — a missing or
        // mistyped `--base-path` fails before the walk starts.
        CoreError::Search { base_dir, message } => {
            anyhow::anyhow!("Cannot search '{base_dir}': {message}")
        }
        // Attach the wording to the underlying `io::Error` rather than to the
        // `CoreError`, so the chain reads `Failed to read 'x' / Caused by:
        // Permission denied` instead of repeating core's own line between the
        // two.
        //
        // Deliberately does not say *what* was being read: `CoreError::Io`
        // carries a path and no notion of the kind of file, and the commands
        // that surface it read aisle and pantry configuration as well as
        // recipes. Naming a `config/pantry.conf` a recipe would send the user
        // looking in the wrong place.
        CoreError::Io { path, source } => {
            anyhow::Error::new(source).context(format!("Failed to read '{path}'"))
        }
        other => other.into(),
    }
}

/// Split `name:factor` into its parts.
///
/// The one definition lives in `cookcli-core`; this wrapper keeps the CLI's
/// remaining call sites (`report`, `shopping_list`) reading the same way until
/// they move over too.
pub fn split_recipe_name_and_scaling_factor(query: &str) -> Option<(&str, f64)> {
    cookcli_core::recipe::split_name_and_scale(query)
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

/// Resolve a recipe name or path to a file, in CLI wording.
///
/// The lookup itself lives in `cookcli-core`; this wrapper only translates the
/// error, so the remaining CLI call sites read the same way as before.
pub fn get_recipe(base_path: &Utf8Path, name: &str) -> Result<RecipeEntry> {
    cookcli_core::find::get_recipe(base_path, name).map_err(cli_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_recipe_with_numeric_scaling_factor() {
        assert_eq!(
            split_recipe_name_and_scaling_factor("recipe.cook:2"),
            Some(("recipe.cook", 2.0))
        );
    }

    #[test]
    fn splits_recipe_with_decimal_scaling_factor() {
        assert_eq!(
            split_recipe_name_and_scaling_factor("recipe.cook:1.5"),
            Some(("recipe.cook", 1.5))
        );
    }

    #[test]
    fn returns_none_when_no_colon() {
        assert_eq!(split_recipe_name_and_scaling_factor("recipe.cook"), None);
    }

    #[test]
    fn returns_none_for_windows_absolute_path() {
        // Regression for https://github.com/cooklang/cookcli/issues/335
        assert_eq!(
            split_recipe_name_and_scaling_factor(r"C:\test\recipe.cook"),
            None
        );
    }

    #[test]
    fn returns_none_when_right_side_is_not_numeric() {
        assert_eq!(
            split_recipe_name_and_scaling_factor("recipe.cook:abc"),
            None
        );
    }
}
