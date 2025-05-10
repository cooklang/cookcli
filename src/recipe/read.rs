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

use crate::{util::write_to_output, Context};

#[derive(Debug, Args)]
pub struct ReadArgs {
    #[command(flatten)]
    input: super::RecipeInputArgs,

    /// Output file, none for stdout.
    #[arg(short, long)]
    output: Option<Utf8PathBuf>,

    /// Output format
    ///
    /// Tries to infer it from output file extension. Defaults to "human".
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// Pretty output format, if available
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
    let mut input = args.input.read(&ctx.base_path)?;
    let recipe = input.recipe()?;
    let title = input.name().unwrap_or("");
    let recipe = recipe.default_scale();

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
                title,
                ctx.parser()?.converter(),
                writer,
            )?,
            OutputFormat::Json => {
                if args.pretty {
                    serde_json::to_writer_pretty(writer, &recipe)?;
                } else {
                    serde_json::to_writer(writer, &recipe)?;
                }
            }
            OutputFormat::Cooklang => crate::util::cooklang_to_cooklang::print_cooklang(&recipe, writer)?,
            OutputFormat::Yaml => serde_yaml::to_writer(writer, &recipe)?,
            OutputFormat::Markdown => {
                crate::util::cooklang_to_md::print_md(&recipe, title, ctx.parser()?.converter(), writer)?
            }
        }

        Ok(())
    })?;

    Ok(())
}
