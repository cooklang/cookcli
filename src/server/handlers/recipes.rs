use crate::{server::AppState, util::PARSER};
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
use tokio::io::AsyncWriteExt;

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

    let recipe =
        crate::util::parse_recipe_from_entry(&entry, query.scale.unwrap_or(1.0)).map_err(|e| {
            tracing::error!("Failed to parse recipe: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get the image path if available
    let image_path = entry.title_image().clone().and_then(|img_path| {
        // If it's a URL, use it directly
        if img_path.starts_with("http://") || img_path.starts_with("https://") {
            Some(img_path)
        } else {
            // For file paths, make them relative and accessible via /api/static
            let img_path = camino::Utf8Path::new(&img_path);

            // Try to strip the base_path prefix to get a relative path
            if let Ok(relative) = img_path.strip_prefix(&state.base_path) {
                Some(format!("/api/static/{relative}"))
            } else {
                // If the path doesn't start with base_path, it might already be relative
                // or it might be an absolute path to a file within base_path
                if !img_path.is_absolute() {
                    Some(format!("/api/static/{img_path}"))
                } else {
                    // Last resort: try to get just the filename
                    img_path
                        .file_name()
                        .map(|name| format!("/api/static/{name}"))
                }
            }
        }
    });

    #[derive(Serialize)]
    struct ApiRecipe {
        #[serde(flatten)]
        recipe: Arc<cooklang::Recipe>,
        grouped_ingredients: Vec<serde_json::Value>,
    }

    let grouped_ingredients = recipe
        .group_ingredients(PARSER.converter())
        .into_iter()
        .map(|entry| {
            serde_json::json!({
                "index": entry.index,
                "quantities": entry.quantity.into_vec()
            })
        })
        .collect();

    let api_recipe = ApiRecipe {
        recipe,
        grouped_ingredients,
    };

    let value = serde_json::json!({
        "recipe": api_recipe,
        "image": image_path,
        "scale": query.scale.unwrap_or(1.0),
        // TODO: add more metadata if needed
    });

    Ok(Json(value))
}

pub async fn recipe_raw(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<String, StatusCode> {
    check_path(&path)?;

    let recipe_path = state.base_path.join(&path);

    // Try .cook extension first, then .menu
    let file_path = if recipe_path.exists() {
        recipe_path
    } else {
        let cook_path = Utf8PathBuf::from(format!("{}.cook", recipe_path));
        let menu_path = Utf8PathBuf::from(format!("{}.menu", recipe_path));

        if cook_path.exists() {
            cook_path
        } else if menu_path.exists() {
            menu_path
        } else {
            tracing::error!("Recipe file not found: {path}");
            return Err(StatusCode::NOT_FOUND);
        }
    };

    tokio::fs::read_to_string(&file_path).await.map_err(|e| {
        tracing::error!("Failed to read recipe file {}: {}", file_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

pub async fn recipe_save(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<serde_json::Value>, StatusCode> {
    check_path(&path)?;

    let recipe_path = state.base_path.join(&path);

    // Determine actual file path (with extension)
    let file_path = if recipe_path.exists() {
        recipe_path
    } else {
        let cook_path = Utf8PathBuf::from(format!("{}.cook", recipe_path));
        let menu_path = Utf8PathBuf::from(format!("{}.menu", recipe_path));

        if cook_path.exists() {
            cook_path
        } else if menu_path.exists() {
            menu_path
        } else {
            // Default to .cook for new files
            Utf8PathBuf::from(format!("{}.cook", recipe_path))
        }
    };

    // Atomic write: write to temp file, then rename
    let temp_path = file_path.with_extension("tmp");

    let mut temp_file = tokio::fs::File::create(&temp_path).await.map_err(|e| {
        tracing::error!("Failed to create temp file {}: {}", temp_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    temp_file.write_all(body.as_bytes()).await.map_err(|e| {
        tracing::error!("Failed to write to temp file {}: {}", temp_path, e);
        // Fire-and-forget cleanup - spawn so we don't block the error path
        let temp_path_clone = temp_path.clone();
        tokio::spawn(async move { let _ = tokio::fs::remove_file(&temp_path_clone).await; });
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tokio::fs::rename(&temp_path, &file_path).await.map_err(|e| {
        tracing::error!("Failed to rename temp file to {}: {}", file_path, e);
        let temp_path_clone = temp_path.clone();
        tokio::spawn(async move { let _ = tokio::fs::remove_file(&temp_path_clone).await; });
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Saved recipe: {}", file_path);

    Ok(Json(serde_json::json!({
        "status": "success",
        "path": path
    })))
}

pub async fn reload() -> Result<Json<serde_json::Value>, StatusCode> {
    // Since the server reads from disk on each request, there's no cache to clear.
    // This endpoint just returns success to indicate the reload was processed.
    tracing::info!("Reload requested - recipes will be refreshed from disk on next request");
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Recipes will be refreshed from disk on next request"
    })))
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
        .filter_map(|recipe| {
            recipe.path().map(|path| {
                let relative_path = path.strip_prefix(&state.base_path).unwrap_or(path);
                serde_json::json!({
                    "name": recipe.name(),
                    "path": relative_path.to_string()
                })
            })
        })
        .collect();

    Ok(Json(results))
}

pub async fn recipe_delete(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    check_path(&path)?;

    let recipe_path = state.base_path.join(&path);

    // Determine actual file path (with extension)
    let file_path = if recipe_path.exists() {
        recipe_path
    } else {
        let cook_path = Utf8PathBuf::from(format!("{}.cook", recipe_path));
        let menu_path = Utf8PathBuf::from(format!("{}.menu", recipe_path));

        if cook_path.exists() {
            cook_path
        } else if menu_path.exists() {
            menu_path
        } else {
            tracing::error!("Recipe file not found for deletion: {path}");
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Delete the file
    std::fs::remove_file(&file_path).map_err(|e| {
        tracing::error!("Failed to delete recipe file {}: {}", file_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Deleted recipe: {}", file_path);

    Ok(Json(serde_json::json!({
        "status": "success",
        "path": path
    })))
}
