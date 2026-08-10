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
    util::{
        format::{self, Style},
        write_to_output, PARSER,
    },
    Context,
};

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
    paper_size: Option<PaperSizeArg>,

    /// Page margin in centimeters for LaTeX and Typst output (default: 2.5)
    ///
    /// Applied equally to all four sides. Has no effect on other formats.
    #[arg(short, long)]
    margin: Option<f64>,
}

/// Clap's view of [`format::PaperSize`].
///
/// The paper names themselves live in `cookcli-core`, which must not depend on
/// clap; this enum exists only to derive [`ValueEnum`], and its variants must
/// stay in step with core's so that `--paper-size` keeps accepting the same
/// values.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum PaperSizeArg {
    A4,
    Letter,
    A5,
    Legal,
}

impl From<PaperSizeArg> for format::PaperSize {
    fn from(value: PaperSizeArg) -> Self {
        match value {
            PaperSizeArg::A4 => format::PaperSize::A4,
            PaperSizeArg::Letter => format::PaperSize::Letter,
            PaperSizeArg::A5 => format::PaperSize::A5,
            PaperSizeArg::Legal => format::PaperSize::Legal,
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
    let source = match args.input.recipe {
        Some(query) => cookcli_core::RecipeSource::Path(query),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("Failed to read stdin")?;
            cookcli_core::RecipeSource::Content {
                text: buf,
                name: "stdin".to_string(),
            }
        }
    };

    let outcome = cookcli_core::recipe::read(
        &ctx.to_core(),
        cookcli_core::recipe::ReadRequest {
            source,
            scale: args.input.scale,
        },
    )
    .map_err(|e| match e {
        // CoreError::Parse has a single-line Display by library convention.
        // The CLI printed cooklang's full rendered report with source line
        // context before this refactor, so re-attach it here.
        cookcli_core::CoreError::Parse {
            ref name,
            ref rendered,
            ..
        } => anyhow::anyhow!("Failed to parse recipe '{name}'\n{rendered}"),
        other => other.into(),
    })?;

    for diagnostic in &outcome.diagnostics {
        tracing::warn!("{}", diagnostic.message);
    }

    let recipe = outcome.value.recipe;
    let title = outcome.value.title;
    // The effective scale, which an inline `name:factor` may have overridden.
    let scale = outcome.value.scale;

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

    let paper_size: format::PaperSize = args.paper_size.unwrap_or(PaperSizeArg::A4).into();
    let margin = args.margin.unwrap_or(2.5);

    write_to_output(args.output.as_deref(), |mut writer| {
        match format {
            // `Style::Ansi` keeps the terminal colours the CLI has always
            // printed; `write_to_output` strips them again for file output.
            OutputFormat::Human => format::human::print_human(
                &recipe,
                &title,
                scale,
                PARSER.converter(),
                Style::Ansi,
                &mut writer,
            )?,
            OutputFormat::Json => {
                if args.pretty {
                    serde_json::to_writer_pretty(writer, &recipe)?;
                } else {
                    serde_json::to_writer(writer, &recipe)?;
                }
            }
            OutputFormat::Cooklang => format::cooklang::print_cooklang(&recipe, writer)?,
            OutputFormat::Yaml => serde_yaml::to_writer(writer, &recipe)?,
            OutputFormat::Markdown => {
                format::markdown::print_md(&recipe, &title, scale, PARSER.converter(), writer)?
            }
            OutputFormat::Latex => format::latex::print_latex(
                &recipe,
                &title,
                scale,
                PARSER.converter(),
                writer,
                paper_size,
                margin,
            )?,
            OutputFormat::Typst => format::typst::print_typst(
                &recipe,
                &title,
                scale,
                PARSER.converter(),
                writer,
                paper_size,
                margin,
            )?,
            OutputFormat::Schema => format::schema::print_schema(
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
