use crate::server::{
    shopping_list_store::{recipe_display_name, ShoppingListApiItem, ShoppingListStore},
    AppState,
};
use crate::util::{extract_ingredients, PARSER};
use anyhow::Context as _;
use axum::{extract::State, http::StatusCode, Json};
use camino::Utf8PathBuf;
use cooklang::ingredient_list::IngredientList;
use serde::Deserialize;
use serde_json;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct RecipeRequest {
    recipe: String,
    scale: Option<f64>,
    /// Which sub-recipe references to include. `None` = all.
    included_references: Option<Vec<String>>,
}

pub async fn shopping_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(payload): axum::extract::Json<Vec<RecipeRequest>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut list = IngredientList::new();
    let mut seen = BTreeMap::new();

    for entry in payload {
        let recipe_with_scale = if let Some(scale) = entry.scale {
            format!("{}:{}", entry.recipe, scale)
        } else {
            entry.recipe
        };

        extract_ingredients(
            &recipe_with_scale,
            &mut list,
            &mut seen,
            &state.base_path,
            PARSER.converter(),
            false,
            entry.included_references.as_deref(),
        )
        .map_err(|e| {
            tracing::error!("Error processing recipe: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;
    }

    // Load aisle configuration with lenient parsing
    let aisle_content = if let Some(path) = &state.aisle_path {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                tracing::debug!("Loaded aisle file from: {:?}", path);
                content
            }
            Err(e) => {
                tracing::warn!("Failed to read aisle file from {:?}: {}", path, e);
                String::new()
            }
        }
    } else {
        tracing::debug!("No aisle file configured");
        String::new()
    };

    // Parse aisle with lenient parsing
    let aisle_result = cooklang::aisle::parse_lenient(&aisle_content);

    if aisle_result.report().has_warnings() {
        for warning in aisle_result.report().warnings() {
            tracing::warn!("Aisle configuration warning: {}", warning);
        }
    }

    let aisle = aisle_result.output().cloned().unwrap_or_default();

    // Load pantry configuration
    let pantry_conf = if let Some(path) = &state.pantry_path {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                tracing::debug!("Loaded pantry file from: {:?}", path);
                // Parse pantry file using cooklang pantry parser
                let result = cooklang::pantry::parse_lenient(&content);

                if result.report().has_warnings() {
                    for warning in result.report().warnings() {
                        tracing::warn!("Pantry configuration warning: {}", warning);
                    }
                }

                result.output().cloned()
            }
            Err(e) => {
                tracing::warn!("Failed to read pantry file from {:?}: {}", path, e);
                None
            }
        }
    } else {
        tracing::debug!("No pantry file configured");
        None
    };

    // Use common names from aisle configuration
    list = list.use_common_names(&aisle, PARSER.converter());

    // Track pantry items that were found and subtracted (excluding zero quantities)
    let mut pantry_items = Vec::new();
    if let Some(ref pantry) = pantry_conf {
        // Check which items from the original list are in the pantry with non-zero quantity
        for (ingredient_name, _) in list.iter() {
            if let Some((_, pantry_item)) = pantry.find_ingredient(ingredient_name) {
                // Check if the pantry item has a non-zero quantity
                if let Some(qty_str) = pantry_item.quantity() {
                    // Special case for unlimited
                    if qty_str == "unlim" || qty_str == "unlimited" {
                        pantry_items.push(ingredient_name.clone());
                    } else if let Some((value, _)) = pantry_item.parsed_quantity() {
                        // Only include if quantity is greater than 0
                        if value > 0.0 {
                            pantry_items.push(ingredient_name.clone());
                        }
                    }
                } else {
                    // No quantity specified means we have it (backward compatibility)
                    pantry_items.push(ingredient_name.clone());
                }
            }
        }
    }

    // Apply pantry subtraction if pantry is available
    let final_list = if let Some(ref pantry) = pantry_conf {
        list.subtract_pantry(pantry, PARSER.converter())
    } else {
        list
    };

    let categories = final_list.categorize(&aisle);

    // Build the response
    let mut shopping_categories = Vec::new();

    for (category, items) in categories {
        let mut entries: Vec<(String, _)> = items.into_iter().collect();

        // The "other" bucket holds ingredients with no aisle category. They
        // arrive in recipe insertion order, which is unhelpful when scanning
        // a long list — sort alphabetically (case-insensitive) so shoppers
        // can find items predictably.
        if category == "other" {
            entries.sort_by(|(a, _), (b, _)| a.to_lowercase().cmp(&b.to_lowercase()));
        }

        let mut shopping_items = Vec::new();
        for (name, qty) in entries {
            let item_json = serde_json::json!({
                "name": name,
                "quantities": qty.into_vec()
            });
            shopping_items.push(item_json);
        }

        if !shopping_items.is_empty() {
            shopping_categories.push(serde_json::json!({
                "category": category,
                "items": shopping_items
            }));
        }
    }

    // Load checked state
    let store = ShoppingListStore::new(&state.base_path);
    let checked = store.checked_set().unwrap_or_default();

    let json_value = serde_json::json!({
        "categories": shopping_categories,
        "pantry_items": pantry_items,
        "checked": checked.into_iter().collect::<Vec<_>>()
    });
    Ok(Json(json_value))
}

pub async fn get_shopping_list_items(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ShoppingListApiItem>>, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    let items = store.load().map_err(|e| {
        tracing::error!("Failed to load shopping list: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(items))
}

#[derive(Debug, Deserialize)]
pub struct AddItemRequest {
    pub path: String,
    pub scale: f64,
    /// Which sub-recipe references to include. `None` = all (menus, backward compat).
    pub included_references: Option<Vec<String>>,
}

pub async fn add_to_shopping_list(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddItemRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    // `name` is derived from `path` on load — any client-supplied display
    // name would be silently discarded, so it's not accepted here.
    let item = ShoppingListApiItem {
        name: recipe_display_name(&payload.path),
        path: payload.path,
        scale: payload.scale,
        included_references: payload.included_references,
        recipes: None,
    };

    store.add(item).map_err(|e| {
        tracing::error!("Failed to add to shopping list: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct RemoveItemRequest {
    pub path: String,
}

pub async fn remove_from_shopping_list(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RemoveItemRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    store.remove(&payload.path).map_err(|e| {
        tracing::error!("Failed to remove from shopping list: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    // Compact the checked log now that one recipe is gone: stale checks
    // (ingredients no longer referenced by any remaining recipe) can drop.
    // Best-effort — a failure here must not break the remove itself.
    // Serialize against concurrent check/uncheck/compact.
    let _guard = state.checked_log_lock.lock().await;
    match aggregate_current_ingredient_names(&state) {
        Ok(names) => {
            if let Err(e) = store.compact(names) {
                tracing::warn!("Failed to compact checked log after remove: {:?}", e);
            }
        }
        Err(e) => tracing::warn!(
            "Skipping compact after remove — aggregation failed: {:?}",
            e
        ),
    }

    Ok(StatusCode::OK)
}

pub async fn clear_shopping_list(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // Acquire the checked-log lock so a concurrent check/uncheck can't
    // recreate `.shopping-checked` between our remove_file and the caller's
    // view of a cleared list.
    let _guard = state.checked_log_lock.lock().await;
    let store = ShoppingListStore::new(&state.base_path);
    store.clear().map_err(|e| {
        tracing::error!("Failed to clear shopping list: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(StatusCode::OK)
}

// -- Check/uncheck endpoints --

#[derive(Debug, Deserialize)]
pub struct CheckItemRequest {
    pub name: String,
}

pub async fn check_shopping_item(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CheckItemRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.checked_log_lock.lock().await;
    let store = ShoppingListStore::new(&state.base_path);
    store.check(&payload.name).map_err(|e| {
        tracing::error!("Failed to check item: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(StatusCode::OK)
}

pub async fn uncheck_shopping_item(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CheckItemRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.checked_log_lock.lock().await;
    let store = ShoppingListStore::new(&state.base_path);
    store.uncheck(&payload.name).map_err(|e| {
        tracing::error!("Failed to uncheck item: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(StatusCode::OK)
}

pub async fn get_checked_items(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    let checked = store.checked_set().map_err(|e| {
        tracing::error!("Failed to get checked items: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(checked.into_iter().collect()))
}

pub async fn compact_checked(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.checked_log_lock.lock().await;
    let store = ShoppingListStore::new(&state.base_path);
    let names = aggregate_current_ingredient_names(&state).map_err(|e| {
        tracing::error!("Failed to aggregate ingredients for compact: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    store.compact(names).map_err(|e| {
        tracing::error!("Failed to compact checked list: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(StatusCode::OK)
}

/// Aggregate the ingredient names a user would see for the currently-stored
/// shopping list. Walks every recipe reference persisted in `.shopping-list`
/// and expands it through `extract_ingredients`, honoring any
/// `included_references` and recipe scale factors.
///
/// Returns names in their raw (non-common) form — `compact_checked` does a
/// case-insensitive comparison so that's fine for the stale-check step.
///
/// Returns `Err(..)` if any recipe fails to parse. The caller should refuse
/// to compact in that case — a partial ingredient set would mark otherwise-
/// valid checks as stale and wipe them, which is how the original bug this
/// module was fixing manifested.
fn aggregate_current_ingredient_names(state: &AppState) -> anyhow::Result<Vec<String>> {
    let store = ShoppingListStore::new(&state.base_path);
    let items = store.load()?;
    let mut list = IngredientList::new();

    // `extract_ingredients` uses `seen` to detect circular references
    // *within a single recipe tree*. The shopping list may legitimately
    // contain the same recipe multiple times (e.g. duplicate entries from
    // the legacy format), so we reset `seen` per top-level entry —
    // otherwise the second occurrence is misreported as a cycle and we'd
    // skip the compact, leaving stale checks in place.
    for item in &items {
        if let Some(recipes) = &item.recipes {
            // Menu/plan entry — expand each nested recipe.
            for recipe in recipes {
                let mut seen = BTreeMap::new();
                let scaled = format!("{}:{}", recipe.path, recipe.scale);
                extract_ingredients(
                    &scaled,
                    &mut list,
                    &mut seen,
                    &state.base_path,
                    PARSER.converter(),
                    false,
                    recipe.included_references.as_deref(),
                )
                .with_context(|| format!("aggregating ingredients for {scaled}"))?;
            }
        } else {
            let mut seen = BTreeMap::new();
            let scaled = format!("{}:{}", item.path, item.scale);
            extract_ingredients(
                &scaled,
                &mut list,
                &mut seen,
                &state.base_path,
                PARSER.converter(),
                false,
                item.included_references.as_deref(),
            )
            .with_context(|| format!("aggregating ingredients for {scaled}"))?;
        }
    }

    Ok(list.iter().map(|(name, _)| name.clone()).collect())
}

// -- Add menu (bulk) endpoint --

#[derive(Debug, Deserialize)]
pub struct AddMenuRequest {
    pub path: String,
    pub scale: f64,
}

/// Information about a referenced recipe needed to convert a menu's
/// `{target%unit}` into a storage multiplier for `.shopping-list`.
struct RecipeInfo {
    sub_refs: Vec<String>,
    default_servings: Option<u32>,
    /// Parsed `yield` metadata (value, unit) if present and well-formed,
    /// e.g. `yield: 500%ml` → `Some((500.0, "ml"))`.
    default_yield: Option<(f64, String)>,
}

/// Parse `"VALUE%UNIT"` (the only format `scale_to_yield` accepts) into its parts.
fn parse_yield(s: &str) -> Option<(f64, String)> {
    let (value, unit) = s.split_once('%')?;
    let value = value.trim().parse::<f64>().ok()?;
    let unit = unit.trim();
    if unit.is_empty() {
        return None;
    }
    Some((value, unit.to_string()))
}

fn resolve_recipe_info(base_path: &Utf8PathBuf, recipe_path: &str) -> anyhow::Result<RecipeInfo> {
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

/// Add all recipe references from a menu to the shopping list as a single
/// plan entry with recipes nested inside.
pub async fn add_menu_to_shopping_list(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddMenuRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    let menu_scale = payload.scale;

    let recipe_path = Utf8PathBuf::from(&payload.path);
    let entry = cooklang_find::get_recipe(vec![&state.base_path], &recipe_path).map_err(|e| {
        tracing::error!("Menu not found: {}", payload.path);
        (
            StatusCode::NOT_FOUND,
            Json(
                serde_json::json!({ "error": format!("Menu not found: {}: {}", payload.path, e) }),
            ),
        )
    })?;

    // Parse at scale 1.0 to get raw quantities for recipe references
    let menu = crate::util::parse_recipe_from_entry(&entry, 1.0).map_err(|e| {
        tracing::error!("Failed to parse menu: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to parse menu: {e}") })),
        )
    })?;

    let mut recipes = Vec::new();

    for ingredient in &menu.ingredients {
        if let Some(ref recipe_ref) = ingredient.reference {
            // Build display path from reference components
            let ref_display = if recipe_ref.components.is_empty() {
                recipe_ref.name.clone()
            } else {
                format!("{}/{}", recipe_ref.components.join("/"), recipe_ref.name)
            };

            // Resolve this recipe's sub-recipe references, default servings, and yield
            let ref_path_for_find = recipe_ref.path(std::path::MAIN_SEPARATOR_STR);
            let info = match resolve_recipe_info(&state.base_path, &ref_path_for_find) {
                Ok(info) => info,
                Err(e) => {
                    tracing::warn!(
                        "Could not resolve referenced recipe '{}': {}",
                        ref_display,
                        e
                    );
                    RecipeInfo {
                        sub_refs: Vec::new(),
                        default_servings: None,
                        default_yield: None,
                    }
                }
            };

            // Convert `{target%unit}` on the menu reference into a scale
            // multiplier for `.shopping-list`. Per the Cooklang spec
            // (conventions.md §"Scaling Referenced Recipes") the unit
            // decides how `target` is interpreted:
            //
            //   - no unit     → raw multiplier (`{2}` = ×2)
            //   - `servings`  → target servings; factor = target / default_servings
            //   - other unit  → target yield;    factor = target / default_yield_value
            //                   (only if the units match — no conversion)
            //
            // Storing a raw multiplier without this conversion was the bug:
            // e.g. a 2-serving recipe referenced as `{3%servings}` got stored
            // as `{3}` and scaled to 6 servings instead of 3.
            let recipe_factor = match ingredient.quantity.as_ref() {
                Some(q) => {
                    let value = match q.value() {
                        cooklang::quantity::Value::Number(n) => Some(n.value()),
                        _ => None,
                    };
                    match (value, q.unit()) {
                        (None, _) => 1.0,
                        (Some(v), None) => v,
                        (Some(target), Some("servings" | "serving")) => {
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
                            Some((base, base_unit)) if base_unit == unit && *base > 0.0 => {
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
                None => 1.0,
            };
            let final_scale = recipe_factor * menu_scale;
            let sub_refs = info.sub_refs;

            // Strip ./ prefix for storage (the format writer adds it back)
            let path = ref_display
                .strip_prefix("./")
                .unwrap_or(&ref_display)
                .to_string();

            recipes.push(ShoppingListApiItem {
                name: recipe_display_name(&path),
                path,
                scale: final_scale,
                included_references: Some(sub_refs),
                recipes: None,
            });
        }
    }

    store
        .add_menu(payload.path, menu_scale, recipes)
        .map_err(|e| {
            tracing::error!("Failed to add menu to shopping list: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    Ok(StatusCode::OK)
}
