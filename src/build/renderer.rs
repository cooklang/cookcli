use crate::build::links::relative_prefix;
use crate::build::writer::write_html;
use crate::server::builders::{
    build_recipe_template, build_recipes_template, RecipeBuildInput, RecipeBuildOutput,
    RecipesBuildInput,
};
use anyhow::Result;
use askama::Template;
use camino::{Utf8Path, Utf8PathBuf};
use unic_langid::LanguageIdentifier;

/// Render the root index page (recipes listing).
pub fn render_index(
    source: &Utf8Path,
    output: &Utf8Path,
    base_url: Option<&str>,
    lang: &LanguageIdentifier,
) -> Result<()> {
    let relpath = Utf8PathBuf::from("index.html");
    let prefix = compute_prefix(base_url, &relpath);
    let template = build_recipes_template(RecipesBuildInput {
        base_path: source,
        url_prefix: &prefix,
        sub_path: None,
        lang: lang.clone(),
        static_mode: true,
    })?;
    let html = template.render()?;
    write_html(output, &relpath, &html)
}

/// Render one directory listing page.
pub fn render_directory(
    source: &Utf8Path,
    output: &Utf8Path,
    sub_path: &str,
    base_url: Option<&str>,
    lang: &LanguageIdentifier,
) -> Result<()> {
    let relpath = Utf8PathBuf::from(format!("directory/{sub_path}.html"));
    let prefix = compute_prefix(base_url, &relpath);
    let template = build_recipes_template(RecipesBuildInput {
        base_path: source,
        url_prefix: &prefix,
        sub_path: Some(sub_path),
        lang: lang.clone(),
        static_mode: true,
    })?;
    let html = template.render()?;
    write_html(output, &relpath, &html)
}

fn compute_prefix(base_url: Option<&str>, relpath: &Utf8Path) -> String {
    match base_url {
        Some(b) => b.trim_end_matches('/').to_string(),
        None => relative_prefix(relpath),
    }
}

/// Render a single recipe (or menu) page.
///
/// Both `recipe/<path>.html` and `menu/<path>.html` sit at the same depth in
/// the output tree, so the page-relative `prefix` is identical regardless of
/// which one we end up writing. We render once and pick the destination path
/// based on whether the entry turned out to be a menu.
pub fn render_recipe(
    source: &Utf8Path,
    output: &Utf8Path,
    recipe_relpath: &str,
    aisle_path: Option<&Utf8PathBuf>,
    base_url: Option<&str>,
    lang: &LanguageIdentifier,
) -> Result<()> {
    let trimmed = recipe_relpath
        .trim_end_matches(".cook")
        .trim_end_matches(".menu");
    let provisional = Utf8PathBuf::from(format!("recipe/{trimmed}.html"));
    let prefix = compute_prefix(base_url, &provisional);

    let kind = build_recipe_template(RecipeBuildInput {
        base_path: source,
        url_prefix: &prefix,
        // Pass the extension-less path so template URLs (e.g. the .cook
        // download link) match the server convention without doubling the
        // extension.
        recipe_path: trimmed,
        aisle_path,
        scale: 1.0,
        lang: lang.clone(),
        static_mode: true,
    })?;

    match kind {
        RecipeBuildOutput::Recipe(t) => {
            let html = t.render()?;
            write_html(output, &provisional, &html)
        }
        RecipeBuildOutput::Menu(t) => {
            let menu_relpath = Utf8PathBuf::from(format!("menu/{trimmed}.html"));
            let html = t.render()?;
            write_html(output, &menu_relpath, &html)
        }
    }
}
