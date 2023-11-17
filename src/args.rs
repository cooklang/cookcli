use clap::{Parser, Subcommand};

use crate::{recipe, seed, server, shopping_list};

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
}
