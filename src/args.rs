use clap::{Parser, Subcommand};

use crate::{recipe, shopping_list, version};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    after_help = "Docs: https://github.com/cooklang/cooklang-chef/blob/main/docs/README.md"
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
    // /// Recipes web server
    // Serve(serve::ServeArgs),
    /// Creates a shopping list from a given list of recipes
    #[command(visible_alias = "sl")]
    ShoppingList(shopping_list::ShoppingListArgs),
    /// Version
    Version(version::VersionArgs),
}
