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

use crate::{doctor, import, recipe, report, search, seed, server, shopping_list};

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
    /// Parse, validate and display recipe files in various formats
    ///
    /// The recipe command allows you to work with Cooklang recipe files.
    /// You can parse recipes, validate their syntax, and output them in
    /// different formats including JSON, YAML, and human-readable text.
    ///
    /// Examples:
    ///   cook recipe myrecipe.cook                 # Display recipe in human format
    ///   cook recipe myrecipe.cook -f json         # Output as JSON
    ///   cook recipe myrecipe.cook@2 -f yaml       # Scale recipe 2x and output as YAML
    #[command(alias = "r", long_about = "Parse and display Cooklang recipe files with support for multiple output formats and scaling")]
    Recipe(recipe::RecipeArgs),

    /// Start a local web server to browse and view your recipe collection
    ///
    /// The server provides a web interface for browsing your recipe collection,
    /// viewing individual recipes with scaling support, and searching through
    /// your recipes. By default, it runs on port 9080 and only accepts local
    /// connections.
    ///
    /// Examples:
    ///   cook server                    # Start server on localhost:9080
    ///   cook server --host --port 8080 # Allow external connections on port 8080
    ///   cook server ~/recipes          # Serve recipes from specific directory
    #[command(alias = "s", long_about = "Run a web server to browse and interact with your recipe collection")]
    Server(server::ServerArgs),

    /// Generate a combined shopping list from multiple recipes
    ///
    /// Creates a shopping list by aggregating ingredients from one or more recipes.
    /// Supports recipe scaling, multiple output formats, and categorization by aisle.
    /// Ingredients with the same name are automatically combined with unit conversion.
    ///
    /// Examples:
    ///   cook shopping-list recipe1.cook recipe2.cook  # Create list from two recipes
    ///   cook sl "Pasta.cook:2" "Salad.cook"           # Scale pasta recipe by 2
    ///   cook sl *.cook -f json -o list.json           # All recipes to JSON file
    ///   cook sl recipe.cook --plain                   # Without categories
    #[command(visible_alias = "sl", long_about = "Create shopping lists from one or more recipes with ingredient aggregation and categorization")]
    ShoppingList(shopping_list::ShoppingListArgs),

    /// Initialize a directory with example Cooklang recipes
    ///
    /// Creates a set of sample recipes to help you get started with Cooklang.
    /// This is useful for learning the syntax or setting up a new recipe collection.
    ///
    /// Examples:
    ///   cook seed                  # Add examples to current directory
    ///   cook seed ~/recipes        # Create examples in specific directory
    #[command(long_about = "Populate a directory with example Cooklang recipes to get started")]
    Seed(seed::SeedArgs),

    /// Search through your recipe collection for matching text
    ///
    /// Performs a full-text search across all recipe files in the specified directory.
    /// Searches through recipe titles, ingredients, instructions, and metadata.
    /// Results are ranked by relevance with the most relevant matches shown first.
    ///
    /// Examples:
    ///   cook search chicken             # Find all recipes mentioning chicken
    ///   cook search "olive oil"         # Search for exact phrase
    ///   cook search tomato basil        # Find recipes with both terms
    ///   cook search -b ~/recipes pasta  # Search in specific directory
    #[command(alias = "f", long_about = "Search for recipes by ingredient, title, or any text content with relevance ranking")]
    Search(search::SearchArgs),

    /// Import recipes from supported websites and convert to Cooklang
    ///
    /// Fetches recipes from URLs and converts them to Cooklang format.
    /// Supports many popular recipe websites and can extract ingredients,
    /// instructions, and metadata automatically.
    ///
    /// Examples:
    ///   cook import https://example.com/recipe       # Import and convert
    ///   cook import URL --skip-conversion            # Import without converting
    #[command(alias = "i", long_about = "Import recipes from websites and automatically convert them to Cooklang format")]
    Import(import::ImportArgs),

    /// Generate custom reports from recipes using templates
    ///
    /// Uses Jinja2 templates to create custom outputs from recipe data.
    /// This allows you to generate shopping lists, meal plans, nutrition
    /// cards, or any custom format you need.
    ///
    /// The template receives the full recipe data including ingredients,
    /// steps, metadata, and calculated values.
    ///
    /// Examples:
    ///   cook report -t card.j2 recipe.cook           # Generate recipe card
    ///   cook report -t nutrition.j2 recipe.cook@2    # Nutrition for 2x recipe
    ///   cook report -t plan.j2 recipe.cook -o out.md # Output to file
    #[command(alias = "rp", long_about = "Generate custom reports and outputs from recipes using Jinja2 templates")]
    Report(report::ReportArgs),

    /// Analyze your recipe collection for issues and improvements
    ///
    /// Performs various checks on your recipe collection to identify
    /// potential problems like missing aisle categories, invalid units,
    /// or syntax issues.
    ///
    /// Examples:
    ///   cook doctor                     # Run all checks
    ///   cook doctor aisle              # Check for uncategorized ingredients
    #[command(long_about = "Check recipe collection for potential issues and suggest improvements")]
    Doctor(doctor::DoctorArgs),
}
