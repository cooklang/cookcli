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

    // Serialize back to regular TOML format (not array format)
    let new_content = serialize_pantry_to_regular_toml(&pantry_conf);

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

    // Serialize back to regular TOML format (not array format)
    let new_content = serialize_pantry_to_regular_toml(&pantry_conf);

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
        return Err(StatusCode::NOT_FOUND);
    }

    // Rebuild index
    pantry_conf.rebuild_index();

    // Serialize back to regular TOML format (not array format)
    let new_content = serialize_pantry_to_regular_toml(&pantry_conf);

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

/// Serialize PantryConf to regular TOML format (not array format)
fn serialize_pantry_to_regular_toml(pantry_conf: &cooklang::pantry::PantryConf) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    // First, handle any top-level items (if they exist)
    if let Some(general_items) = pantry_conf.sections.get("general") {
        for item in general_items {
            write_pantry_item(&mut output, item);
        }
        if !general_items.is_empty() {
            writeln!(&mut output).unwrap();
        }
    }

    // Then handle all other sections in alphabetical order
    for (section_name, items) in &pantry_conf.sections {
        if section_name == "general" {
            continue; // Already handled
        }

        writeln!(&mut output, "[{section_name}]").unwrap();

        for item in items {
            write_pantry_item(&mut output, item);
        }

        writeln!(&mut output).unwrap();
    }

    output
}

fn write_pantry_item(output: &mut String, item: &cooklang::pantry::PantryItem) {
    use std::fmt::Write;

    match item {
        cooklang::pantry::PantryItem::Simple(name) => {
            writeln!(output, "{} = true", toml_escape_key(name)).unwrap();
        }
        cooklang::pantry::PantryItem::WithAttributes(attrs) => {
            let has_quantity = attrs.quantity.is_some();
            let has_other_attrs =
                attrs.bought.is_some() || attrs.expire.is_some() || attrs.low.is_some();

            let value = if has_quantity && has_other_attrs {
                // Use inline table when quantity and other attributes are present
                let mut parts = Vec::new();
                if let Some(qty) = &attrs.quantity {
                    parts.push(format!("quantity = \"{}\"", qty.replace('"', "\\\"")));
                }
                if let Some(bought) = &attrs.bought {
                    parts.push(format!("bought = \"{}\"", bought.replace('"', "\\\"")));
                }
                if let Some(expire) = &attrs.expire {
                    parts.push(format!("expire = \"{}\"", expire.replace('"', "\\\"")));
                }
                if let Some(low) = &attrs.low {
                    parts.push(format!("low = \"{}\"", low.replace('"', "\\\"")));
                }
                format!("{{ {} }}", parts.join(", "))
            } else if let Some(qty) = &attrs.quantity {
                format!("\"{}\"", qty.replace('"', "\\\""))
            } else if has_other_attrs {
                let mut parts = Vec::new();
                if let Some(bought) = &attrs.bought {
                    parts.push(format!("bought = \"{}\"", bought.replace('"', "\\\"")));
                }
                if let Some(expire) = &attrs.expire {
                    parts.push(format!("expire = \"{}\"", expire.replace('"', "\\\"")));
                }
                if let Some(low) = &attrs.low {
                    parts.push(format!("low = \"{}\"", low.replace('"', "\\\"")));
                }
                format!("{{ {} }}", parts.join(", "))
            } else {
                "true".to_string()
            };

            writeln!(output, "{} = {}", toml_escape_key(&attrs.name), value).unwrap();
        }
    }
}

fn toml_escape_key(key: &str) -> String {
    // If the key contains special characters or spaces, quote it
    if key.contains(' ') || key.contains('.') || key.contains('[') || key.contains(']') {
        format!("\"{}\"", key.replace('"', "\\\""))
    } else {
        key.to_string()
    }
}
