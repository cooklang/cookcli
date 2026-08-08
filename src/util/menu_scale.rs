//! Turning a menu's `@./Some Recipe{target%unit}` reference into a scale
//! multiplier for the recipe it points at.
//!
//! Three callers need this and must agree: the menu JSON API, the shopping
//! list's `add_menu`, and the HTML menu page (which is also the static export).
//! It lives in `util` rather than next to the server handlers because
//! `crate::server` is behind the `server` feature while `crate::web` is not.

use camino::Utf8Path;

/// Information about a referenced recipe needed to interpret a `{target%unit}`
/// pointed at it.
#[derive(Default)]
pub struct RecipeInfo {
    /// Recipes this one references in turn. Only the shopping list's
    /// `add_menu` reads this, and that lives behind the `server` feature,
    /// so it is genuinely dead code in a `--no-default-features` build.
    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    pub sub_refs: Vec<String>,
    /// Numeric `servings` metadata. The cooklang API only exposes this as
    /// `u32`, so fractional defaults like `servings: 1.5` appear as `None`
    /// and fall back to raw-multiplier mode.
    pub default_servings: Option<u32>,
    /// Parsed `yield` metadata (value, unit) if present and well-formed,
    /// e.g. `yield: 500%ml` → `Some((500.0, "ml"))`.
    pub default_yield: Option<(f64, String)>,
}

/// Parse the `yield` metadata format `"VALUE%UNIT"` (e.g. `"500%ml"`) into
/// its numeric value and unit.
pub fn parse_yield(s: &str) -> Option<(f64, String)> {
    let (value, unit) = s.split_once('%')?;
    let value = value.trim().parse::<f64>().ok()?;
    let unit = unit.trim();
    if unit.is_empty() {
        return None;
    }
    Some((value, unit.to_string()))
}

pub fn resolve_recipe_info(base_path: &Utf8Path, recipe_path: &str) -> anyhow::Result<RecipeInfo> {
    let entry = crate::util::get_recipe(&base_path.to_path_buf(), recipe_path)?;
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
    let default_yield = recipe
        .metadata
        .get("yield")
        .and_then(|v| v.as_str())
        .and_then(parse_yield);

    Ok(RecipeInfo {
        sub_refs,
        default_servings,
        default_yield,
    })
}

/// Read a referenced recipe's `servings` / `yield` metadata, degrading to
/// defaults (and a warning) when the file cannot be found or parsed.
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

/// Convert a menu reference's `{target%unit}` into a scale multiplier for the
/// referenced recipe, per the Cooklang spec's "Scaling Referenced Recipes"
/// (conventions.md). The unit decides how `target` is interpreted:
///
/// - no unit — raw multiplier (`{2}` = ×2).
/// - `servings` / `serving` — target servings; factor = target / default_servings.
/// - any other unit — target yield; factor = target / default_yield_value, and
///   only when the units match, since no conversion is attempted.
///
/// A reference with no quantity at all (`@foo{}`) is ×1. Every fallback
/// degrades to treating `target` as a raw multiplier, which is what the
/// notation meant before units were interpreted at all.
///
/// `quantity` must be the quantity as authored, i.e. taken from a menu parsed
/// at scale 1.0. Do not pass an already-scaled quantity: the parser normalises
/// units while scaling (750 ml × 3 becomes 2.25 l), which would silently break
/// the yield unit comparison below. Multiply the returned factor by the menu
/// scale instead.
pub fn reference_scale_factor(
    quantity: Option<&cooklang::Quantity>,
    info: &RecipeInfo,
    ref_display: &str,
) -> f64 {
    let Some(q) = quantity else {
        return 1.0;
    };

    let value = match q.value() {
        cooklang::Value::Number(n) => Some(n.value()),
        _ => None,
    };

    match (value, q.unit()) {
        // Non-numeric quantity (e.g. `{some%servings}`): no target to scale
        // against, so use identity.
        (None, _) => 1.0,
        (Some(v), None) => v,
        (Some(target), Some(unit))
            if unit.eq_ignore_ascii_case("servings") || unit.eq_ignore_ascii_case("serving") =>
        {
            match info.default_servings {
                Some(base) if base > 0 => target / base as f64,
                _ => {
                    tracing::warn!(
                        "Recipe '{}' has no numeric servings metadata; \
                         treating {} servings as a raw multiplier",
                        ref_display,
                        target
                    );
                    target
                }
            }
        }
        (Some(target), Some(unit)) => match &info.default_yield {
            Some((base, base_unit)) if base_unit.eq_ignore_ascii_case(unit) && *base > 0.0 => {
                target / base
            }
            Some((_, base_unit)) => {
                tracing::warn!(
                    "Recipe '{}' yield unit '{}' does not match \
                     reference unit '{}'; treating {} as a raw multiplier",
                    ref_display,
                    base_unit,
                    unit,
                    target
                );
                target
            }
            None => {
                tracing::warn!(
                    "Recipe '{}' has no yield metadata to scale \
                     against '{}'; treating {} as a raw multiplier",
                    ref_display,
                    unit,
                    target
                );
                target
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::parse_yield;

    #[test]
    fn parse_yield_basic() {
        assert_eq!(parse_yield("500%ml"), Some((500.0, "ml".to_string())));
    }

    #[test]
    fn parse_yield_decimal() {
        assert_eq!(parse_yield("1.5%l"), Some((1.5, "l".to_string())));
    }

    #[test]
    fn parse_yield_trims_whitespace() {
        assert_eq!(parse_yield(" 250 % g "), Some((250.0, "g".to_string())));
    }

    #[test]
    fn parse_yield_missing_unit() {
        assert_eq!(parse_yield("500%"), None);
        assert_eq!(parse_yield("500"), None);
    }

    #[test]
    fn parse_yield_missing_value() {
        assert_eq!(parse_yield("%ml"), None);
    }

    #[test]
    fn parse_yield_non_numeric_value() {
        assert_eq!(parse_yield("abc%ml"), None);
    }
}
