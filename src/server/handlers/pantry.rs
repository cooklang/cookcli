use super::common::{json_error, ApiError};
use axum::{
    extract::{Extension, Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use camino::Utf8PathBuf;
use chrono::prelude::*;
use cookcli_core::{
    pantry::{self as core_pantry, PantryContents},
    ConfigSource, CoreError, Outcome,
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;

use crate::server::aisle_file::revision;
use crate::server::{activity, AppState};
use crate::web::viewer::Viewer;

/// Load and parse the pantry file asynchronously.
async fn load_pantry(
    state: &AppState,
) -> Result<cooklang::pantry::PantryConf, (StatusCode, Json<serde_json::Value>)> {
    let pantry_path = get_pantry_path(state)?;

    let content = tokio::fs::read_to_string(pantry_path.as_std_path())
        .await
        .map_err(|e| {
            tracing::error!("Failed to read pantry file: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!("Failed to read pantry file: {e}")),
            )
        })?;

    let result = cooklang::pantry::parse_lenient(&content);
    result.output().cloned().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            json_error("Failed to parse pantry configuration"),
        )
    })
}

fn get_pantry_path(
    state: &AppState,
) -> Result<&Utf8PathBuf, (StatusCode, Json<serde_json::Value>)> {
    state.pantry_path.as_ref().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            json_error("Pantry configuration not found"),
        )
    })
}

#[derive(Debug, Deserialize)]
pub struct AddPantryItem {
    pub section: String,
    pub name: String,
    pub quantity: Option<String>,
    pub bought: Option<String>,
    pub expire: Option<String>,
    pub low: Option<String>,
}

impl AddPantryItem {
    /// The item with the spaces typed around each field taken off, and blank
    /// attributes left unset.
    ///
    /// The pantry is matched against recipe ingredients by name, so ` Milk`
    /// kept as typed is never taken off a shopping list that asks for `milk`,
    /// and a quantity of ` unlim` is not unlimited.
    ///
    /// # Errors
    ///
    /// When the section or the name is blank.
    fn trimmed(self) -> Result<Self, &'static str> {
        let section = self.section.trim().to_string();
        let name = self.name.trim().to_string();
        if section.is_empty() {
            return Err("Section cannot be empty");
        }
        if name.is_empty() {
            return Err("Item name cannot be empty");
        }

        let attribute = |value: Option<String>| {
            value
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Ok(Self {
            section,
            name,
            quantity: attribute(self.quantity),
            bought: attribute(self.bought),
            expire: attribute(self.expire),
            low: attribute(self.low),
        })
    }
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

/// Apply a `cookcli-core` change to the server's pantry file.
///
/// Core edits the file as a TOML document, so the comments and layout written
/// by hand — or on the page's Text tab — survive a change made from the page,
/// and it writes atomically and through a symlink. Changes are made one at a
/// time: two requests that each read the file and write back their own edit
/// would otherwise lose one of them.
async fn change_pantry<F>(state: &AppState, change: F) -> Result<(), ApiError>
where
    F: FnOnce(&cookcli_core::Context) -> Result<Outcome<PantryContents>, CoreError>
        + Send
        + 'static,
{
    let path = get_pantry_path(state)?.clone();
    let _guard = state.pantry_lock.lock().await;
    let ctx =
        cookcli_core::Context::new(state.base_path.clone()).with_pantry(ConfigSource::Path(path));
    tokio::task::spawn_blocking(move || change(&ctx))
        .await
        .map_err(|e| {
            tracing::error!("Pantry change did not finish: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error("The pantry change did not finish"),
            )
        })?
        .map(drop)
        .map_err(core_error)
}

/// A core error as the API reports it, without the absolute path some of them
/// carry.
fn core_error(error: CoreError) -> ApiError {
    match error {
        CoreError::PantryEdit { message } => {
            let status = if message.contains(" not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            (status, json_error(sentence(&message)))
        }
        CoreError::Config { message, .. } => (
            StatusCode::BAD_REQUEST,
            json_error(format!("Not a valid pantry: {message}")),
        ),
        CoreError::MissingConfig { .. } => (
            StatusCode::NOT_FOUND,
            json_error("Pantry configuration not found"),
        ),
        CoreError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound => (
            StatusCode::NOT_FOUND,
            json_error("Pantry configuration not found"),
        ),
        other => {
            tracing::error!("Pantry change failed: {other}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error("Failed to write the pantry file"),
            )
        }
    }
}

/// `message` with its first letter capitalised, for display as it stands.
fn sentence(message: &str) -> String {
    let mut chars = message.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

pub async fn add_item(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(item): Json<AddPantryItem>,
) -> Result<impl IntoResponse, ApiError> {
    let item = item
        .trimmed()
        .map_err(|e| (StatusCode::BAD_REQUEST, json_error(e)))?;
    let (section, name) = (item.section.clone(), item.name.clone());

    let request = core_pantry::AddRequest {
        section: item.section,
        name: item.name,
        quantity: item.quantity,
        bought: item.bought,
        expire: item.expire,
        low: item.low,
    };
    change_pantry(&state, move |ctx| core_pantry::add(ctx, request)).await?;

    activity::record(
        &viewer,
        format_args!(
            "added {} to the {} section of the pantry",
            activity::quoted(&name),
            activity::quoted(&section)
        ),
    );

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Added {name} to {section}"),
    }))
}

pub async fn remove_item(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Path((section, name)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let request = core_pantry::RemoveRequest {
        section: section.clone(),
        name: name.clone(),
    };
    change_pantry(&state, move |ctx| core_pantry::remove(ctx, request)).await?;

    activity::record(
        &viewer,
        format_args!(
            "removed {} from the {} section of the pantry",
            activity::quoted(&name),
            activity::quoted(&section)
        ),
    );

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Removed {name} from {section}"),
    }))
}

pub async fn update_item(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Path((section, name)): Path<(String, String)>,
    Json(update): Json<UpdatePantryItem>,
) -> Result<impl IntoResponse, ApiError> {
    // The edit dialog sends only the fields it changed, so saving it untouched
    // sends nothing — which is not worth an error, nor a write.
    let unchanged = update.quantity.is_none()
        && update.bought.is_none()
        && update.expire.is_none()
        && update.low.is_none();
    if !unchanged {
        let request = core_pantry::UpdateRequest {
            section: section.clone(),
            name: name.clone(),
            quantity: update.quantity,
            bought: update.bought,
            expire: update.expire,
            low: update.low,
        };
        change_pantry(&state, move |ctx| core_pantry::update(ctx, request)).await?;

        activity::record(
            &viewer,
            format_args!(
                "updated {} in the {} section of the pantry",
                activity::quoted(&name),
                activity::quoted(&section)
            ),
        );
    }

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Updated {name} in {section}"),
    }))
}

#[derive(Debug, Deserialize)]
pub struct RenamePantrySection {
    pub section: String,
    pub new_name: String,
}

/// `POST /api/pantry/rename`: gives a section a new name, keeping its place,
/// its items and the comments around it.
pub async fn rename_section(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(rename): Json<RenamePantrySection>,
) -> Result<Json<ApiResponse>, ApiError> {
    let section = rename.section;
    let new_name = rename.new_name.trim().to_string();

    let request = core_pantry::RenameSectionRequest {
        section: section.clone(),
        new_name: new_name.clone(),
    };
    change_pantry(&state, move |ctx| core_pantry::rename_section(ctx, request)).await?;

    activity::record(
        &viewer,
        format_args!(
            "renamed the pantry section {} to {}",
            activity::quoted(&section),
            activity::quoted(&new_name)
        ),
    );

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Renamed {section} to {new_name}"),
    }))
}

/// The pantry file as written, and the revision a change to it must name.
#[derive(Debug, Serialize)]
pub struct RawPantry {
    pub content: String,
    pub revision: String,
}

impl RawPantry {
    fn new(content: String) -> Self {
        Self {
            revision: revision(&content),
            content,
        }
    }
}

async fn read_raw(state: &AppState) -> Result<String, ApiError> {
    let path = get_pantry_path(state)?;
    tokio::fs::read_to_string(path.as_std_path())
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                return (
                    StatusCode::NOT_FOUND,
                    json_error("Pantry configuration not found"),
                );
            }
            tracing::error!("Failed to read pantry file {path}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error("Failed to read the pantry file"),
            )
        })
}

/// `GET /api/pantry/raw`: the file as written.
pub async fn get_raw_pantry(
    State(state): State<Arc<AppState>>,
) -> Result<Json<RawPantry>, ApiError> {
    Ok(Json(RawPantry::new(read_raw(&state).await?)))
}

#[derive(Debug, Deserialize)]
pub struct RawPantryUpdate {
    pub content: String,
    pub revision: Option<String>,
}

/// `PUT /api/pantry/raw`: replaces the whole file, once it reads as a pantry.
///
/// A change made against another version of the file is refused with `409` and
/// the current revision; one sent without a revision replaces whatever is
/// there.
pub async fn put_raw_pantry(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(update): Json<RawPantryUpdate>,
) -> Result<Json<RawPantry>, ApiError> {
    let path = get_pantry_path(&state)?.clone();
    let _guard = state.pantry_lock.lock().await;

    let current = revision(&read_raw(&state).await?);
    if update
        .revision
        .as_deref()
        .is_some_and(|sent| sent != current)
    {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "The pantry file changed since this page loaded it",
                "revision": current,
            })),
        ));
    }

    let ctx = cookcli_core::Context::new(state.base_path.clone())
        .with_pantry(ConfigSource::Path(path.clone()));
    let content = update.content;
    let written = content.clone();
    tokio::task::spawn_blocking(move || core_pantry::replace(&ctx, &written))
        .await
        .map_err(|e| {
            tracing::error!("Pantry file write did not finish: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error("The pantry change did not finish"),
            )
        })?
        .map_err(core_error)?;

    activity::record(
        &viewer,
        format_args!(
            "rewrote the pantry file {}",
            activity::file(&state.base_path, &path)
        ),
    );
    Ok(Json(RawPantry::new(content)))
}

pub async fn get_pantry(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let pantry_conf = load_pantry(&state).await?;
    Ok(Json(pantry_conf))
}

#[derive(Debug, Deserialize)]
pub struct ExpiringQuery {
    pub days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ExpiringItemResponse {
    pub section: String,
    pub name: String,
    pub expire: String,
    pub days_remaining: i64,
}

#[derive(Debug, Serialize)]
pub struct DepletedItemResponse {
    pub section: String,
    pub name: String,
    pub low: Option<String>,
}

pub async fn get_expiring(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ExpiringQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let days = query.days.unwrap_or(7);
    if days < 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            json_error("days must be non-negative"),
        ));
    }

    let pantry_conf = load_pantry(&state).await?;
    let today = Local::now().date_naive();
    // A `days` big enough to run off the end of the calendar means every date
    // is within it. Saturating rather than panicking matters here more than in
    // the CLI: `days` comes straight off the query string, so an out-of-range
    // value would take down the request handler.
    let threshold = today
        .checked_add_signed(chrono::Duration::days(days))
        .unwrap_or(NaiveDate::MAX);

    let mut items = Vec::new();

    for (section, section_items) in &pantry_conf.sections {
        for item in section_items {
            if let Some(expire_str) = item.expire() {
                if let Some(date) = parse_date(expire_str) {
                    if date <= threshold {
                        let days_remaining = (date - today).num_days();
                        items.push(ExpiringItemResponse {
                            section: section.clone(),
                            name: item.name().to_string(),
                            expire: date.format("%Y-%m-%d").to_string(),
                            days_remaining,
                        });
                    }
                }
            }
        }
    }

    // Sort by days_remaining so most urgent items come first
    items.sort_by_key(|item| item.days_remaining);

    Ok(Json(items))
}

pub async fn get_depleted(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let pantry_conf = load_pantry(&state).await?;
    let mut items = Vec::new();

    for (section, section_items) in &pantry_conf.sections {
        for item in section_items {
            if item.is_low() {
                items.push(DepletedItemResponse {
                    section: section.clone(),
                    name: item.name().to_string(),
                    low: item.low().map(|l| l.to_string()),
                });
            }
        }
    }

    Ok(Json(items))
}

/// Parse a date string supporting multiple formats
pub fn parse_date(date_str: &str) -> Option<NaiveDate> {
    let formats = [
        "%Y-%m-%d", "%d.%m.%Y", "%d/%m/%Y", "%m/%d/%Y", "%Y.%m.%d", "%d-%m-%Y",
    ];

    for format in &formats {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
            return Some(date);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::AddPantryItem;

    fn item(section: &str, name: &str, quantity: Option<&str>) -> AddPantryItem {
        AddPantryItem {
            section: section.to_string(),
            name: name.to_string(),
            quantity: quantity.map(str::to_string),
            bought: None,
            expire: Some(" 2026-01-01 ".to_string()),
            low: Some("   ".to_string()),
        }
    }

    #[test]
    fn a_new_item_is_trimmed() {
        let added = item(" fridge ", " Milk ", Some(" unlim "))
            .trimmed()
            .unwrap();
        assert_eq!(added.section, "fridge");
        assert_eq!(added.name, "Milk");
        assert_eq!(added.quantity.as_deref(), Some("unlim"));
        assert_eq!(added.expire.as_deref(), Some("2026-01-01"));
        // A blank attribute is not an attribute.
        assert_eq!(added.low, None);
    }

    #[test]
    fn a_blank_name_or_section_is_refused() {
        assert!(item("fridge", "   ", None).trimmed().is_err());
        assert!(item(" ", "Milk", None).trimmed().is_err());
    }
}
