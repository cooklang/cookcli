//! `/api/recipe_image/{*path}`: read, upload and remove a recipe's title
//! picture — the `Recipe.jpg` beside `Recipe.cook` that `cooklang-find` picks
//! up — and, with `?section=S&step=N`, the picture of one step, saved as
//! `Recipe.S.N.jpg` (#562).
//!
//! It is not a `/api/recipes/image/{*path}` sub-route because axum only
//! allows a catch-all last, so the static segment would come first and claim
//! every recipe in a folder named `image/` the way `/recipes/raw/` already
//! claims one named `raw/`. The step goes in the query for the same reason.

use crate::server::activity;
use crate::server::{
    fs_atomic,
    handlers::{
        common::{check_path, json_error},
        recipe_rename::RECIPE_FILES,
    },
    title_image::{self, PrepareError},
    AppState,
};
use crate::web::viewer::Viewer;
use axum::{
    body::Bytes,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang_find::RecipeEntry;
use serde::Deserialize;
use std::sync::Arc;

type ApiError = (StatusCode, Json<serde_json::Value>);

/// The extensions `cooklang-find` tries for a title or step picture, in its
/// order.
const IMAGE_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

/// `?section=S&step=N`: which step's picture a request is about. Both are
/// one-based; `section` defaults to 1. Without `step` it is the title picture.
///
/// Taken as text and checked by [`requested_step`], so a bad value gets the
/// same JSON error as every other refusal rather than axum's plain text.
#[derive(Debug, Default, Deserialize)]
pub struct PictureQuery {
    section: Option<String>,
    step: Option<String>,
}

/// A step that exists in the recipe as saved.
struct StepSlot {
    /// One-based, counting every section of the parsed recipe.
    section: usize,
    /// One-based, within the section.
    step: usize,
    /// One-based, counted across every section: the `G` of `Recipe.G.jpg`.
    overall: usize,
}

/// A section of the parsed recipe, reduced to what the picture dialog lists.
struct SectionSteps {
    name: Option<String>,
    /// Each step's text, as plain words.
    steps: Vec<String>,
}

/// The title picture as the recipe page will show it, with every step and
/// its picture; or, with `?step=`, that one step's picture.
pub async fn recipe_image_get(
    Path(path): Path<String>,
    Query(query): Query<PictureQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let requested = requested_step(&query)?;
    match requested {
        None => picture_state(&state, &path),
        Some((section, step)) => {
            let slot = step_slot(&find_recipe(&state, &path)?, section, step, &path)?;
            step_state(&state, &path, &slot)
        }
    }
}

/// Stores the request body as the recipe's title picture, or with `?step=`
/// as that step's picture.
///
/// The body is the picture's bytes, not a multipart form. Whatever arrives is
/// saved as `Recipe.jpg` or `Recipe.S.N.jpg` (see [`title_image`]), and the
/// older files that picture would compete with are removed, so the folder
/// keeps one picture for it: see [`picture_files`].
pub async fn recipe_image_put(
    Path(path): Path<String>,
    Query(query): Query<PictureQuery>,
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let requested = requested_step(&query)?;
    let entry = find_recipe(&state, &path)?;
    let recipe = recipe_file(&entry, &path)?;
    let slot = requested
        .map(|(section, step)| step_slot(&entry, section, step, &path))
        .transpose()?;
    if body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            json_error("The request body is empty; send the picture's bytes."),
        ));
    }

    let jpeg = title_image::prepare_async(body)
        .await
        .map_err(prepare_error)?;

    // Not before decoding, which takes seconds. The recipe may have been
    // renamed meanwhile: the picture would then land under its old name.
    let _writing = RECIPE_FILES.lock().await;
    if !recipe.is_file() {
        return Err((
            StatusCode::NOT_FOUND,
            json_error(format!("{path} was renamed or removed meanwhile")),
        ));
    }
    let files = picture_files(&recipe, slot.as_ref());
    let (target, older) = files.split_first().expect("there is always a target");
    write_atomically(target, jpeg).await?;
    for file in older {
        remove_if_present(file).await?;
    }
    activity::record(
        &viewer,
        format_args!(
            "set the picture of {}{}",
            step_label(slot.as_ref()),
            activity::file(&state.base_path, &recipe)
        ),
    );

    match slot {
        None => picture_state(&state, &path),
        Some(slot) => step_state(&state, &path, &slot),
    }
}

/// Removes every picture file of the title, or with `?step=` of that step.
///
/// A picture named by the recipe's `image` metadata is left alone; that is
/// the recipe's text to change.
pub async fn recipe_image_delete(
    Path(path): Path<String>,
    Query(query): Query<PictureQuery>,
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _writing = RECIPE_FILES.lock().await;
    let requested = requested_step(&query)?;
    let entry = find_recipe(&state, &path)?;
    let recipe = recipe_file(&entry, &path)?;
    let slot = requested
        .map(|(section, step)| step_slot(&entry, section, step, &path))
        .transpose()?;

    let mut removed = false;
    for file in picture_files(&recipe, slot.as_ref()) {
        removed |= remove_if_present(&file).await?;
    }
    if !removed {
        let what = match &slot {
            None => "title picture file".to_string(),
            Some(slot) => format!(
                "picture file for step {} in section {}",
                slot.step, slot.section
            ),
        };
        return Err((
            StatusCode::NOT_FOUND,
            json_error(format!("{path} has no {what}")),
        ));
    }
    activity::record(
        &viewer,
        format_args!(
            "removed the picture of {}{}",
            step_label(slot.as_ref()),
            activity::file(&state.base_path, &recipe)
        ),
    );

    match slot {
        None => picture_state(&state, &path),
        Some(slot) => step_state(&state, &path, &slot),
    }
}

/// `"step N in section S of "` for the activity line, or nothing for the
/// title picture. Numbers only, so nothing from the request reaches the line.
fn step_label(slot: Option<&StepSlot>) -> String {
    slot.map(|slot| format!("step {} in section {} of ", slot.step, slot.section))
        .unwrap_or_default()
}

/// The files a picture may be stored under, the one an upload writes first.
///
/// A title picture is `Recipe.jpg`, `.jpeg`, `.png` or `.webp`. A step has
/// two conventions (#374), and the recipe page takes the first it finds: its
/// own `Recipe.S.N.ext`, then `Recipe.G.ext` with the step counted across
/// every section. Both are this step's, in every extension, so an upload
/// clears them all — an older `Recipe.G.png` would otherwise come back once
/// the new picture is removed.
fn picture_files(recipe: &Utf8Path, slot: Option<&StepSlot>) -> Vec<Utf8PathBuf> {
    let Some(slot) = slot else {
        return IMAGE_EXTENSIONS
            .iter()
            .map(|ext| recipe.with_extension(ext))
            .collect();
    };
    let stem = recipe.file_stem().unwrap_or_default();
    let named = |numbers: &str, ext: &str| recipe.with_file_name(format!("{stem}.{numbers}.{ext}"));
    let own = format!("{}.{}", slot.section, slot.step);
    let overall = slot.overall.to_string();
    IMAGE_EXTENSIONS
        .iter()
        .map(|ext| named(&own, ext))
        .chain(IMAGE_EXTENSIONS.iter().map(|ext| named(&overall, ext)))
        .collect()
}

/// Reads `?section=&step=`: `None` for the title picture.
fn requested_step(query: &PictureQuery) -> Result<Option<(usize, usize)>, ApiError> {
    let number = |name: &str, value: &str| {
        value
            .parse::<usize>()
            .ok()
            .filter(|n| *n >= 1)
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    json_error(format!("`{name}` must be a whole number from 1 up")),
                )
            })
    };
    let Some(step) = &query.step else {
        if query.section.is_some() {
            return Err((
                StatusCode::BAD_REQUEST,
                json_error("`section` needs a `step` to go with it"),
            ));
        }
        return Ok(None);
    };
    let section = match &query.section {
        Some(section) => number("section", section)?,
        None => 1,
    };
    Ok(Some((section, number("step", step)?)))
}

/// The steps of the recipe as saved, section by section, empty sections
/// included so the positions match the file names.
fn recipe_steps(entry: &RecipeEntry) -> anyhow::Result<Vec<SectionSteps>> {
    let recipe = crate::util::parse_unscaled_recipe_from_entry(entry)?;
    Ok(recipe
        .sections
        .iter()
        .map(|section| SectionSteps {
            name: section.name.clone(),
            steps: section
                .content
                .iter()
                .filter_map(|content| match content {
                    cooklang::Content::Step(step) => Some(step_text(&recipe, step)),
                    cooklang::Content::Text(_) => None,
                })
                .collect(),
        })
        .collect())
}

/// A step as plain words, for the dialog to tell steps apart by.
fn step_text(recipe: &cooklang::Recipe, step: &cooklang::Step) -> String {
    use cooklang::Item;
    let mut text = String::new();
    for item in &step.items {
        match item {
            Item::Text { value } => text.push_str(value),
            Item::Ingredient { index } => {
                if let Some(ingredient) = recipe.ingredients.get(*index) {
                    text.push_str(&ingredient.display_name());
                }
            }
            Item::Cookware { index } => {
                if let Some(cookware) = recipe.cookware.get(*index) {
                    text.push_str(cookware.display_name());
                }
            }
            Item::Timer { index } => {
                if let Some(timer) = recipe.timers.get(*index) {
                    match (&timer.name, &timer.quantity) {
                        (Some(name), _) => text.push_str(name),
                        (None, Some(quantity)) => text.push_str(&quantity.to_string()),
                        (None, None) => {}
                    }
                }
            }
            Item::InlineQuantity { index } => {
                if let Some(quantity) = recipe.inline_quantities.get(*index) {
                    text.push_str(&quantity.to_string());
                }
            }
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Checks that step `step` of section `section` exists in the recipe as
/// saved: a picture for a step that is not there would show on none, or on
/// whichever step later takes that place.
fn step_slot(
    entry: &RecipeEntry,
    section: usize,
    step: usize,
    path: &str,
) -> Result<StepSlot, ApiError> {
    let sections = recipe_steps(entry).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            json_error(format!("Failed to parse {path}: {e}")),
        )
    })?;
    let exists = sections
        .get(section - 1)
        .is_some_and(|found| step <= found.steps.len());
    if !exists {
        return Err((
            StatusCode::BAD_REQUEST,
            json_error(format!("{path} has no step {step} in section {section}")),
        ));
    }
    let before: usize = sections[..section - 1].iter().map(|s| s.steps.len()).sum();
    Ok(StepSlot {
        section,
        step,
        overall: before + step,
    })
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

/// `{ path, image, source, sections }`, read back from disk so it reports
/// what the recipe page will now show rather than what was just written.
///
/// `source` is `"metadata"` when the recipe's `image` (or `images`,
/// `picture`, `pictures`) metadata names the picture — that wins over any
/// file — `"file"` for a picture beside the recipe, and null for none.
/// `sections` lists every section that has steps, with each step's text and
/// picture, so the editor draws its step picker from one request.
fn picture_state(state: &AppState, path: &str) -> Result<Json<serde_json::Value>, ApiError> {
    let entry = find_recipe(state, path)?;
    let image = entry
        .title_image()
        .clone()
        .and_then(|image| url_of(state, image));
    let source = image.as_ref().map(|_| {
        if entry.metadata().image_url().is_some() {
            "metadata"
        } else {
            "file"
        }
    });

    // The title picture does not depend on the steps, so a recipe that does
    // not parse still reports it, with no steps to pick from.
    let sections = recipe_steps(&entry).unwrap_or_else(|e| {
        tracing::warn!("Listing no step pictures for {path}: {e}");
        Vec::new()
    });
    let mut before = 0;
    let mut listed = Vec::new();
    for (index, section) in sections.iter().enumerate() {
        if section.steps.is_empty() {
            continue;
        }
        let steps: Vec<_> = section
            .steps
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let image = crate::web::builders::step_image(&entry, index + 1, i + 1, before)
                    .and_then(|image| url_of(state, image.clone()));
                serde_json::json!({ "step": i + 1, "text": text, "image": image })
            })
            .collect();
        listed.push(serde_json::json!({
            "section": index + 1,
            "name": section.name,
            "steps": steps,
        }));
        before += section.steps.len();
    }

    Ok(Json(serde_json::json!({
        "path": path,
        "image": image,
        "source": source,
        "sections": listed,
    })))
}

/// `{ path, section, step, image, source }` for one step, read back from
/// disk like [`picture_state`]. `source` is `"file"` or null: no metadata
/// names a step's picture.
fn step_state(
    state: &AppState,
    path: &str,
    slot: &StepSlot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let entry = find_recipe(state, path)?;
    let image =
        crate::web::builders::step_image(&entry, slot.section, slot.step, slot.overall - slot.step)
            .and_then(|image| url_of(state, image.clone()));
    let source = image.as_ref().map(|_| "file");
    Ok(Json(serde_json::json!({
        "path": path,
        "section": slot.section,
        "step": slot.step,
        "image": image,
        "source": source,
    })))
}

fn url_of(state: &AppState, image: String) -> Option<String> {
    crate::web::builders::get_image_path(&state.base_path, &state.url_prefix, image)
}

fn prepare_error(e: PrepareError) -> ApiError {
    let (status, code) = match e {
        PrepareError::Heif => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "heif"),
        PrepareError::Unsupported => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported"),
        PrepareError::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid"),
        PrepareError::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "too_large"),
    };
    tracing::warn!("Refused a picture: {e}");
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
        tracing::error!("Failed to save picture {target}: {e}");
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
