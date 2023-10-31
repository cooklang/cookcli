use anyhow::{Context as _, Result};

use camino::Utf8Path;

use crate::Context;

pub fn write_to_output<F>(output: Option<&Utf8Path>, f: F) -> Result<()>
where
    F: FnOnce(Box<dyn std::io::Write>) -> Result<()>,
{
    let stream: Box<dyn std::io::Write> = if let Some(path) = output {
        let file = std::fs::File::create(path).context("Failed to create output file")?;
        let stream = anstream::StripStream::new(file);
        Box::new(stream)
    } else {
        Box::new(anstream::stdout().lock())
    };
    f(stream)?;
    Ok(())
}

pub enum Input {
    File { content: cooklang_fs::RecipeContent },
    Stdin { text: String },
}

impl Input {
    pub fn parse(&self, ctx: &Context) -> Result<cooklang::ScalableRecipe> {
        self.parse_result(ctx)
            .and_then(|r| unwrap_recipe(r, self.text(), ctx))
    }

    pub fn parse_result(&self, ctx: &Context) -> Result<cooklang::RecipeResult> {
        let parser = ctx.parser()?;
        let r = match self {
            Input::File { content, .. } => {
                parser.parse_with_recipe_ref_checker(content.text(), "", None)
            }
            Input::Stdin { text } => parser.parse_with_recipe_ref_checker(text, "", None),
        };
        Ok(r)
    }

    pub fn text(&self) -> &str {
        match self {
            Input::File { content, .. } => content.text(),
            Input::Stdin { text, .. } => text,
        }
    }
}

pub fn unwrap_recipe(
    r: cooklang::RecipeResult,
    _text: &str,
    _ctx: &Context,
) -> Result<cooklang::ScalableRecipe> {
    let (recipe, _) = r.into_result().unwrap();
    Ok(recipe)
}
