use super::common::{check_path, json_error, normalize_tags};
use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang_find::{Menu, MenuItem, RecipeTree};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Serialize)]
pub struct MenuListItem {
    pub name: String,
    pub path: String,
}

pub fn collect_menus(
    tree: &RecipeTree,
    base_path: &camino::Utf8Path,
    result: &mut Vec<MenuListItem>,
) {
    if let Some(ref entry) = tree.recipe {
        if entry.is_menu() {
            if let Some(full_path) = entry.path() {
                let relative = full_path
                    .strip_prefix(base_path)
                    .unwrap_or(full_path.as_ref());
                let name = entry.name().clone().unwrap_or_else(|| relative.to_string());
                result.push(MenuListItem {
                    name,
                    path: relative.to_string(),
                });
            }
        }
    }

    for child in tree.children.values() {
        collect_menus(child, base_path, result);
    }
}

pub async fn list_menus(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<MenuListItem>>, (StatusCode, Json<serde_json::Value>)> {
    let tree = cooklang_find::build_tree(&state.base_path).map_err(|e| {
        tracing::error!("Failed to build recipe tree: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(&e))
    })?;

    let mut menus = Vec::new();
    collect_menus(&tree, &state.base_path, &mut menus);

    Ok(Json(menus))
}

// --- GET /api/menus/*path ---

#[derive(Deserialize)]
pub struct MenuQuery {
    scale: Option<f64>,
}

#[derive(Serialize)]
pub struct MenuResponse {
    pub name: String,
    pub path: String,
    pub metadata: serde_json::Value,
    pub sections: Vec<MenuApiSection>,
}

#[derive(Serialize)]
pub struct MenuApiSection {
    pub name: Option<String>,
    pub date: Option<String>,
    pub meals: Vec<MenuMeal>,
}

#[derive(Serialize)]
pub struct MenuMeal {
    #[serde(rename = "type")]
    pub meal_type: String,
    pub time: Option<String>,
    pub items: Vec<MenuMealItem>,
}

#[derive(Serialize)]
#[serde(tag = "kind")]
pub enum MenuMealItem {
    #[serde(rename = "recipe_reference")]
    RecipeReference {
        name: String,
        path: Option<String>,
        /// Multiplier for the referenced recipe. Always present: a reference
        /// with no `{...}` target is ×1 before the menu scale is applied.
        scale: f64,
        /// The reference is another menu, used as a meal of this one; `path`
        /// then ends in `.menu`. Only sent when true, so a response without
        /// menus reads as it always has.
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        menu: bool,
    },
    #[serde(rename = "ingredient")]
    Ingredient {
        name: String,
        quantity: Option<String>,
        unit: Option<String>,
    },
}

/// Meal type reported for items before a menu's first meal header.
const DEFAULT_MEAL_TYPE: &str = "Items";

pub async fn get_menu(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<MenuQuery>,
) -> Result<Json<MenuResponse>, (StatusCode, Json<serde_json::Value>)> {
    check_path(&path)?;

    let scale = query.scale.unwrap_or(1.0);
    let recipe_path = Utf8PathBuf::from(&path);

    let entry = cooklang_find::get_recipe(vec![&state.base_path], &recipe_path).map_err(|e| {
        tracing::error!("Menu not found: {path}");
        (
            StatusCode::NOT_FOUND,
            json_error(format!("Menu not found: {path}: {e}")),
        )
    })?;

    if !entry.is_menu() {
        return Err((
            StatusCode::BAD_REQUEST,
            json_error(format!("Path is not a menu file: {path}")),
        ));
    }

    let menu = crate::util::menu::load_menu(&entry, &recipe_path, &state.base_path, scale)
        .map_err(|e| {
            tracing::error!("Failed to parse menu: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!("Failed to parse menu: {e}")),
            )
        })?;

    Ok(Json(menu_response(&menu, path, scale, &state.base_path)))
}

/// Shape a [`Menu`] into the API response.
///
/// The response keeps the semantics it had before `cooklang-find` parsed
/// menus: a missing meal type is `"Items"`, text and notes are dropped,
/// reference names and paths are `./Dir/Name` and `./Dir/Name.cook` (or
/// `./Dir/Name.menu` for a menu used as a meal), and loose quantities are
/// scaled and formatted.
fn menu_response(menu: &Menu, path: String, scale: f64, base_path: &Utf8Path) -> MenuResponse {
    // Whether each referenced path is a menu, looked up once per path.
    let mut is_menu: HashMap<String, bool> = HashMap::new();
    let sections = menu
        .sections
        .iter()
        .map(|section| MenuApiSection {
            name: section.name.clone(),
            date: section.date.clone(),
            meals: section
                .meals
                .iter()
                .filter_map(|meal| {
                    let items: Vec<_> = meal
                        .items
                        .iter()
                        .filter_map(|item| api_item(item, scale, base_path, &mut is_menu))
                        .collect();
                    // A meal of only notes or text has nothing to report.
                    (!items.is_empty()).then(|| MenuMeal {
                        meal_type: meal
                            .meal_type
                            .clone()
                            .unwrap_or_else(|| DEFAULT_MEAL_TYPE.to_string()),
                        time: meal.time.clone(),
                        items,
                    })
                })
                .collect(),
        })
        .collect();

    MenuResponse {
        name: menu.name.clone(),
        path,
        metadata: api_metadata(menu, scale),
        sections,
    }
}

fn api_item(
    item: &MenuItem,
    scale: f64,
    base_path: &Utf8Path,
    is_menu: &mut HashMap<String, bool>,
) -> Option<MenuMealItem> {
    match item {
        MenuItem::RecipeReference {
            path,
            scale: factor,
            ..
        } => {
            // A reference out of the recipe directory is not looked up.
            let menu = !crate::util::menu::is_outside_reference(path)
                && *is_menu.entry(path.clone()).or_insert_with(|| {
                    crate::util::get_recipe(base_path, path).is_ok_and(|entry| entry.is_menu())
                });
            // The file the reference names: a recipe's `.cook`, or the
            // `.menu` of a menu used as a meal of this one.
            let file = if menu {
                format!("./{}.menu", path.trim_end_matches(".menu"))
            } else {
                format!("./{path}.cook")
            };
            Some(MenuMealItem::RecipeReference {
                name: format!("./{path}"),
                path: Some(file),
                scale: factor.unwrap_or(1.0),
                menu,
            })
        }
        MenuItem::Ingredient {
            name,
            quantity,
            unit,
        } => {
            let (quantity, unit) =
                crate::util::menu::scaled_quantity(quantity.as_deref(), unit.as_deref(), scale);
            Some(MenuMealItem::Ingredient {
                name: name.clone(),
                quantity,
                unit,
            })
        }
        // Text, notes, line breaks and anything added later are not part of
        // the API.
        _ => None,
    }
}

/// The menu's frontmatter as a JSON object of strings (`tags` stays an array),
/// with a numeric `servings` scaled like the recipe parser scales it.
fn api_metadata(menu: &Menu, scale: f64) -> serde_json::Value {
    // `Metadata` has no way to iterate its entries other than serialising it.
    let raw = serde_json::to_value(&menu.metadata).unwrap_or_default();
    let mut map = serde_json::Map::new();
    for (key, value) in raw.as_object().into_iter().flatten() {
        let val = if key == "tags" {
            // `tags` keeps its YAML shape so that a sequence survives:
            // the stringification below would report `tags: [a, b]` as
            // null. `normalize_tags` then gives the comma-separated
            // spelling the same array shape.
            value.clone()
        } else if let Some(s) = value.as_str() {
            serde_json::Value::String(s.to_string())
        } else if let Some(n) = value.as_i64() {
            serde_json::Value::String(n.to_string())
        } else if let Some(n) = value.as_f64() {
            serde_json::Value::String(crate::util::format::number::format_number(n))
        } else {
            serde_json::Value::Null
        };
        map.insert(key.clone(), val);
    }
    if let Some(base) = menu.metadata.get("servings").and_then(|v| {
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
    }) {
        let scaled = (base as f64 * scale).round() as u64;
        map.insert(
            "servings".to_string(),
            serde_json::Value::String(scaled.to_string()),
        );
    }
    let mut metadata = serde_json::Value::Object(map);
    normalize_tags(&mut metadata);
    metadata
}
