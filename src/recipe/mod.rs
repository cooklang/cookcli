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
use clap::{Args, Subcommand};

use crate::Context;

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
}

pub fn run(ctx: &Context, args: RecipeArgs) -> Result<()> {
    let command = args.command.unwrap_or(RecipeCommand::Read(args.read_args));

    match command {
        RecipeCommand::Read(args) => read::run(ctx, args),
    }
}

#[derive(Debug, Args)]
struct RecipeInputArgs {
    /// Recipe file to read (stdin if not specified)
    ///
    /// Can be specified as:
    ///   - Full path: /path/to/recipe.cook
    ///   - Relative path: recipes/pasta.cook
    ///   - Recipe name: "Pasta Carbonara" (searches in recipe directory)
    ///   - With scaling: recipe.cook:2 or "Pasta:3" (scales by factor)
    ///   - Stdin: omit to read from standard input
    ///
    /// The .cook extension is optional and will be added automatically.
    /// When using recipe names (not paths), the tool searches in the
    /// current directory and configured recipe directories.
    #[arg(value_hint = clap::ValueHint::FilePath, value_name = "RECIPE")]
    recipe: Option<Utf8PathBuf>,

    /// Scaling factor for ingredient quantities
    ///
    /// Multiplies all ingredient quantities by this factor.
    /// Can also be specified inline with : syntax (e.g., recipe:2).
    /// The inline syntax takes precedence over this flag.
    #[arg(short, long, default_value_t = 1.0)]
    scale: f64,
}
