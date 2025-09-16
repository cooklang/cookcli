use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct AddPantryItem {
    pub section: String,
    pub name: String,
    pub quantity: Option<String>,
    pub bought: Option<String>,
    pub expire: Option<String>,
    pub low: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePantryItem {
    pub quantity: Option<String>,
    pub bought: Option<String>,
    pub expire: Option<String>,
    pub low: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

pub async fn add_item(
    State(state): State<Arc<AppState>>,
    Json(item): Json<AddPantryItem>,
) -> Result<impl IntoResponse, StatusCode> {
    let pantry_path = state.pantry_path.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    // Read existing pantry configuration
    let content =
        std::fs::read_to_string(pantry_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = cooklang::pantry::parse_lenient(&content);
    let mut pantry_conf = result.output().cloned().unwrap_or_default();

    // Create new item
    let new_item = if item.quantity.is_some()
        || item.bought.is_some()
        || item.expire.is_some()
        || item.low.is_some()
    {
        cooklang::pantry::PantryItem::WithAttributes(cooklang::pantry::ItemWithAttributes {
            name: item.name.clone(),
            quantity: item.quantity,
            bought: item.bought,
            expire: item.expire,
            low: item.low,
        })
    } else {
        cooklang::pantry::PantryItem::Simple(item.name.clone())
    };

    // Add item to the specified section
    pantry_conf
        .sections
        .entry(item.section.clone())
        .or_insert_with(Vec::new)
        .push(new_item);

    // Rebuild index
    pantry_conf.rebuild_index();

    // Serialize back to TOML
    let new_content =
        toml::to_string_pretty(&pantry_conf).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Write back to file
    std::fs::write(pantry_path, new_content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Added {} to {}", item.name, item.section),
    }))
}

pub async fn remove_item(
    State(state): State<Arc<AppState>>,
    Path((section, name)): Path<(String, String)>,
) -> Result<impl IntoResponse, StatusCode> {
    let pantry_path = state.pantry_path.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    // Read existing pantry configuration
    let content =
        std::fs::read_to_string(pantry_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = cooklang::pantry::parse_lenient(&content);
    let mut pantry_conf = result.output().cloned().ok_or(StatusCode::NOT_FOUND)?;

    // Remove item from the specified section
    if let Some(items) = pantry_conf.sections.get_mut(&section) {
        items.retain(|item| item.name() != name);

        // Remove section if empty
        if items.is_empty() {
            pantry_conf.sections.remove(&section);
        }
    } else {
        return Err(StatusCode::NOT_FOUND);
    }

    // Rebuild index
    pantry_conf.rebuild_index();

    // Serialize back to TOML
    let new_content =
        toml::to_string_pretty(&pantry_conf).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Write back to file
    std::fs::write(pantry_path, new_content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Removed {name} from {section}"),
    }))
}

pub async fn update_item(
    State(state): State<Arc<AppState>>,
    Path((section, name)): Path<(String, String)>,
    Json(update): Json<UpdatePantryItem>,
) -> Result<impl IntoResponse, StatusCode> {
    let pantry_path = state.pantry_path.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    // Read existing pantry configuration
    let content =
        std::fs::read_to_string(pantry_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = cooklang::pantry::parse_lenient(&content);
    let mut pantry_conf = result.output().cloned().ok_or(StatusCode::NOT_FOUND)?;

    // Find and update the item
    if let Some(items) = pantry_conf.sections.get_mut(&section) {
        for item in items.iter_mut() {
            if item.name() == name {
                // Convert to WithAttributes if needed
                let updated_item = match item {
                    cooklang::pantry::PantryItem::Simple(item_name) => {
                        if update.quantity.is_some()
                            || update.bought.is_some()
                            || update.expire.is_some()
                            || update.low.is_some()
                        {
                            cooklang::pantry::PantryItem::WithAttributes(
                                cooklang::pantry::ItemWithAttributes {
                                    name: item_name.clone(),
                                    quantity: update.quantity.clone(),
                                    bought: update.bought.clone(),
                                    expire: update.expire.clone(),
                                    low: update.low,
                                },
                            )
                        } else {
                            cooklang::pantry::PantryItem::Simple(item_name.clone())
                        }
                    }
                    cooklang::pantry::PantryItem::WithAttributes(attrs) => {
                        cooklang::pantry::PantryItem::WithAttributes(
                            cooklang::pantry::ItemWithAttributes {
                                name: attrs.name.clone(),
                                quantity: update.quantity.clone().or(attrs.quantity.clone()),
                                bought: update.bought.clone().or(attrs.bought.clone()),
                                expire: update.expire.clone().or(attrs.expire.clone()),
                                low: update.low.or_else(|| attrs.low.clone()),
                            },
                        )
                    }
                };
                *item = updated_item;
                break;
            }
        }
    } else {
        return Err(StatusCode::NOT_FOUND);
    }

    // Rebuild index
    pantry_conf.rebuild_index();

    // Serialize back to TOML
    let new_content =
        toml::to_string_pretty(&pantry_conf).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Write back to file
    std::fs::write(pantry_path, new_content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Updated {name} in {section}"),
    }))
}

pub async fn get_pantry(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let pantry_path = state.pantry_path.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    // Read existing pantry configuration
    let content =
        std::fs::read_to_string(pantry_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = cooklang::pantry::parse_lenient(&content);
    let pantry_conf = result.output().cloned().ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(pantry_conf))
}
