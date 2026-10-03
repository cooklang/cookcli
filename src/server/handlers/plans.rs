//! `POST /api/plans/{*path}`: change a meal plan from its calendar, one line
//! at a time. The text operations live in [`crate::server::plan_text`].

use super::common::{json_error, recipe_file, RecipeFile};
use crate::server::plan_text::{EditError, PlanText};
use crate::server::{activity, AppState};
use crate::web::plan::{PlanEdit, PlanView};
use crate::web::viewer::Viewer;
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use camino::Utf8Path;
use chrono::NaiveDate;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::{Arc, LazyLock};
use unic_langid::LanguageIdentifier;

type ApiError = (StatusCode, Json<serde_json::Value>);

/// One change at a time: each reads the file, checks its version and writes
/// it back, so two at once could each undo the other.
static EDITS: LazyLock<tokio::sync::Mutex<()>> = LazyLock::new(|| tokio::sync::Mutex::new(()));

/// The version the page is sent and gives back: a hash of the file's text.
pub fn version(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A day's meal.
#[derive(Deserialize)]
pub struct Slot {
    date: String,
    meal: String,
}

/// A line of a day's meal, as the page showed it.
#[derive(Deserialize)]
pub struct Line {
    date: String,
    meal: String,
    /// Its place among the meal's lines, from 0.
    index: usize,
    /// Its text, without the bullet: checked against the file.
    text: String,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum PlanOp {
    /// Adds a recipe, by its path in the collection, to the end of a meal.
    Add {
        #[serde(flatten)]
        to: Slot,
        recipe: String,
        servings: Option<f64>,
    },
    Remove {
        #[serde(flatten)]
        line: Line,
    },
    Move {
        #[serde(flatten)]
        line: Line,
        to: Slot,
    },
    Copy {
        #[serde(flatten)]
        line: Line,
        to: Slot,
    },
}

#[derive(Deserialize)]
pub struct PlanEditRequest {
    /// The version of the text the page was showing.
    version: String,
    #[serde(flatten)]
    op: PlanOp,
}

fn bad_request(message: impl std::fmt::Display) -> ApiError {
    (StatusCode::BAD_REQUEST, json_error(message))
}

fn stale(current: &str) -> ApiError {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "error": "The plan changed since the page was loaded",
            "version": current,
        })),
    )
}

fn parse_date(date: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| bad_request(format!("Not a date: {date:?}")))
}

/// The Cooklang a recipe added from the page is written as: `@./Path{}` or
/// `@./Path{2%servings}`, as the editor's Add recipe writes it.
fn recipe_reference(
    base: &Utf8Path,
    recipe: &str,
    servings: Option<f64>,
) -> Result<String, ApiError> {
    let recipe = recipe.replace('\\', "/");
    match recipe_file(base, &recipe)? {
        RecipeFile::Existing(file) if file.extension() == Some("cook") => {}
        _ => return Err(bad_request(format!("No such recipe: {recipe:?}"))),
    }
    let name = recipe.strip_suffix(".cook").unwrap_or(&recipe);
    if name.contains(['{', '}', '@', '\n', '\r']) {
        return Err(bad_request(format!(
            "This recipe's name cannot be written in a plan: {recipe:?}"
        )));
    }
    let amount = match servings {
        None => String::new(),
        Some(servings) if servings.is_finite() && servings > 0.0 => format!(
            "{}%servings",
            crate::util::format::number::format_number(servings)
        ),
        Some(_) => return Err(bad_request("Servings must be above zero")),
    };
    Ok(format!("@./{name}{{{amount}}}"))
}

fn edit_error(error: EditError, current: &str) -> ApiError {
    match error {
        EditError::Stale => stale(current),
        EditError::BadMeal => bad_request("Not a meal name a plan can use"),
    }
}

pub async fn plan_edit(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Extension(lang): Extension<LanguageIdentifier>,
    Json(request): Json<PlanEditRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let file_path = match recipe_file(&state.base_path, &path)? {
        RecipeFile::Existing(file) if file.extension() == Some("menu") => file,
        RecipeFile::Existing(_) => return Err(bad_request("Not a meal plan")),
        RecipeFile::Missing(_) => {
            return Err((
                StatusCode::NOT_FOUND,
                json_error(format!("Meal plan not found: {path}")),
            ))
        }
    };

    let _one_at_a_time = EDITS.lock().await;

    let text = tokio::fs::read_to_string(&file_path).await.map_err(|e| {
        tracing::error!("Failed to read {file_path}: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error(format!("Failed to read the plan: {e}")),
        )
    })?;
    let current = version(&text);
    if request.version != current {
        return Err(stale(&current));
    }
    let mut plan = PlanText::parse(&text);
    if !crate::web::plan::is_plan(plan.section_names()) {
        return Err(bad_request("Not a meal plan"));
    }
    let meals = plan.meal_order();
    let fail = |error| edit_error(error, &current);
    let summary = match request.op {
        PlanOp::Add {
            to,
            recipe,
            servings,
        } => {
            let date = parse_date(&to.date)?;
            let item = recipe_reference(&state.base_path, &recipe, servings)?;
            plan.add(date, &to.meal, &item, &meals, &lang)
                .map_err(fail)?;
            format!(
                "added {} to {} on {date}",
                activity::quoted(&item),
                activity::quoted(&to.meal)
            )
        }
        PlanOp::Remove { line } => {
            let date = parse_date(&line.date)?;
            let item = plan
                .remove(date, &line.meal, line.index, &line.text)
                .map_err(fail)?;
            format!(
                "removed {} from {} on {date}",
                activity::quoted(&item),
                activity::quoted(&line.meal)
            )
        }
        PlanOp::Move { line, to } => {
            let (from, date) = (parse_date(&line.date)?, parse_date(&to.date)?);
            let item = plan
                .remove(from, &line.meal, line.index, &line.text)
                .map_err(fail)?;
            plan.add(date, &to.meal, &item, &meals, &lang)
                .map_err(fail)?;
            format!(
                "moved {} from {} on {from} to {} on {date}",
                activity::quoted(&item),
                activity::quoted(&line.meal),
                activity::quoted(&to.meal)
            )
        }
        PlanOp::Copy { line, to } => {
            let (from, date) = (parse_date(&line.date)?, parse_date(&to.date)?);
            let item = plan
                .read(from, &line.meal, line.index, &line.text)
                .map_err(fail)?;
            plan.add(date, &to.meal, &item, &meals, &lang)
                .map_err(fail)?;
            format!(
                "copied {} to {} on {date}",
                activity::quoted(&item),
                activity::quoted(&to.meal)
            )
        }
    };

    let changed = plan.render();
    write_atomically(&file_path, &changed).await?;
    activity::record(
        &viewer,
        format_args!(
            "updated {}: {summary}",
            activity::file(&state.base_path, &file_path)
        ),
    );

    Ok(Json(serde_json::json!({
        "status": "success",
        "version": version(&changed),
    })))
}

/// Writes `text` to a temporary file next to `file_path`, then renames it
/// over the plan, as saving from the editor does.
async fn write_atomically(file_path: &Utf8Path, text: &str) -> Result<(), ApiError> {
    let temp_path = file_path.with_extension("tmp");
    let failed = |e: std::io::Error| {
        tracing::error!("Failed to save {file_path}: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error(format!(
                "Failed to save the plan: {e}. Check that the recipes folder has write permissions."
            )),
        )
    };
    if let Err(e) = tokio::fs::write(&temp_path, text).await {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(failed(e));
    }
    if let Err(e) =
        crate::server::fs_atomic::rename_replace_async(temp_path.clone(), file_path.to_owned())
            .await
    {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(failed(e));
    }
    Ok(())
}

/// Gives `plan` what the page needs to change it: the file's version, and
/// each day's meal lines as the file has them, for the meals whose lines
/// line up with the card's.
pub async fn annotate(plan: &mut PlanView, base: &Utf8Path, path: &str, servings: Option<String>) {
    let Ok(RecipeFile::Existing(file_path)) = recipe_file(base, path) else {
        return;
    };
    if file_path.extension() != Some("menu") {
        return;
    }
    let Ok(text) = tokio::fs::read_to_string(&file_path).await else {
        return;
    };
    let parsed = PlanText::parse(&text);
    let mut edit = PlanEdit {
        path: file_path
            .strip_prefix(base)
            .unwrap_or(&file_path)
            .as_str()
            .replace('\\', "/"),
        version: version(&text),
        servings,
        ..PlanEdit::default()
    };
    for day in plan.weeks.iter().flatten().flatten() {
        let Ok(date) = NaiveDate::parse_from_str(&day.date, "%Y-%m-%d") else {
            continue;
        };
        for meal in &day.meals {
            let Some(name) = &meal.name else {
                continue;
            };
            if !edit.meals.contains(name) {
                edit.meals.push(name.clone());
            }
            let items = parsed.items(date, name);
            if items.len() == meal.lines.len() {
                edit.items.insert(format!("{}|{name}", day.date), items);
            }
        }
    }
    plan.edit = Some(edit);
}
