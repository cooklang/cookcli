use anyhow::Result;
use clap::Args;
use cooklang_import::{fetch_recipe, import_recipe};

use crate::Context;

#[derive(Debug, Args)]
pub struct ImportArgs {
    /// URL of the recipe to import
    url: String,

    /// Skip conversion to Cooklang format and just fetch the original recipe
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

    println!("{}", recipe);
    Ok(())
}
