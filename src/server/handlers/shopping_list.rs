use crate::server::{
    shopping_list_store::{ShoppingListItem, ShoppingListStore},
    AppState,
};
use crate::util::{extract_ingredients, PARSER};
use axum::{extract::State, http::StatusCode, Json};
use cooklang::ingredient_list::IngredientList;
use serde::Deserialize;
use serde_json;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct RecipeRequest {
    recipe: String,
    scale: Option<f64>,
}

pub async fn shopping_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(payload): axum::extract::Json<Vec<RecipeRequest>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
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
        )
        .map_err(|e| {
            tracing::error!("Error processing recipe: {}", e);
            StatusCode::BAD_REQUEST
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
        let mut shopping_items = Vec::new();

        for (name, qty) in items {
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

    let json_value = serde_json::json!({
        "categories": shopping_categories,
        "pantry_items": pantry_items
    });
    Ok(Json(json_value))
}

pub async fn get_shopping_list_items(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ShoppingListItem>>, StatusCode> {
    let store = ShoppingListStore::new(&state.base_path);
    let items = store.load().map_err(|e| {
        tracing::error!("Failed to load shopping list: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(items))
}

#[derive(Debug, Deserialize)]
pub struct AddItemRequest {
    pub path: String,
    pub name: String,
    pub scale: f64,
}

pub async fn add_to_shopping_list(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddItemRequest>,
) -> Result<StatusCode, StatusCode> {
    let store = ShoppingListStore::new(&state.base_path);
    let item = ShoppingListItem {
        path: payload.path,
        name: payload.name,
        scale: payload.scale,
    };

    store.add(item).map_err(|e| {
        tracing::error!("Failed to add to shopping list: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
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
) -> Result<StatusCode, StatusCode> {
    let store = ShoppingListStore::new(&state.base_path);
    store.remove(&payload.path).map_err(|e| {
        tracing::error!("Failed to remove from shopping list: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::OK)
}

pub async fn clear_shopping_list(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    let store = ShoppingListStore::new(&state.base_path);
    store.clear().map_err(|e| {
        tracing::error!("Failed to clear shopping list: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::OK)
}
