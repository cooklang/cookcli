use crate::server::AppState;
use axum::{extract::State, http::StatusCode, Json};
use cooklang_find::RecipeTree;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub struct StatsResponse {
    pub recipe_count: usize,
    pub menu_count: usize,
}

fn json_error(msg: impl std::fmt::Display) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": msg.to_string() }))
}

fn count_entries(tree: &RecipeTree) -> (usize, usize) {
    let mut recipe_count = 0;
    let mut menu_count = 0;

    if let Some(ref entry) = tree.recipe {
        if entry.is_menu() {
            menu_count += 1;
        } else {
            recipe_count += 1;
        }
    }

    for child in tree.children.values() {
        let (r, m) = count_entries(child);
        recipe_count += r;
        menu_count += m;
    }

    (recipe_count, menu_count)
}

pub async fn stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let tree = cooklang_find::build_tree(&state.base_path).map_err(|e| {
        tracing::error!("Failed to build recipe tree: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(&e))
    })?;

    let (recipe_count, menu_count) = count_entries(&tree);

    Ok(Json(StatsResponse {
        recipe_count,
        menu_count,
    }))
}
