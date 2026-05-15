use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct SearchEntry {
    pub title: String,
    pub path: String,
    pub tags: Vec<String>,
    pub ingredients: Vec<String>,
}

/// Build a flat list of search entries by walking the recipe tree.
pub fn build_search_index(tree: &cooklang_find::RecipeTree) -> Result<Vec<SearchEntry>> {
    let mut out = Vec::new();
    collect(tree, String::new(), &mut out);
    Ok(out)
}

fn collect(tree: &cooklang_find::RecipeTree, prefix: String, out: &mut Vec<SearchEntry>) {
    for (name, child) in &tree.children {
        if child.children.is_empty() {
            let Some(recipe) = child.recipe.as_ref() else {
                continue;
            };
            // URL path uses the on-disk file stem, not the tree key
            // (the key may be the title from metadata).
            let stem = recipe
                .file_name()
                .as_deref()
                .map(|f| {
                    f.trim_end_matches(".cook")
                        .trim_end_matches(".menu")
                        .to_string()
                })
                .unwrap_or_else(|| name.clone());
            let sub = if prefix.is_empty() {
                stem
            } else {
                format!("{prefix}/{stem}")
            };
            let url_path = if recipe.is_menu() {
                format!("menu/{sub}.html")
            } else {
                format!("recipe/{sub}.html")
            };
            let tags = recipe.tags();
            let ingredients = match crate::util::parse_recipe_from_entry(recipe, 1.0) {
                Ok(parsed) => parsed
                    .group_ingredients(crate::util::PARSER.converter())
                    .into_iter()
                    .map(|e| e.ingredient.display_name().to_string())
                    .collect(),
                Err(_) => Vec::new(),
            };
            out.push(SearchEntry {
                title: name.clone(),
                path: url_path,
                tags,
                ingredients,
            });
        } else {
            let sub = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}/{name}")
            };
            collect(child, sub, out);
        }
    }
}
