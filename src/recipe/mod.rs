use std::io::Read;

use anyhow::{Context as _, Result};
use camino::Utf8PathBuf;
use clap::{Args, Subcommand};

use crate::{util::Input, Context};
use cooklang_fs::{resolve_recipe, FsIndex};

mod read;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct RecipeArgs {
    #[command(subcommand)]
    command: Option<RecipeCommand>,

    #[command(flatten)]
    read_args: read::ReadArgs,
}

#[derive(Debug, Subcommand)]
enum RecipeCommand {
    /// Reads a recipe
    #[command(alias = "r")]
    Read(read::ReadArgs),
}

pub fn run(ctx: &Context, args: RecipeArgs) -> Result<()> {
    let command = args.command.unwrap_or(RecipeCommand::Read(args.read_args));

    match command {
        RecipeCommand::Read(args) => read::run(ctx, args),
    }
}

#[derive(Debug, Args)]
struct RecipeInputArgs {
    /// Input recipe, none for stdin
    ///
    /// This can be a full path, a partial path, or just the name.
    #[arg(value_hint = clap::ValueHint::FilePath)]
    recipe: Option<Utf8PathBuf>,
}

impl RecipeInputArgs {
    pub fn read(&self, index: &FsIndex) -> Result<Input> {
        let input = if let Some(query) = &self.recipe {
            // RecipeInputArgs::recipe is a pathbuf even if inmediatly converted
            // to a string to enforce validation.
            let entry = resolve_recipe(query.as_str(), index, None)?;

            Input::File {
                content: entry.read()?,
            }
        } else {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("Failed to read stdin")?;
            Input::Stdin { text: buf }
        };
        Ok(input)
    }
}
