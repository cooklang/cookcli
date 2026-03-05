use super::common::json_error;
use crate::server::AppState;
use axum::{extract::State, http::StatusCode, Json};
use chrono::prelude::*;
use cooklang_find::RecipeTree;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub struct StatsResponse {
    pub recipe_count: usize,
    pub menu_count: usize,
    pub pantry_item_count: usize,
    pub pantry_expiring_count: usize,
    pub pantry_depleted_count: usize,
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

    // Count pantry stats if pantry file is available
    let (pantry_item_count, pantry_expiring_count, pantry_depleted_count) = match &state.pantry_path
    {
        Some(pantry_path) => match tokio::fs::read_to_string(pantry_path.as_std_path()).await {
            Ok(content) => {
                let result = cooklang::pantry::parse_lenient(&content);
                match result.output() {
                    Some(pantry_conf) => {
                        let item_count: usize =
                            pantry_conf.sections.values().map(|items| items.len()).sum();

                        let today = Local::now().date_naive();
                        let threshold = today + chrono::Duration::days(7);
                        let mut expiring = 0;
                        let mut depleted = 0;

                        for items in pantry_conf.sections.values() {
                            for item in items {
                                if let Some(expire_str) = item.expire() {
                                    if let Some(date) = super::pantry::parse_date(expire_str) {
                                        if date <= threshold {
                                            expiring += 1;
                                        }
                                    }
                                }
                                if item.is_low() {
                                    depleted += 1;
                                }
                            }
                        }

                        (item_count, expiring, depleted)
                    }
                    None => (0, 0, 0),
                }
            }
            Err(_) => (0, 0, 0),
        },
        None => (0, 0, 0),
    };

    Ok(Json(StatsResponse {
        recipe_count,
        menu_count,
        pantry_item_count,
        pantry_expiring_count,
        pantry_depleted_count,
    }))
}
