use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use cooklang_find::RecipeTree;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};

static DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((\d{4}-\d{2}-\d{2})\)").unwrap());
static TIME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((\d{2}:\d{2})\)").unwrap());
static MEAL_HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*\(\d{2}:\d{2}\)\s*").unwrap());

#[derive(Serialize)]
pub struct MenuListItem {
    pub name: String,
    pub path: String,
}

fn json_error(msg: impl std::fmt::Display) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": msg.to_string() }))
}

fn collect_menus(tree: &RecipeTree, base_path: &camino::Utf8Path, result: &mut Vec<MenuListItem>) {
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
        scale: Option<f64>,
    },
    #[serde(rename = "ingredient")]
    Ingredient {
        name: String,
        quantity: Option<String>,
        unit: Option<String>,
    },
}

fn check_path(p: &str) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
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

/// Extract a date in YYYY-MM-DD format from a section name.
/// Matches patterns like "Day 1 (2026-03-04)".
fn extract_date(name: &str) -> Option<String> {
    DATE_RE.captures(name).map(|caps| caps[1].to_string())
}

/// Extract a time in HH:MM format from a meal type header.
/// Matches patterns like "Breakfast (08:30):".
fn extract_time(header: &str) -> Option<String> {
    TIME_RE.captures(header).map(|caps| caps[1].to_string())
}

/// Extract the meal type name from a header string.
/// Strips the trailing colon and any time in parentheses.
/// "Breakfast (08:30):" -> "Breakfast"
/// "Dinner:" -> "Dinner"
fn extract_meal_type(header: &str) -> String {
    // Remove trailing colon (and whitespace around it)
    let stripped = header.trim().trim_end_matches(':').trim();
    // Remove time in parentheses
    MEAL_HEADER_RE.replace_all(stripped, "").trim().to_string()
}

/// Check if a text line is a meal type header (ends with ":" possibly with whitespace).
fn is_meal_header(text: &str) -> bool {
    let trimmed = text.trim();
    // Must end with ':'
    if !trimmed.ends_with(':') {
        return false;
    }
    // Must have some content before the colon
    let before_colon = trimmed.trim_end_matches(':').trim();
    !before_colon.is_empty()
}

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

    let recipe = crate::util::parse_recipe_from_entry(&entry, scale).map_err(|e| {
        tracing::error!("Failed to parse menu: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error(format!("Failed to parse menu: {e}")),
        )
    })?;

    // Build metadata as a JSON object
    let metadata = if recipe.metadata.map.is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        let mut map = serde_json::Map::new();
        for (key, value) in recipe.metadata.map.iter() {
            if let Some(key_str) = key.as_str() {
                let val = if let Some(s) = value.as_str() {
                    serde_json::Value::String(s.to_string())
                } else if let Some(n) = value.as_i64() {
                    serde_json::Value::String(n.to_string())
                } else if let Some(n) = value.as_f64() {
                    serde_json::Value::String(crate::util::format::format_number(n))
                } else {
                    serde_json::Value::Null
                };
                map.insert(key_str.to_string(), val);
            }
        }
        serde_json::Value::Object(map)
    };

    // Extract the menu name
    let menu_name = recipe
        .metadata
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            path.split('/')
                .next_back()
                .unwrap_or(&path)
                .replace(".menu", "")
        });

    // Parse sections following the same approach as menu_page_handler in ui.rs
    let mut sections = Vec::new();

    for section in &recipe.sections {
        let section_name = section.name.clone();
        let date = section_name.as_deref().and_then(extract_date);

        // Collect all items in this section as a flat list of (is_text, item) tuples
        // We first build "lines" similar to ui.rs, then group by meal headers
        let mut lines: Vec<Vec<LineItem>> = Vec::new();

        for content in &section.content {
            use cooklang::Content;
            if let Content::Step(step) = content {
                let mut step_items: Vec<LineItem> = Vec::new();
                let mut current_text = String::new();

                for item in &step.items {
                    use cooklang::Item;
                    match item {
                        Item::Text { value } => {
                            if value == "-" {
                                // Bullet marker - flush current line
                                if !current_text.is_empty() {
                                    step_items.push(LineItem::Text(current_text.clone()));
                                    current_text.clear();
                                }
                                if !step_items.is_empty() {
                                    lines.push(step_items.clone());
                                    step_items.clear();
                                }
                            } else {
                                let parts: Vec<&str> = value.split('\n').collect();
                                for (i, part) in parts.iter().enumerate() {
                                    if i > 0 {
                                        if !current_text.is_empty() {
                                            step_items.push(LineItem::Text(current_text.clone()));
                                            current_text.clear();
                                        }
                                        if !step_items.is_empty() {
                                            lines.push(step_items.clone());
                                            step_items.clear();
                                        }
                                    }
                                    if !part.is_empty() {
                                        current_text.push_str(part);
                                    }
                                }
                            }
                        }
                        Item::Ingredient { index } => {
                            if !current_text.is_empty() {
                                step_items.push(LineItem::Text(current_text.clone()));
                                current_text.clear();
                            }

                            if let Some(ing) = recipe.ingredients.get(*index) {
                                if let Some(ref recipe_ref) = ing.reference {
                                    let recipe_scale =
                                        ing.quantity.as_ref().and_then(|q| match q.value() {
                                            cooklang::Value::Number(n) => Some(n.value()),
                                            _ => None,
                                        });
                                    let final_scale = recipe_scale.map(|s| s * scale);

                                    let name = if recipe_ref.components.is_empty() {
                                        recipe_ref.name.clone()
                                    } else {
                                        format!(
                                            "{}/{}",
                                            recipe_ref.components.join("/"),
                                            recipe_ref.name
                                        )
                                    };

                                    // Build the .cook path for the reference
                                    let ref_path = format!("{}.cook", name);

                                    step_items.push(LineItem::RecipeRef {
                                        name,
                                        path: Some(ref_path),
                                        scale: final_scale,
                                    });
                                } else {
                                    let quantity = ing.quantity.as_ref().and_then(|q| {
                                        crate::util::format::format_quantity(q.value())
                                    });
                                    let unit = ing
                                        .quantity
                                        .as_ref()
                                        .and_then(|q| q.unit().as_ref().map(|u| u.to_string()));

                                    step_items.push(LineItem::Ingredient {
                                        name: ing.name.to_string(),
                                        quantity,
                                        unit,
                                    });
                                }
                            }
                        }
                        _ => {} // Ignore other items in menu files
                    }
                }

                if !current_text.is_empty() {
                    step_items.push(LineItem::Text(current_text));
                }
                if !step_items.is_empty() {
                    lines.push(step_items);
                }
            }
        }

        // Now group lines into meals.
        // A line that is purely a text item ending with ":" is a meal header.
        // Everything after it until the next header belongs to that meal.
        // Items before any header go into a default "Items" meal.
        let mut meals: Vec<MenuMeal> = Vec::new();
        let mut current_meal_type = String::from("Items");
        let mut current_meal_time: Option<String> = None;
        let mut current_items: Vec<MenuMealItem> = Vec::new();

        for line in &lines {
            // Check if this line is a meal header: single text item that is a meal header
            if line.len() == 1 {
                if let LineItem::Text(ref text) = line[0] {
                    if is_meal_header(text) {
                        // Flush previous meal if it has items
                        if !current_items.is_empty() {
                            meals.push(MenuMeal {
                                meal_type: current_meal_type.clone(),
                                time: current_meal_time.take(),
                                items: std::mem::take(&mut current_items),
                            });
                        }
                        current_meal_time = extract_time(text);
                        current_meal_type = extract_meal_type(text);
                        continue;
                    }
                }
            }

            // Not a meal header - add items to current meal
            for item in line {
                match item {
                    LineItem::Text(_) => {
                        // Plain text lines in menus are typically just whitespace or
                        // decorative; skip them to keep the API output clean.
                    }
                    LineItem::RecipeRef { name, path, scale } => {
                        current_items.push(MenuMealItem::RecipeReference {
                            name: name.clone(),
                            path: path.clone(),
                            scale: *scale,
                        });
                    }
                    LineItem::Ingredient {
                        name,
                        quantity,
                        unit,
                    } => {
                        current_items.push(MenuMealItem::Ingredient {
                            name: name.clone(),
                            quantity: quantity.clone(),
                            unit: unit.clone(),
                        });
                    }
                }
            }
        }

        // Flush the last meal
        if !current_items.is_empty() {
            meals.push(MenuMeal {
                meal_type: current_meal_type,
                time: current_meal_time,
                items: current_items,
            });
        }

        sections.push(MenuApiSection {
            name: section_name,
            date,
            meals,
        });
    }

    Ok(Json(MenuResponse {
        name: menu_name,
        path,
        metadata,
        sections,
    }))
}

/// Internal representation of a line item while parsing.
#[derive(Clone)]
enum LineItem {
    Text(String),
    RecipeRef {
        name: String,
        path: Option<String>,
        scale: Option<f64>,
    },
    Ingredient {
        name: String,
        quantity: Option<String>,
        unit: Option<String>,
    },
}
