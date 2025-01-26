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

use std::io::Read;

use anyhow::{Context as _, Result};
use camino::Utf8PathBuf;
use clap::{Args, Subcommand};

use crate::{util::Input, Context};
use cooklang_fs::LazyFsIndex;

mod image;
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
    /// Parse and print a Cooklang recipe file
    #[command(alias = "r")]
    Read(read::ReadArgs),

    /// Download a random image from unsplash.com to match the recipe title.
    /// Environment variable COOK_UNSPLASH_ACCESS_KEY needs to be set.
    #[command()]
    Image(image::ImageArgs),
}

pub fn run(ctx: &Context, args: RecipeArgs) -> Result<()> {
    let command = args.command.unwrap_or(RecipeCommand::Read(args.read_args));

    match command {
        RecipeCommand::Read(args) => read::run(ctx, args),
        RecipeCommand::Image(args) => image::run(ctx, args),
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
    pub fn read(&self, index: &LazyFsIndex) -> Result<Input> {
        let input = if let Some(query) = &self.recipe {
            // RecipeInputArgs::recipe is a pathbuf even if inmediatly converted
            // to a string to enforce validation.
            let entry = index.resolve(query.as_str(), None)?;

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
