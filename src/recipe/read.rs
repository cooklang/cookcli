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
    #[value(alias("cook"))]
    Cooklang,
    #[value(alias("md"))]
    Markdown,
}

pub fn run(ctx: &Context, args: ReadArgs) -> Result<()> {
    let input = args.input.read(&ctx.recipe_index)?;
    let recipe = input.parse(ctx)?.default_scale();

    let format = args.format.unwrap_or_else(|| match &args.output {
        Some(p) => match p.extension() {
            Some("json") => OutputFormat::Json,
            Some("cook") => OutputFormat::Cooklang,
            Some("md") => OutputFormat::Markdown,
            _ => OutputFormat::Human,
        },
        None => OutputFormat::Human,
    });

    write_to_output(args.output.as_deref(), |writer| {
        match format {
            OutputFormat::Human => {
                cooklang_to_human::print_human(&recipe, ctx.parser()?.converter(), writer)?
            }
            OutputFormat::Json => {
                if args.pretty {
                    serde_json::to_writer_pretty(writer, &recipe)?;
                } else {
                    serde_json::to_writer(writer, &recipe)?;
                }
            }
            OutputFormat::Cooklang => cooklang_to_cooklang::print_cooklang(&recipe, writer)?,
            OutputFormat::Markdown => {
                cooklang_to_md::print_md(&recipe, ctx.parser()?.converter(), writer)?
            }
        }

        Ok(())
    })?;

    Ok(())
}
