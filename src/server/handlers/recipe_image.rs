//! `/api/recipe_image/{*path}`: read, upload and remove a recipe's title
//! picture — the `Recipe.jpg` beside `Recipe.cook` that `cooklang-find` picks
//! up.
//!
//! It is not a `/api/recipes/image/{*path}` sub-route because axum only
//! allows a catch-all last, so the static segment would come first and claim
//! every recipe in a folder named `image/` the way `/recipes/raw/` already
//! claims one named `raw/`.

use crate::server::{
    fs_atomic,
    handlers::common::{check_path, json_error},
    title_image::{self, PrepareError},
    AppState,
};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang_find::RecipeEntry;
use std::sync::Arc;

type ApiError = (StatusCode, Json<serde_json::Value>);

/// The extensions `cooklang-find` tries for a title picture, in its order.
const TITLE_IMAGE_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

/// The title picture as the recipe page will show it.
pub async fn recipe_image_get(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    picture_state(&state, &path)
}

/// Stores the request body as the recipe's title picture.
///
/// The body is the picture's bytes, not a multipart form. Whatever arrives is
/// saved as `Recipe.jpg` (see [`title_image`]), and any `Recipe.jpeg`,
/// `.png` or `.webp` from before is removed so the folder keeps one picture.
pub async fn recipe_image_put(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let recipe = recipe_file(&find_recipe(&state, &path)?, &path)?;
    if body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            json_error("The request body is empty; send the picture's bytes."),
        ));
    }

    let jpeg = title_image::prepare_async(body)
        .await
        .map_err(prepare_error)?;

    let target = recipe.with_extension("jpg");
    write_atomically(&target, jpeg).await?;
    for ext in TITLE_IMAGE_EXTENSIONS
        .into_iter()
        .filter(|ext| *ext != "jpg")
    {
        remove_if_present(&recipe.with_extension(ext)).await?;
    }
    tracing::info!("Saved title picture: {target}");

    picture_state(&state, &path)
}

/// Removes every title picture file the recipe has.
///
/// A picture named by the recipe's `image` metadata is left alone; that is
/// the recipe's text to change.
pub async fn recipe_image_delete(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let recipe = recipe_file(&find_recipe(&state, &path)?, &path)?;

    let mut removed = false;
    for ext in TITLE_IMAGE_EXTENSIONS {
        removed |= remove_if_present(&recipe.with_extension(ext)).await?;
    }
    if !removed {
        return Err((
            StatusCode::NOT_FOUND,
            json_error(format!("{path} has no title picture file")),
        ));
    }
    tracing::info!("Removed title picture of {recipe}");

    picture_state(&state, &path)
}

fn find_recipe(state: &AppState, path: &str) -> Result<RecipeEntry, ApiError> {
    check_path(path)?;
    cooklang_find::get_recipe(vec![&state.base_path], &Utf8PathBuf::from(path)).map_err(|e| {
        tracing::error!("Recipe not found: {path}");
        (
            StatusCode::NOT_FOUND,
            json_error(format!("Recipe not found: {path}: {e}")),
        )
    })
}

fn recipe_file(entry: &RecipeEntry, path: &str) -> Result<Utf8PathBuf, ApiError> {
    entry.path().cloned().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            json_error(format!("Recipe has no file path: {path}")),
        )
    })
}

/// `{ path, image, source }`, read back from disk so it reports what the
/// recipe page will now show rather than what was just written.
///
/// `source` is `"metadata"` when the recipe's `image` (or `images`,
/// `picture`, `pictures`) metadata names the picture — that wins over any
/// file — `"file"` for a picture beside the recipe, and null for none.
fn picture_state(state: &AppState, path: &str) -> Result<Json<serde_json::Value>, ApiError> {
    let entry = find_recipe(state, path)?;
    let image = entry.title_image().clone().and_then(|image| {
        crate::web::builders::get_image_path(&state.base_path, &state.url_prefix, image)
    });
    let source = image.as_ref().map(|_| {
        if entry.metadata().image_url().is_some() {
            "metadata"
        } else {
            "file"
        }
    });
    Ok(Json(serde_json::json!({
        "path": path,
        "image": image,
        "source": source,
    })))
}

fn prepare_error(e: PrepareError) -> ApiError {
    let (status, code) = match e {
        PrepareError::Heif => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "heif"),
        PrepareError::Unsupported => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported"),
        PrepareError::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid"),
        PrepareError::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "too_large"),
    };
    tracing::warn!("Refused a title picture: {e}");
    (
        status,
        Json(serde_json::json!({ "error": e.to_string(), "code": code })),
    )
}

/// Writes beside `target` and renames over it, so a reader never sees half a
/// picture. The temporary name is hidden and ends in `.tmp`, which
/// `cooklang-find` and cook.md sync both pass over.
async fn write_atomically(target: &Utf8Path, bytes: Vec<u8>) -> Result<(), ApiError> {
    let name = target.file_name().unwrap_or("picture");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or_default();
    let temp = target.with_file_name(format!(".{name}.{}-{nanos}.tmp", std::process::id()));

    let failed = |e: std::io::Error| {
        tracing::error!("Failed to save title picture {target}: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error(format!("Failed to save the picture: {e}")),
        )
    };
    tokio::fs::write(&temp, bytes).await.map_err(failed)?;
    if let Err(e) = fs_atomic::rename_replace_async(temp.clone(), target.to_owned()).await {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err(failed(e));
    }
    Ok(())
}

/// Removes `file`, reporting whether there was one.
async fn remove_if_present(file: &Utf8Path) -> Result<bool, ApiError> {
    match tokio::fs::remove_file(file).await {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => {
            tracing::error!("Failed to remove {file}: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!(
                    "Failed to remove {}: {e}",
                    file.file_name().unwrap_or("")
                )),
            ))
        }
    }
}
