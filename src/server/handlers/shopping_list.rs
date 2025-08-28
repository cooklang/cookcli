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
    let pantry_items = if let Some(path) = &state.pantry_path {
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

                if let Some(pantry) = result.output() {
                    // Extract all ingredient names from pantry sections
                    let mut items = Vec::new();
                    for section_items in pantry.sections.values() {
                        for item in section_items {
                            // Handle both simple and with-attributes items
                            let name = match item {
                                cooklang::pantry::PantryItem::Simple(name) => name.clone(),
                                cooklang::pantry::PantryItem::WithAttributes(attrs) => {
                                    attrs.name.clone()
                                }
                            };
                            items.push(name);
                        }
                    }
                    items
                } else {
                    tracing::warn!("Failed to parse pantry file");
                    Vec::new()
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read pantry file from {:?}: {}", path, e);
                Vec::new()
            }
        }
    } else {
        tracing::debug!("No pantry file configured");
        Vec::new()
    };

    let categories = list.categorize(&aisle);

    // Separate items that are in pantry
    let mut shopping_categories = Vec::new();
    let mut pantry_available = Vec::new();

    for (category, items) in categories {
        let mut shopping_items = Vec::new();

        for (name, qty) in items {
            // Check if item is in pantry
            let in_pantry = pantry_items
                .iter()
                .any(|pantry_item| pantry_item.to_lowercase() == name.to_lowercase());

            let item_json = serde_json::json!({
                "name": name.clone(),
                "quantities": qty.into_vec()
            });

            if in_pantry {
                pantry_available.push(item_json);
            } else {
                shopping_items.push(item_json);
            }
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
        "pantry_items": pantry_available
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
