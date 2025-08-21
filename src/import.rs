use anyhow::Result;
use clap::Args;
use cooklang_import::{fetch_recipe, import_recipe};

use crate::Context;

#[derive(Debug, Args)]
pub struct ImportArgs {
    /// URL of the recipe webpage to import
    ///
    /// The importer supports many popular recipe websites and will
    /// automatically extract ingredients, instructions, and metadata.
    /// The recipe will be converted to Cooklang format unless
    /// --skip-conversion is used.
    ///
    /// Example URLs:
    ///   https://www.allrecipes.com/recipe/...
    ///   https://www.bbcgoodfood.com/recipes/...
    ///   https://cooking.nytimes.com/recipes/...
    #[arg(value_name = "URL")]
    url: String,

    /// Output the original recipe data without converting to Cooklang
    ///
    /// By default, imported recipes are converted to Cooklang format.
    /// Use this flag to get the raw recipe data as extracted from
    /// the website (useful for debugging or custom processing).
    #[arg(short, long)]
    skip_conversion: bool,
}

pub fn run(_ctx: &Context, args: ImportArgs) -> Result<()> {
    let recipe = tokio::runtime::Runtime::new()?.block_on(async {
        if args.skip_conversion {
            let recipe = fetch_recipe(&args.url)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok(format!(
                "{}\n\n[Ingredients]\n{}\n\n[Instructions]\n{}",
                recipe.name, recipe.ingredients, recipe.instructions
            ))
        } else {
            import_recipe(&args.url)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        }
    })?;

    println!("{recipe}");
    Ok(())
}
