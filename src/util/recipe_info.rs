//! What a menu consumer needs to know about a recipe a menu references, beyond
//! the `scale` that `cooklang_find::Menu::resolve_scales` already computes: its
//! default servings (the menu page links by servings) and the recipes it
//! references in turn (the shopping list's `included_references`).

use camino::Utf8Path;

/// Information about a referenced recipe.
#[derive(Default)]
pub struct RecipeInfo {
    /// Recipes this one references in turn. Only the shopping list's
    /// `add_menu` reads this, and that lives behind the `server` feature,
    /// so it is genuinely dead code in a `--no-default-features` build.
    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    pub sub_refs: Vec<String>,
    /// Numeric `servings` metadata. The cooklang API only exposes this as
    /// `u32`, so fractional defaults like `servings: 1.5` appear as `None`.
    pub default_servings: Option<u32>,
    /// The reference names a `.menu` rather than a recipe: a whole menu used
    /// as one meal of another, such as `@./Brunches/Sunday.menu{}`.
    pub is_menu: bool,
}

pub fn resolve_recipe_info(base_path: &Utf8Path, recipe_path: &str) -> anyhow::Result<RecipeInfo> {
    let entry = crate::util::get_recipe(base_path, recipe_path)?;
    let is_menu = entry.is_menu();
    let recipe = crate::util::parse_recipe_from_entry(&entry, 1.0)?;

    let mut sub_refs = Vec::new();
    for ingredient in &recipe.ingredients {
        if let Some(ref recipe_ref) = ingredient.reference {
            let path = if recipe_ref.components.is_empty() {
                recipe_ref.name.clone()
            } else {
                format!("{}/{}", recipe_ref.components.join("/"), recipe_ref.name)
            };
            sub_refs.push(path);
        }
    }
    let default_servings = recipe.metadata.servings().and_then(|s| s.as_number());

    Ok(RecipeInfo {
        sub_refs,
        default_servings,
        is_menu,
    })
}

/// Read a referenced recipe's metadata, degrading to defaults (and a warning)
/// when the file cannot be found or parsed.
pub fn ref_info_or_default(base_path: &Utf8Path, lookup: &str, ref_display: &str) -> RecipeInfo {
    match resolve_recipe_info(base_path, lookup) {
        Ok(info) => info,
        Err(e) => {
            tracing::warn!(
                "Could not resolve referenced recipe '{}': {}",
                ref_display,
                e
            );
            RecipeInfo::default()
        }
    }
}
