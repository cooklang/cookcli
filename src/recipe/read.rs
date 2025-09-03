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
use clap::{Args, CommandFactory, ValueEnum};
use std::io::Read;

use camino::Utf8PathBuf;

use crate::{
    util::{split_recipe_name_and_scaling_factor, write_to_output, PARSER},
    Context,
};
use cooklang_find::RecipeEntry;

#[derive(Debug, Args)]
pub struct ReadArgs {
    #[command(flatten)]
    input: super::RecipeInputArgs,

    /// File to write output (stdout if not specified)
    ///
    /// The output format can be automatically inferred from the file
    /// extension (.json, .yaml, .md, .cook, .txt)
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    output: Option<Utf8PathBuf>,

    /// Output format for the recipe
    ///
    /// Available formats:
    ///   human    - Human-readable text with formatting (default)
    ///   json     - JSON representation of the recipe data
    ///   yaml     - YAML representation of the recipe data
    ///   cooklang - Regenerated Cooklang format
    ///   markdown - Markdown formatted recipe
    ///
    /// If not specified, format is inferred from output file extension.
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// Enable pretty formatting for structured output
    ///
    /// Adds indentation and formatting to JSON and YAML output.
    /// Has no effect on human, cooklang, or markdown formats.
    #[arg(long)]
    pretty: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
    #[value(alias("yml"))]
    Yaml,
    #[value(alias("cook"))]
    Cooklang,
    #[value(alias("md"))]
    Markdown,
}

pub fn run(ctx: &Context, args: ReadArgs) -> Result<()> {
    let mut scale = args.input.scale;

    let (recipe, title) = if let Some(query) = args.input.recipe {
        let (name, scaling_factor) = split_recipe_name_and_scaling_factor(query.as_str())
            .map(|(name, scaling_factor)| {
                let target = scaling_factor.parse::<f64>().unwrap_or_else(|err| {
                    let mut cmd = crate::args::CliArgs::command();
                    cmd.error(
                        clap::error::ErrorKind::InvalidValue,
                        format!("Invalid scaling target for '{name}': {err}. Use a number value after : to specify a scaling factor."),
                    )
                    .exit()
                });
                (name, Some(target))
            })
            .unwrap_or((query.as_str(), None));

        if let Some(scaling_factor) = scaling_factor {
            scale = scaling_factor;
        }

        let recipe_entry = cooklang_find::get_recipe(vec![ctx.base_path().clone()], name.into())
            .map_err(|e| anyhow::anyhow!("Recipe not found: {}", e))?;
        let recipe = crate::util::parse_recipe_from_entry(&recipe_entry, scale)?;
        (recipe, recipe_entry.name().clone().unwrap_or(String::new()))
    } else {
        // Read from stdin and create a RecipeEntry
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("Failed to read stdin")?;

        // Create a RecipeEntry from the stdin content
        let recipe_entry = RecipeEntry::from_content(buf, Some("stdin".to_string()))
            .context("Failed to create recipe entry from stdin")?;

        // Use the same parsing function as for file-based recipes
        let recipe = crate::util::parse_recipe_from_entry(&recipe_entry, scale)?;
        (recipe, recipe_entry.name().clone().unwrap_or(String::new()))
    };

    let format = args.format.unwrap_or_else(|| match &args.output {
        Some(p) => match p.extension() {
            Some("json") => OutputFormat::Json,
            Some("cook") => OutputFormat::Cooklang,
            Some("md") => OutputFormat::Markdown,
            Some("yaml") => OutputFormat::Yaml,
            Some("yml") => OutputFormat::Yaml,
            _ => OutputFormat::Human,
        },
        None => OutputFormat::Human,
    });

    write_to_output(args.output.as_deref(), |writer| {
        match format {
            OutputFormat::Human => crate::util::cooklang_to_human::print_human(
                &recipe,
                &title,
                scale,
                PARSER.converter(),
                writer,
            )?,
            OutputFormat::Json => {
                if args.pretty {
                    serde_json::to_writer_pretty(writer, &recipe)?;
                } else {
                    serde_json::to_writer(writer, &recipe)?;
                }
            }
            OutputFormat::Cooklang => {
                crate::util::cooklang_to_cooklang::print_cooklang(&recipe, writer)?
            }
            OutputFormat::Yaml => serde_yaml::to_writer(writer, &recipe)?,
            OutputFormat::Markdown => crate::util::cooklang_to_md::print_md(
                &recipe,
                &title,
                scale,
                PARSER.converter(),
                writer,
            )?,
        }

        Ok(())
    })?;

    Ok(())
}
