//! What a menu consumer needs to know about a recipe a menu references, beyond
//! the `scale` that `cooklang_find::Menu::resolve_scales` already computes: its
//! default servings (the menu page links by servings) and the recipes it
//! references in turn (the shopping list's `included_references`).
//!
//! A recipe's own references (`@./Sauce{2%servings}`) are worked out here too,
//! by [`reference_factor`], so that its page and the shopping list open them at
//! the size the recipe asks for.

use camino::Utf8Path;
use cooklang::model::Ingredient;
use cooklang::quantity::{GroupedQuantity, Quantity};

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
    /// `yield` metadata written as `500%g`: the amount and its unit.
    pub yield_amount: Option<(f64, String)>,
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
    let yield_amount = recipe
        .metadata
        .get("yield")
        .and_then(|v| v.as_str())
        .and_then(parse_yield);

    Ok(RecipeInfo {
        sub_refs,
        default_servings,
        yield_amount,
        is_menu,
    })
}

/// `500%g` as `(500.0, "g")`; `None` for any other spelling.
fn parse_yield(text: &str) -> Option<(f64, String)> {
    let (value, unit) = text.split_once('%')?;
    let value = value.trim().parse::<f64>().ok()?;
    let unit = unit.trim();
    (value.is_finite() && value > 0.0 && !unit.is_empty()).then(|| (value, unit.to_string()))
}

/// How many times its own size a referenced recipe is made, for a reference
/// written with `quantity` in a recipe already scaled by `parent_scale` —
/// the quantity has been scaled with it, so it is the amount actually asked
/// for.
///
/// - no quantity: the recipe follows its parent, so `parent_scale`
/// - a plain number: that many times
/// - servings: that many servings of the recipe's own `servings` (taken as a
///   multiplier when it declares none, as menus do)
/// - the unit of the recipe's `yield`: that amount of what it yields
///
/// `None` when the amount cannot be turned into a factor — another unit, a
/// range or text — rather than a guess such as ×200 for `{200%g}`.
pub fn reference_factor(
    quantity: Option<&Quantity>,
    parent_scale: f64,
    info: &RecipeInfo,
) -> Option<f64> {
    let factor = match quantity {
        None => parent_scale,
        Some(quantity) => {
            let cooklang::Value::Number(number) = quantity.value() else {
                return None;
            };
            let target = number.value();
            match quantity.unit() {
                None => target,
                Some(unit)
                    if unit.eq_ignore_ascii_case("servings")
                        || unit.eq_ignore_ascii_case("serving") =>
                {
                    match info.default_servings.filter(|&n| n > 0) {
                        Some(base) => target / f64::from(base),
                        None => target,
                    }
                }
                Some(unit) => {
                    let (base, _) = info
                        .yield_amount
                        .as_ref()
                        .filter(|(_, yield_unit)| yield_unit.eq_ignore_ascii_case(unit))?;
                    target / base
                }
            }
        }
    };
    (factor.is_finite() && factor > 0.0).then_some(factor)
}

/// The one quantity a recipe asks for of something it uses several times,
/// such as a sauce added in two steps: the uses added up when they share a
/// unit, otherwise the first use's own. `None` when no use gives one.
pub fn combined_quantity(uses: &[&Ingredient]) -> Option<Quantity> {
    let mut total = GroupedQuantity::empty();
    for quantity in uses.iter().filter_map(|i| i.quantity.as_ref()) {
        total.add(quantity, crate::util::PARSER.converter());
    }
    match crate::util::format::quantity::ordered_components(&total).as_slice() {
        [] => None,
        [one] => Some((*one).clone()),
        _ => uses.first().and_then(|i| i.quantity.clone()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn quantity_of(recipe: &str) -> Option<Quantity> {
        let (recipe, _) = crate::util::PARSER.parse(recipe).into_result().unwrap();
        let uses: Vec<&Ingredient> = recipe.ingredients.iter().collect();
        combined_quantity(&uses)
    }

    fn sauce() -> RecipeInfo {
        RecipeInfo {
            default_servings: Some(4),
            yield_amount: Some((500.0, "ml".to_string())),
            ..RecipeInfo::default()
        }
    }

    #[test]
    fn a_sub_recipe_used_twice_is_made_for_both_uses() {
        let quantity = quantity_of("Add @./Sauce{1%servings}, then @./Sauce{1%servings}.");
        assert_eq!(
            reference_factor(quantity.as_ref(), 1.0, &sauce()),
            Some(0.5)
        );
    }

    #[test]
    fn uses_in_two_units_go_by_the_first() {
        let quantity = quantity_of("Add @./Sauce{250%ml}, then @./Sauce{1%servings}.");
        assert_eq!(
            reference_factor(quantity.as_ref(), 1.0, &sauce()),
            Some(0.5)
        );
    }

    #[test]
    fn units_are_matched_whatever_their_case() {
        let quantity = quantity_of("Add @./Sauce{1000%ML}.");
        assert_eq!(
            reference_factor(quantity.as_ref(), 1.0, &sauce()),
            Some(2.0)
        );
        let quantity = quantity_of("Add @./Sauce{2%Servings}.");
        assert_eq!(
            reference_factor(quantity.as_ref(), 1.0, &sauce()),
            Some(0.5)
        );
    }

    #[test]
    fn what_cannot_be_a_factor_is_none() {
        for text in ["@./Sauce{200%g}", "@./Sauce{some}", "@./Sauce{0}"] {
            let quantity = quantity_of(text);
            assert_eq!(
                reference_factor(quantity.as_ref(), 1.0, &sauce()),
                None,
                "{text}"
            );
        }
        // No yield to measure millilitres against.
        let quantity = quantity_of("@./Sauce{250%ml}");
        assert_eq!(
            reference_factor(quantity.as_ref(), 1.0, &RecipeInfo::default()),
            None
        );
    }

    #[test]
    fn yields_are_read_as_amount_and_unit() {
        assert_eq!(parse_yield("500%ml"), Some((500.0, "ml".to_string())));
        assert_eq!(parse_yield(" 6 % balls "), Some((6.0, "balls".to_string())));
        assert_eq!(parse_yield("500 ml"), None);
        assert_eq!(parse_yield("0%g"), None);
        assert_eq!(parse_yield("5%"), None);
    }
}
