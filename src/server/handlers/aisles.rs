//! The aisle configuration, read and changed from the web UI.
//!
//! Changes go through [`AisleFile`], which edits the lines concerned and
//! leaves the rest of `aisle.conf` — comments, blank lines — as it was. Every
//! change carries the revision of the file it was made against, so one made
//! from a page someone else's edit has outdated is refused rather than applied
//! to whatever now sits at those lines.

use super::common::{json_error, ApiError};
use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::aisle_file::{revision, AisleFile, EditError};
use crate::server::{activity, fs_atomic, AppState};
use crate::web::viewer::Viewer;

/// The aisles as the shopping list sees them.
#[derive(Debug, Serialize)]
pub struct Aisles {
    /// Whether there is an aisle file at all. Without one, everything lands
    /// in the shopping list's "other" group.
    pub configured: bool,
    pub path: Option<String>,
    /// Hands back with a change; see the module note.
    pub revision: Option<String>,
    pub aisles: Vec<Aisle>,
    /// What the parser had to skip: a duplicate name, an ingredient above
    /// the first aisle.
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Aisle {
    pub name: String,
    pub ingredients: Vec<Ingredient>,
}

#[derive(Debug, Serialize)]
pub struct Ingredient {
    /// The first is the name the shopping list shows; the others are the
    /// spellings it is merged with.
    pub names: Vec<String>,
}

impl Aisles {
    fn unconfigured() -> Self {
        Self {
            configured: false,
            path: None,
            revision: None,
            aisles: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn from_text(path: &Utf8PathBuf, text: &str) -> Self {
        let parsed = cooklang::aisle::parse_lenient(text);
        let aisles = parsed
            .output()
            .map(|conf| {
                conf.categories
                    .iter()
                    .map(|category| Aisle {
                        name: category.name.to_string(),
                        ingredients: category
                            .ingredients
                            .iter()
                            .map(|ingredient| Ingredient {
                                names: ingredient
                                    .names
                                    .iter()
                                    .filter(|name| !name.is_empty())
                                    .map(|name| name.to_string())
                                    .collect(),
                            })
                            .filter(|ingredient| !ingredient.names.is_empty())
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            configured: true,
            path: Some(path.to_string()),
            revision: Some(revision(text)),
            aisles,
            warnings: parsed
                .report()
                .warnings()
                .map(|warning| warning.to_string())
                .collect(),
        }
    }
}

/// The aisle file and its text, or `None` when there is none.
async fn read(state: &AppState) -> Result<Option<(Utf8PathBuf, String)>, ApiError> {
    let Some(path) = state.aisle_file() else {
        return Ok(None);
    };
    match tokio::fs::read_to_string(&path).await {
        Ok(text) => Ok(Some((path, text))),
        Err(e) => {
            tracing::error!("Failed to read aisle file {path}: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!("Failed to read the aisle file: {e}")),
            ))
        }
    }
}

async fn read_existing(state: &AppState) -> Result<(Utf8PathBuf, String), ApiError> {
    read(state).await?.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            json_error("There is no aisle file yet; create one first"),
        )
    })
}

async fn write(path: &Utf8PathBuf, text: String) -> Result<(), ApiError> {
    fs_atomic::write_replace_async(path.clone(), text.into_bytes())
        .await
        .map_err(|e| {
            tracing::error!("Failed to write aisle file {path}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!("Failed to write the aisle file: {e}")),
            )
        })
}

/// Refuses a change made against another version of the file. A change sent
/// without a revision is applied to whatever is there.
fn check_revision(text: &str, sent: Option<&str>) -> Result<(), ApiError> {
    let current = revision(text);
    match sent {
        Some(sent) if sent != current => Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "The aisle file changed since this page loaded it",
                "revision": current,
            })),
        )),
        _ => Ok(()),
    }
}

fn edit_error(error: EditError) -> ApiError {
    let status = match error {
        EditError::NotFound(_) => StatusCode::NOT_FOUND,
        EditError::InvalidName(_) | EditError::Taken(_) => StatusCode::BAD_REQUEST,
    };
    (status, json_error(error))
}

/// The aisle parser's complaint about `text`, with the line it is on.
fn parse_problem(text: &str) -> Option<String> {
    use cooklang::aisle::AisleConfError;
    let error = cooklang::aisle::parse(text).err()?;
    let at = match &error {
        AisleConfError::Parse { span, .. } => span.start(),
        AisleConfError::DuplicateCategory { second_span, .. }
        | AisleConfError::DuplicateIngredient { second_span, .. } => second_span.start(),
    };
    let line = text[..at.min(text.len())].matches('\n').count() + 1;
    Some(format!("Line {line}: {error}"))
}

/// The aisles, for the API and for the aisles page alike.
pub async fn load(state: &AppState) -> Result<Aisles, ApiError> {
    Ok(match read(state).await? {
        Some((path, text)) => Aisles::from_text(&path, &text),
        None => Aisles::unconfigured(),
    })
}

/// `GET /api/aisles`
pub async fn get_aisles(State(state): State<Arc<AppState>>) -> Result<Json<Aisles>, ApiError> {
    Ok(Json(load(&state).await?))
}

/// `POST /api/aisles`: starts an empty `config/aisle.conf` in the recipe
/// directory, where none was found.
pub async fn create_aisles(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
) -> Result<(StatusCode, Json<Aisles>), ApiError> {
    let _guard = state.aisle_lock.lock().await;
    if let Some(path) = state.aisle_file() {
        return Err((
            StatusCode::CONFLICT,
            json_error(format!("There is already an aisle file at {path}")),
        ));
    }

    let path = state.local_aisle_file();
    let created = async {
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
    }
    .await;
    if let Err(e) = created {
        tracing::error!("Failed to create aisle file {path}: {e}");
        let status = if e.kind() == std::io::ErrorKind::AlreadyExists {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        return Err((
            status,
            json_error(format!("Failed to create the aisle file: {e}")),
        ));
    }

    activity::record(
        &viewer,
        format_args!(
            "created the aisle file {}",
            activity::file(&state.base_path, &path)
        ),
    );
    Ok((StatusCode::CREATED, Json(Aisles::from_text(&path, ""))))
}

/// One change to the aisles. `name` is an aisle's name, or any one of an
/// ingredient's names, matched ignoring case.
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Change {
    /// An empty aisle, at `position` among the aisles or last.
    AddAisle {
        name: String,
        position: Option<usize>,
    },
    RenameAisle {
        name: String,
        new_name: String,
    },
    /// Removes the aisle and the ingredients listed under it.
    RemoveAisle {
        name: String,
    },
    /// Makes the aisle the one at `position`; the shopping list follows
    /// this order.
    MoveAisle {
        name: String,
        position: usize,
    },
    /// Lists an ingredient under `aisle`: the first of `names` is shown on
    /// the shopping list, the others are merged into it.
    AddIngredient {
        aisle: String,
        names: Vec<String>,
    },
    /// Replaces the ingredient's names, and moves it when `aisle` is given.
    UpdateIngredient {
        name: String,
        names: Vec<String>,
        aisle: Option<String>,
    },
    RemoveIngredient {
        name: String,
    },
}

impl Change {
    fn apply(&self, file: &mut AisleFile) -> Result<(), EditError> {
        match self {
            Change::AddAisle { name, position } => file.add_aisle(name, *position),
            Change::RenameAisle { name, new_name } => file.rename_aisle(name, new_name),
            Change::RemoveAisle { name } => file.remove_aisle(name),
            Change::MoveAisle { name, position } => file.move_aisle(name, *position),
            Change::AddIngredient { aisle, names } => file.add_ingredient(aisle, names),
            Change::UpdateIngredient { name, names, aisle } => {
                file.update_ingredient(name, names, aisle.as_deref())
            }
            Change::RemoveIngredient { name } => file.remove_ingredient(name),
        }
    }

    fn record(&self, viewer: &Viewer) {
        use activity::quoted;
        match self {
            Change::AddAisle { name, .. } => {
                activity::record(viewer, format_args!("added the aisle {}", quoted(name)))
            }
            Change::RenameAisle { name, new_name } => activity::record(
                viewer,
                format_args!("renamed the aisle {} to {}", quoted(name), quoted(new_name)),
            ),
            Change::RemoveAisle { name } => {
                activity::record(viewer, format_args!("removed the aisle {}", quoted(name)))
            }
            Change::MoveAisle { name, position } => activity::record(
                viewer,
                format_args!("moved the aisle {} to place {}", quoted(name), position + 1),
            ),
            Change::AddIngredient { aisle, names } => activity::record(
                viewer,
                format_args!(
                    "added {} to the aisle {}",
                    quoted(&names.join(" | ")),
                    quoted(aisle)
                ),
            ),
            Change::UpdateIngredient {
                name,
                names,
                aisle: Some(aisle),
            } => activity::record(
                viewer,
                format_args!(
                    "changed the aisle entry {} to {} in {}",
                    quoted(name),
                    quoted(&names.join(" | ")),
                    quoted(aisle)
                ),
            ),
            Change::UpdateIngredient { name, names, .. } => activity::record(
                viewer,
                format_args!(
                    "changed the aisle entry {} to {}",
                    quoted(name),
                    quoted(&names.join(" | "))
                ),
            ),
            Change::RemoveIngredient { name } => activity::record(
                viewer,
                format_args!("removed {} from the aisles", quoted(name)),
            ),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChangeRequest {
    pub revision: Option<String>,
    #[serde(flatten)]
    pub change: Change,
}

/// `POST /api/aisles/changes`
pub async fn change_aisles(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(request): Json<ChangeRequest>,
) -> Result<Json<Aisles>, ApiError> {
    let _guard = state.aisle_lock.lock().await;
    let (path, text) = read_existing(&state).await?;
    check_revision(&text, request.revision.as_deref())?;

    let mut file = AisleFile::parse(&text);
    request.change.apply(&mut file).map_err(edit_error)?;
    let changed = file.to_text();

    // The edits are written to keep the file readable; this is the backstop.
    // A file that was already broken is left to the text editor to mend.
    if parse_problem(&text).is_none() {
        if let Some(problem) = parse_problem(&changed) {
            tracing::error!("Refused an aisle edit that would break the file: {problem}");
            return Err((
                StatusCode::BAD_REQUEST,
                json_error(format!("That change would break the aisle file: {problem}")),
            ));
        }
    }

    write(&path, changed.clone()).await?;
    request.change.record(&viewer);
    Ok(Json(Aisles::from_text(&path, &changed)))
}

#[derive(Debug, Serialize)]
pub struct RawAisles {
    pub path: String,
    pub content: String,
    pub revision: String,
}

/// `GET /api/aisles/raw`: the file as written.
pub async fn get_raw_aisles(
    State(state): State<Arc<AppState>>,
) -> Result<Json<RawAisles>, ApiError> {
    let (path, text) = read_existing(&state).await?;
    Ok(Json(RawAisles {
        path: path.to_string(),
        revision: revision(&text),
        content: text,
    }))
}

#[derive(Debug, Deserialize)]
pub struct RawAislesUpdate {
    pub content: String,
    pub revision: Option<String>,
}

/// `PUT /api/aisles/raw`: replaces the whole file, once the aisle parser
/// accepts the new text.
pub async fn put_raw_aisles(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(update): Json<RawAislesUpdate>,
) -> Result<Json<Aisles>, ApiError> {
    let _guard = state.aisle_lock.lock().await;
    let (path, text) = read_existing(&state).await?;
    check_revision(&text, update.revision.as_deref())?;

    if let Some(problem) = parse_problem(&update.content) {
        return Err((StatusCode::BAD_REQUEST, json_error(problem)));
    }

    write(&path, update.content.clone()).await?;
    activity::record(
        &viewer,
        format_args!(
            "rewrote the aisle file {}",
            activity::file(&state.base_path, &path)
        ),
    );
    Ok(Json(Aisles::from_text(&path, &update.content)))
}

#[derive(Debug, Serialize)]
pub struct Uncategorized {
    pub total_recipes: usize,
    pub ingredients: Vec<UncategorizedIngredient>,
}

#[derive(Debug, Serialize)]
pub struct UncategorizedIngredient {
    pub name: String,
    pub recipes: Vec<RecipeLink>,
}

#[derive(Debug, Serialize)]
pub struct RecipeLink {
    /// Relative to the recipe directory, as the file is named.
    pub path: String,
    /// What follows `/recipe/` in the recipe page's address.
    pub link: String,
}

/// `GET /api/aisles/uncategorized`: the ingredients the recipes use that no
/// aisle lists — what `cook doctor aisle` reports. On the shopping list they
/// end up under "other".
pub async fn get_uncategorized(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Uncategorized>, ApiError> {
    let base = state.base_path.clone();
    let aisle = match state.aisle_file() {
        Some(path) => cookcli_core::source::ConfigSource::Path(path),
        None => cookcli_core::source::ConfigSource::None,
    };

    let outcome = tokio::task::spawn_blocking(move || {
        let ctx = cookcli_core::Context::discover(base).with_aisle(aisle);
        cookcli_core::doctor::aisle_coverage(&ctx, Default::default())
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error(format!("Failed to scan the recipes: {e}")),
        )
    })?
    .map_err(|e| {
        tracing::error!("Failed to check the recipes against the aisles: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error(format!("Failed to scan the recipes: {e}")),
        )
    })?;

    let coverage = outcome.value;
    Ok(Json(Uncategorized {
        total_recipes: coverage.total_recipes,
        ingredients: coverage
            .unknown_entries()
            .map(|ingredient| UncategorizedIngredient {
                name: ingredient.name.clone(),
                recipes: ingredient
                    .recipes
                    .iter()
                    .map(|path| RecipeLink {
                        path: path.to_string(),
                        link: path
                            .as_str()
                            .strip_suffix(".cook")
                            .unwrap_or(path.as_str())
                            .to_string(),
                    })
                    .collect(),
            })
            .collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_parse_problem_names_its_line() {
        assert_eq!(parse_problem("[a]\nx\n"), None);
        let problem = parse_problem("[a]\nx\n\n[b]\ny\nx\n").unwrap();
        assert!(problem.starts_with("Line 6:"), "{problem}");
        let problem = parse_problem("[a]\n[b]\n[a]\n").unwrap();
        assert!(problem.starts_with("Line 3:"), "{problem}");
    }

    #[test]
    fn a_change_reads_as_one_flat_object() {
        let request: ChangeRequest = serde_json::from_str(
            r#"{"revision": "abc", "action": "add_ingredient", "aisle": "dairy", "names": ["eggs"]}"#,
        )
        .unwrap();
        assert_eq!(request.revision.as_deref(), Some("abc"));
        assert!(matches!(request.change, Change::AddIngredient { .. }));
    }
}
