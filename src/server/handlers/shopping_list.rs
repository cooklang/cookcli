use crate::server::AppState;
use crate::util::extract_ingredients;
use axum::{extract::State, http::StatusCode, Json};
use cooklang::ingredient_list::IngredientList;
use serde_json;
use std::collections::BTreeMap;
use std::sync::Arc;

pub async fn shopping_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(payload): axum::extract::Json<Vec<String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut list = IngredientList::new();
    let mut seen = BTreeMap::new();

    for entry in payload {
        extract_ingredients(
            &entry,
            &mut list,
            &mut seen,
            &state.base_path,
            state.parser.converter(),
            false,
        )
        .map_err(|e| {
            tracing::error!("Error processing recipe: {}", e);
            StatusCode::BAD_REQUEST
        })?;
    }

    let aisle_content = if let Some(path) = &state.aisle_path {
        std::fs::read_to_string(path).unwrap_or_default()
    } else {
        tracing::warn!("No aisle file set");
        String::new()
    };

    let aisle = cooklang::aisle::parse(&aisle_content).unwrap_or_default();

    let categories = list.categorize(&aisle);
    let json_value = serde_json::json!({
        "categories": categories.into_iter().map(|(category, items)| {
            serde_json::json!({
                "category": category,
                "items": items.into_iter().map(|(name, qty)| {
                    serde_json::json!({
                        "name": name,
                        "quantities": qty.into_vec()
                    })
                }).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>()
    });
    Ok(Json(json_value))
}
