use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use cooklang_find;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(default)]
struct ColorConfig {
    color: bool,
}

#[derive(Deserialize)]
pub struct RecipeQuery {
    scale: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    q: String,
}

fn check_path(p: &str) -> Result<(), StatusCode> {
    let path = Utf8Path::new(p);
    if !path
        .components()
        .all(|c| matches!(c, Utf8Component::Normal(_)))
    {
        tracing::error!("Invalid path: {p}");
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

pub async fn all_recipes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let recipes = cooklang_find::build_tree(&state.base_path).map_err(|e| {
        tracing::error!("Failed to build recipe tree: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let recipes = serde_json::to_value(recipes).map_err(|e| {
        tracing::error!("Failed to serialize recipes: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(recipes))
}

pub async fn recipe(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<RecipeQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    check_path(&path)?;

    let entry = cooklang_find::get_recipe(vec![&state.base_path], &Utf8PathBuf::from(&path))
        .map_err(|_| {
            tracing::error!("Recipe not found: {path}");
            StatusCode::NOT_FOUND
        })?;

    let recipe = entry.recipe(query.scale.unwrap_or(1.0));

    #[derive(Serialize)]
    struct ApiRecipe {
        #[serde(flatten)]
        recipe: Arc<cooklang::ScaledRecipe>,
        grouped_ingredients: Vec<serde_json::Value>,
    }

    let grouped_ingredients = recipe
        .group_ingredients(state.parser.converter())
        .into_iter()
        .map(|entry| {
            serde_json::json!({
                "index": entry.index,
                "quantities": entry.quantity.into_vec(),
                "outcome": entry.outcome
            })
        })
        .collect();

    let api_recipe = ApiRecipe {
        recipe,
        grouped_ingredients,
    };

    let value = serde_json::json!({
        "recipe": api_recipe,
        // TODO: add images
        // TODO: add scaling info
        // TODO: add metadata
    });

    Ok(Json(value))
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let recipes = cooklang_find::search(&state.base_path, &query.q).map_err(|e| {
        tracing::error!("Failed to search recipes: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let results = recipes
        .into_iter()
        .map(|recipe| {
            let path = recipe.path().as_ref().unwrap();
            let relative_path = path.strip_prefix(&state.base_path).unwrap_or(path);
            serde_json::json!({
                "name": recipe.name(),
                "path": relative_path.to_string()
            })
        })
        .collect();

    Ok(Json(results))
}
