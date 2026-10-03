//! Glue between `cooklang_find::Menu` and CookCLI's three menu consumers: the
//! JSON API, the HTML page (also the static export) and the shopping list.
//!
//! It lives in `util` rather than next to the server handlers because
//! `crate::server` is behind the `server` feature while `crate::web` is not.

use anyhow::{anyhow, Result};
use camino::Utf8Path;
use cooklang_find::{Menu, MenuItem, RecipeEntry};

/// Load a menu with every reference's `scale` resolved.
///
/// `menu_path` is the menu as named relative to `base_path`. Reference paths
/// from `Menu` are the authored ones, so a `@../Shared/Sauce` in `Plans/Week`
/// is still `../Shared/Sauce`; `cooklang-find` resolves paths against the
/// library root only. They are rewritten here to be root-relative, which is
/// what the rest of CookCLI expects (and what `resolve_scales` needs to find
/// the referenced recipe's `servings`/`yield`).
pub fn load_menu(
    entry: &RecipeEntry,
    menu_path: &Utf8Path,
    base_path: &Utf8Path,
    scale: f64,
) -> Result<Menu> {
    let mut menu = parse_menu(entry, menu_path)?;
    menu.resolve_scales(&[base_path], scale);
    Ok(menu)
}

/// [`load_menu`] without resolving scales, for the page, which needs the
/// menu's own `servings` to pick the scale first.
pub fn parse_menu(entry: &RecipeEntry, menu_path: &Utf8Path) -> Result<Menu> {
    let mut menu = entry
        .menu()
        .ok_or_else(|| anyhow!("not a menu file"))?
        .map_err(|e| anyhow!("Failed to read menu: {e}"))?;

    let menu_dir = menu_path.parent().unwrap_or(Utf8Path::new(""));
    for item in menu_items_mut(&mut menu) {
        if let MenuItem::RecipeReference { path, unit, .. } = item {
            if path.starts_with("../") {
                match cookcli_core::resolve_reference(menu_dir, path) {
                    Some(resolved) => *path = resolved.to_string(),
                    // Climbs out of the recipe directory. The authored path is
                    // kept so callers can tell (`is_outside_reference`), and
                    // the unit is dropped so `resolve_scales` does not join
                    // the path onto the base dir and read a file outside it:
                    // the target is then used as a raw multiplier.
                    None => *unit = None,
                }
            }
        }
    }

    Ok(menu)
}

/// True for a reference [`load_menu`] could not keep inside the recipe
/// directory (`@../../etc/passwd{}`).
#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub fn is_outside_reference(path: &str) -> bool {
    path.starts_with("../")
}

fn menu_items_mut(menu: &mut Menu) -> impl Iterator<Item = &mut MenuItem> {
    menu.sections
        .iter_mut()
        .flat_map(|section| &mut section.meals)
        .flat_map(|meal| &mut meal.items)
}

/// Quantity and unit of a loose ingredient, scaled by `scale` and formatted
/// the way the menu API and page have always shown them.
///
/// `cooklang-find` hands back the authored text (`1 1/2`, `1-2`, `some`,
/// `=100`), and scaling that is cooklang's job (fractions, ranges, fixed
/// `=` quantities, text values). So the authored pieces are put back into a
/// one-ingredient recipe, parsed and scaled by `cooklang`, and formatted as
/// before. If the fragment somehow does not parse the authored text is
/// returned unscaled.
pub fn scaled_quantity(
    quantity: Option<&str>,
    unit: Option<&str>,
    scale: f64,
) -> (Option<String>, Option<String>) {
    if quantity.is_none() && unit.is_none() {
        return (None, None);
    }
    let fragment = match unit {
        Some(unit) => format!("@x{{{}%{}}}", quantity.unwrap_or(""), unit),
        None => format!("@x{{{}}}", quantity.unwrap_or("")),
    };
    let scaled = crate::util::PARSER
        .parse(&fragment)
        .into_output()
        .and_then(|mut recipe| {
            recipe.scale(scale, crate::util::PARSER.converter());
            let ingredient = recipe.ingredients.into_iter().next()?;
            let quantity = ingredient.quantity?;
            Some((
                crate::util::format::number::format_quantity(quantity.value()),
                quantity.unit().as_ref().map(|u| u.to_string()),
            ))
        });
    scaled.unwrap_or_else(|| (quantity.map(str::to_string), unit.map(str::to_string)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_numbers_scale() {
        assert_eq!(
            scaled_quantity(Some("2"), Some("cup"), 3.0),
            (Some("6".to_string()), Some("cup".to_string()))
        );
    }

    #[test]
    fn fractions_and_mixed_numbers_scale() {
        assert_eq!(
            scaled_quantity(Some("1/2"), Some("tbsp"), 3.0).0.as_deref(),
            Some("1.5")
        );
        assert_eq!(
            scaled_quantity(Some("1 1/2"), Some("cup"), 2.0)
                .0
                .as_deref(),
            Some("3")
        );
    }

    #[test]
    fn fixed_and_text_quantities_do_not_scale() {
        assert_eq!(
            scaled_quantity(Some("=100"), Some("g"), 3.0).0.as_deref(),
            Some("100")
        );
        assert_eq!(
            scaled_quantity(Some("some"), None, 3.0).0.as_deref(),
            Some("some")
        );
    }

    #[test]
    fn nothing_authored_is_nothing() {
        assert_eq!(scaled_quantity(None, None, 3.0), (None, None));
    }
}
