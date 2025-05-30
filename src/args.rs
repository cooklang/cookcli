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

use clap::{Parser, Subcommand};

use crate::{recipe, search, seed, server, shopping_list, import};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    after_help = "Docs: https://cooklang.org/cli/help/"
)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage recipe files
    #[command(alias = "r")]
    Recipe(recipe::RecipeArgs),

    /// Run a webserver to serve your recipes on the web
    #[command(alias = "s")]
    Server(server::ServerArgs),

    /// Create a shopping list
    #[command(visible_alias = "sl")]
    ShoppingList(shopping_list::ShoppingListArgs),

    /// Populate directory with seed recipes
    #[command()]
    Seed(seed::SeedArgs),

    /// Search for recipes containing the given text.
    /// Multiple search terms are supported, separated by spaces.
    /// Results are sorted by relevance.
    #[command(alias = "f")]
    Search(search::SearchArgs),

    /// Import a recipe from a URL
    #[command(alias = "i")]
    Import(import::ImportArgs),
}
