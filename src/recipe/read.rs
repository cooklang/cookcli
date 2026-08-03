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
use clap::{Args, ValueEnum};
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
    /// extension (.json, .yaml, .md, .cook, .tex, .typ, .txt)
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
    ///   latex    - LaTeX formatted recipe for creating cookbooks
    ///   typst    - Typst formatted recipe for creating cookbooks
    ///   schema   - Schema.org Recipe JSON-LD format
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

    /// Paper size for LaTeX and Typst output (default: a4)
    ///
    /// Has no effect on other formats.
    #[arg(short = 'p', long, value_enum)]
    paper_size: Option<PaperSize>,

    /// Page margin in centimeters for LaTeX and Typst output (default: 2.5)
    ///
    /// Applied equally to all four sides. Has no effect on other formats.
    #[arg(short, long)]
    margin: Option<f64>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PaperSize {
    A4,
    Letter,
    A5,
    Legal,
}

impl PaperSize {
    fn latex_name(self) -> &'static str {
        match self {
            PaperSize::A4 => "a4paper",
            PaperSize::Letter => "letterpaper",
            PaperSize::A5 => "a5paper",
            PaperSize::Legal => "legalpaper",
        }
    }

    fn typst_name(self) -> &'static str {
        match self {
            PaperSize::A4 => "a4",
            PaperSize::Letter => "us-letter",
            PaperSize::A5 => "a5",
            PaperSize::Legal => "us-legal",
        }
    }
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
    #[value(alias("tex"))]
    Latex,
    #[value(alias("typ"))]
    Typst,
    #[value(alias("jsonld"))]
    Schema,
}

pub fn run(ctx: &Context, args: ReadArgs) -> Result<()> {
    let mut scale = args.input.scale;

    let (recipe, title) = if let Some(query) = args.input.recipe {
        let (name, scaling_factor) = split_recipe_name_and_scaling_factor(query.as_str())
            .map(|(name, factor)| (name, Some(factor)))
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
            Some("tex") => OutputFormat::Latex,
            Some("latex") => OutputFormat::Latex,
            Some("typ") => OutputFormat::Typst,
            Some("jsonld") => OutputFormat::Schema,
            _ => OutputFormat::Human,
        },
        None => OutputFormat::Human,
    });

    if !matches!(format, OutputFormat::Latex | OutputFormat::Typst) {
        if args.paper_size.is_some() {
            eprintln!("warning: --paper-size has no effect with the selected output format");
        }
        if args.margin.is_some() {
            eprintln!("warning: --margin has no effect with the selected output format");
        }
    }

    let paper_size = args.paper_size.unwrap_or(PaperSize::A4);
    let margin = args.margin.unwrap_or(2.5);

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
            OutputFormat::Latex => crate::util::cooklang_to_latex::print_latex(
                &recipe,
                &title,
                scale,
                PARSER.converter(),
                writer,
                paper_size.latex_name(),
                margin,
            )?,
            OutputFormat::Typst => crate::util::cooklang_to_typst::print_typst(
                &recipe,
                &title,
                scale,
                PARSER.converter(),
                writer,
                paper_size.typst_name(),
                margin,
            )?,
            OutputFormat::Schema => crate::util::cooklang_to_schema::print_schema(
                &recipe,
                &title,
                scale,
                PARSER.converter(),
                writer,
                args.pretty,
            )?,
        }

        Ok(())
    })?;

    Ok(())
}
