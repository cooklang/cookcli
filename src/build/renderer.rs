use crate::build::links::relative_prefix;
use crate::build::writer::write_html;
use crate::server::builders::{build_recipes_template, RecipesBuildInput};
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
