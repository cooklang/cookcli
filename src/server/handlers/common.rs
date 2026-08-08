use axum::{http::StatusCode, Json};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};

pub fn json_error(msg: impl std::fmt::Display) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": msg.to_string() }))
}

pub fn check_path(p: &str) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let path = Utf8Path::new(p);
    if !path
        .components()
        .all(|c| matches!(c, Utf8Component::Normal(_)))
    {
        tracing::error!("Invalid path: {p}");
        return Err((
            StatusCode::BAD_REQUEST,
            json_error(format!("Invalid path: {p}")),
        ));
    }
    Ok(())
}

/// Information about a referenced recipe needed to convert a menu's
/// `{target%unit}` into a scale multiplier for that recipe.
#[derive(Default)]
pub(crate) struct RecipeInfo {
    pub(crate) sub_refs: Vec<String>,
    /// Numeric `servings` metadata. The cooklang API only exposes this as
    /// `u32`, so fractional defaults like `servings: 1.5` appear as `None`
    /// and fall back to raw-multiplier mode.
    pub(crate) default_servings: Option<u32>,
    /// Parsed `yield` metadata (value, unit) if present and well-formed,
    /// e.g. `yield: 500%ml` → `Some((500.0, "ml"))`.
    pub(crate) default_yield: Option<(f64, String)>,
}

/// Parse the `yield` metadata format `"VALUE%UNIT"` (e.g. `"500%ml"`) into
/// its numeric value and unit.
pub(crate) fn parse_yield(s: &str) -> Option<(f64, String)> {
    let (value, unit) = s.split_once('%')?;
    let value = value.trim().parse::<f64>().ok()?;
    let unit = unit.trim();
    if unit.is_empty() {
        return None;
    }
    Some((value, unit.to_string()))
}

pub(crate) fn resolve_recipe_info(
    base_path: &Utf8PathBuf,
    recipe_path: &str,
) -> anyhow::Result<RecipeInfo> {
    let entry = crate::util::get_recipe(base_path, recipe_path)?;
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

/// Convert a menu reference's `{target%unit}` into a scale multiplier for the
/// referenced recipe, per the Cooklang spec's "Scaling Referenced Recipes"
/// (conventions.md). The unit decides how `target` is interpreted:
///
/// - no unit — raw multiplier (`{2}` = ×2).
/// - `servings` / `serving` — target servings; factor = target / default_servings.
/// - any other unit — target yield; factor = target / default_yield_value, and
///   only when the units match, since no conversion is attempted.
///
/// Every fallback degrades to treating `target` as a raw multiplier, which is
/// what the notation meant before units were interpreted at all.
///
/// `quantity` must be the quantity as authored, i.e. taken from a menu parsed
/// at scale 1.0. Do not pass an already-scaled quantity: the parser normalises
/// units while scaling (750 ml × 3 becomes 2.25 l), which would silently break
/// the yield unit comparison below. Multiply the returned factor by the menu
/// scale instead.
pub(crate) fn reference_scale_factor(
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
