use super::common::json_error;
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use camino::Utf8PathBuf;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;

use crate::server::AppState;

/// Load and parse the pantry file asynchronously.
async fn load_pantry(
    state: &AppState,
) -> Result<cooklang::pantry::PantryConf, (StatusCode, Json<serde_json::Value>)> {
    let pantry_path = get_pantry_path(state)?;

    let content = tokio::fs::read_to_string(pantry_path.as_std_path())
        .await
        .map_err(|e| {
            tracing::error!("Failed to read pantry file: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!("Failed to read pantry file: {e}")),
            )
        })?;

    let result = cooklang::pantry::parse_lenient(&content);
    result.output().cloned().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            json_error("Failed to parse pantry configuration"),
        )
    })
}

fn get_pantry_path(
    state: &AppState,
) -> Result<&Utf8PathBuf, (StatusCode, Json<serde_json::Value>)> {
    state.pantry_path.as_ref().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            json_error("Pantry configuration not found"),
        )
    })
}

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
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let pantry_path = get_pantry_path(&state)?;
    let mut pantry_conf = load_pantry(&state).await.unwrap_or_default();

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

    // Serialize back to regular TOML format (not array format)
    let new_content = serialize_pantry_to_regular_toml(&pantry_conf);

    // Write back to file
    tokio::fs::write(pantry_path.as_std_path(), new_content)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write pantry file: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!("Failed to write pantry file: {e}")),
            )
        })?;

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Added {} to {}", item.name, item.section),
    }))
}

pub async fn remove_item(
    State(state): State<Arc<AppState>>,
    Path((section, name)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let pantry_path = get_pantry_path(&state)?;
    let mut pantry_conf = load_pantry(&state).await?;

    // Remove item from the specified section
    if let Some(items) = pantry_conf.sections.get_mut(&section) {
        items.retain(|item| item.name() != name);

        // Remove section if empty
        if items.is_empty() {
            pantry_conf.sections.shift_remove(&section);
        }
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            json_error(format!("Section not found: {section}")),
        ));
    }

    // Rebuild index
    pantry_conf.rebuild_index();

    // Serialize back to regular TOML format (not array format)
    let new_content = serialize_pantry_to_regular_toml(&pantry_conf);

    // Write back to file
    tokio::fs::write(pantry_path.as_std_path(), new_content)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write pantry file: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!("Failed to write pantry file: {e}")),
            )
        })?;

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Removed {name} from {section}"),
    }))
}

pub async fn update_item(
    State(state): State<Arc<AppState>>,
    Path((section, name)): Path<(String, String)>,
    Json(update): Json<UpdatePantryItem>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let pantry_path = get_pantry_path(&state)?;
    let mut pantry_conf = load_pantry(&state).await?;

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
                                low: update.low.clone().or(attrs.low.clone()),
                            },
                        )
                    }
                };
                *item = updated_item;
                break;
            }
        }
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            json_error(format!("Section not found: {section}")),
        ));
    }

    // Rebuild index
    pantry_conf.rebuild_index();

    // Serialize back to regular TOML format (not array format)
    let new_content = serialize_pantry_to_regular_toml(&pantry_conf);

    // Write back to file
    tokio::fs::write(pantry_path.as_std_path(), new_content)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write pantry file: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!("Failed to write pantry file: {e}")),
            )
        })?;

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Updated {name} in {section}"),
    }))
}

pub async fn get_pantry(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let pantry_conf = load_pantry(&state).await?;
    Ok(Json(pantry_conf))
}

#[derive(Debug, Deserialize)]
pub struct ExpiringQuery {
    pub days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ExpiringItemResponse {
    pub section: String,
    pub name: String,
    pub expire: String,
    pub days_remaining: i64,
}

#[derive(Debug, Serialize)]
pub struct DepletedItemResponse {
    pub section: String,
    pub name: String,
    pub low: Option<String>,
}

pub async fn get_expiring(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ExpiringQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let days = query.days.unwrap_or(7);
    if days < 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            json_error("days must be non-negative"),
        ));
    }

    let pantry_conf = load_pantry(&state).await?;
    let today = Local::now().date_naive();
    let threshold = today + chrono::Duration::days(days);

    let mut items = Vec::new();

    for (section, section_items) in &pantry_conf.sections {
        for item in section_items {
            if let Some(expire_str) = item.expire() {
                if let Some(date) = parse_date(expire_str) {
                    if date <= threshold {
                        let days_remaining = (date - today).num_days();
                        items.push(ExpiringItemResponse {
                            section: section.clone(),
                            name: item.name().to_string(),
                            expire: date.format("%Y-%m-%d").to_string(),
                            days_remaining,
                        });
                    }
                }
            }
        }
    }

    // Sort by days_remaining so most urgent items come first
    items.sort_by_key(|item| item.days_remaining);

    Ok(Json(items))
}

pub async fn get_depleted(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let pantry_conf = load_pantry(&state).await?;
    let mut items = Vec::new();

    for (section, section_items) in &pantry_conf.sections {
        for item in section_items {
            if item.is_low() {
                items.push(DepletedItemResponse {
                    section: section.clone(),
                    name: item.name().to_string(),
                    low: item.low().map(|l| l.to_string()),
                });
            }
        }
    }

    Ok(Json(items))
}

/// Parse a date string supporting multiple formats
pub fn parse_date(date_str: &str) -> Option<NaiveDate> {
    let formats = [
        "%Y-%m-%d", "%d.%m.%Y", "%d/%m/%Y", "%m/%d/%Y", "%Y.%m.%d", "%d-%m-%Y",
    ];

    for format in &formats {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
            return Some(date);
        }
    }

    None
}

fn serialize_pantry_to_regular_toml(pantry_conf: &cooklang::pantry::PantryConf) -> String {
    cooklang::pantry::to_toml_string(pantry_conf)
}
