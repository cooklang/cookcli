use crate::server::AppState;
use axum::{extract::State, http::StatusCode, Json};
use cooklang_find::RecipeTree;
use serde::Serialize;
use std::sync::Arc;

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
                let name = entry
                    .name()
                    .clone()
                    .unwrap_or_else(|| relative.to_string());
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
