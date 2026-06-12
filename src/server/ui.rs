use crate::server::language::FeatureFlags;
use crate::server::{templates::*, AppState};
use axum::{
    extract::{Extension, Host, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Form, Router,
};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use serde::Deserialize;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

fn error_page(
    lang: LanguageIdentifier,
    prefix: &str,
    msg: impl std::fmt::Display,
    features: FeatureFlags,
) -> axum::response::Response {
    let template = ErrorTemplate {
        active: String::new(),
        error_message: msg.to_string(),
        tr: Tr::new(lang),
        prefix: prefix.to_string(),
        static_mode: false,
        features,
    };
    template.into_response()
}

pub fn ui() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(recipes_page))
        .route("/directory/*path", get(recipes_directory))
        .route("/recipe/*path", get(recipe_page))
        .route("/edit/*path", get(edit_page))
        .route("/new", get(new_page).post(create_recipe))
        .route("/shopping-list", get(shopping_list_page))
        .route("/pantry", get(pantry_page))
        .route("/preferences", get(preferences_page))
}

async fn recipes_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> axum::response::Response {
    recipes_handler(state, None, lang, features).await
}

async fn recipes_directory(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> axum::response::Response {
    recipes_handler(state, Some(path), lang, features).await
}

async fn recipes_handler(
    state: Arc<AppState>,
    path: Option<String>,
    lang: LanguageIdentifier,
    features: FeatureFlags,
) -> axum::response::Response {
    let input = crate::server::builders::RecipesBuildInput {
        base_path: &state.base_path,
        url_prefix: &state.url_prefix,
        sub_path: path.as_deref(),
        lang: lang.clone(),
        static_mode: false,
        features,
    };
    match crate::server::builders::build_recipes_template(input) {
        Ok(template) => template.into_response(),
        Err(e) => {
            tracing::error!("Failed to build recipes template: {:?}", e);
            error_page(lang, &state.url_prefix, &e, features)
        }
    }
}

#[derive(Deserialize)]
struct RecipeQuery {
    scale: Option<f64>,
}

async fn recipe_page(
    Path(path): Path<String>,
    Query(query): Query<RecipeQuery>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> axum::response::Response {
    let scale = query.scale.unwrap_or(1.0);

    let input = crate::server::builders::RecipeBuildInput {
        base_path: &state.base_path,
        url_prefix: &state.url_prefix,
        recipe_path: &path,
        aisle_path: state.aisle_path.as_ref(),
        scale,
        lang: lang.clone(),
        static_mode: false,
        features,
    };

    match crate::server::builders::build_recipe_template(input) {
        Ok(crate::server::builders::RecipeBuildOutput::Recipe(template)) => {
            template.into_response()
        }
        Ok(crate::server::builders::RecipeBuildOutput::Menu(template)) => template.into_response(),
        Err(e) => {
            tracing::error!("Failed to build recipe template: {:?}", e);
            error_page(lang, &state.url_prefix, &e, features)
        }
    }
}

async fn edit_page(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> axum::response::Response {
    tracing::info!("Edit page requested for path: {}", path);

    // Validate path to prevent directory traversal
    let path_check = Utf8Path::new(&path);
    if !path_check
        .components()
        .all(|c| matches!(c, Utf8Component::Normal(_)))
    {
        tracing::error!("Invalid path: {path}");
        return error_page(
            lang,
            &state.url_prefix,
            format!("Invalid path: {path}"),
            features,
        );
    }

    let recipe_path = Utf8PathBuf::from(&path);

    // Find the actual file
    let entry = match cooklang_find::get_recipe(vec![&state.base_path], &recipe_path) {
        Ok(entry) => entry,
        Err(e) => {
            tracing::error!("Recipe not found: {path}");
            return error_page(
                lang,
                &state.url_prefix,
                format!("Recipe not found: {path}: {e}"),
                features,
            );
        }
    };

    let file_path = match entry.path() {
        Some(p) => p,
        None => {
            tracing::error!("Recipe has no file path: {path}");
            return error_page(
                lang,
                &state.url_prefix,
                format!("Recipe has no file path: {path}"),
                features,
            );
        }
    };

    // Read raw content
    let content = match tokio::fs::read_to_string(file_path).await {
        Ok(content) => content,
        Err(e) => {
            tracing::error!("Failed to read recipe file: {e}");
            return error_page(
                lang,
                &state.url_prefix,
                format!("Failed to read recipe file: {e}"),
                features,
            );
        }
    };

    // Get recipe name from path
    let recipe_name = path
        .split('/')
        .next_back()
        .unwrap_or(&path)
        .replace(".cook", "")
        .replace(".menu", "");

    let template = crate::server::templates::EditTemplate {
        active: "recipes".to_string(),
        recipe_name,
        recipe_path: path,
        content,
        base_path: state.base_path.to_string(),
        tr: crate::server::templates::Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        features,
    };

    template.into_response()
}

#[derive(Deserialize, Default)]
struct NewPageQuery {
    error: Option<String>,
    filename: Option<String>,
}

async fn new_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Query(query): Query<NewPageQuery>,
) -> impl askama_axum::IntoResponse {
    crate::server::templates::NewTemplate {
        active: "recipes".to_string(),
        tr: Tr::new(lang),
        error: query.error,
        filename: query.filename,
        prefix: state.url_prefix.clone(),
        static_mode: false,
        features,
    }
}

#[derive(Deserialize)]
struct NewRecipeForm {
    filename: String,
}

/// Helper to build redirect URL with error message
fn new_page_error(prefix: &str, error: &str, filename: &str) -> axum::response::Response {
    let encoded_error = urlencoding::encode(error);
    let encoded_filename = urlencoding::encode(filename);
    axum::response::Redirect::to(&format!(
        "{prefix}/new?error={}&filename={}",
        encoded_error, encoded_filename
    ))
    .into_response()
}

/// Validates that the request originated from the same host (CSRF protection)
fn validate_same_origin(headers: &HeaderMap, host: &str) -> bool {
    // Check Origin header first (preferred for CSRF protection)
    if let Some(origin) = headers.get(header::ORIGIN) {
        if let Ok(origin_str) = origin.to_str() {
            // Origin format is scheme://host[:port]
            if let Ok(origin_url) = url::Url::parse(origin_str) {
                if let Some(origin_host) = origin_url.host_str() {
                    let origin_with_port = if let Some(port) = origin_url.port() {
                        format!("{}:{}", origin_host, port)
                    } else {
                        origin_host.to_string()
                    };
                    return origin_with_port == host || origin_host == host;
                }
            }
        }
        return false;
    }

    // Fallback to Referer header (less reliable but better than nothing)
    if let Some(referer) = headers.get(header::REFERER) {
        if let Ok(referer_str) = referer.to_str() {
            if let Ok(referer_url) = url::Url::parse(referer_str) {
                if let Some(referer_host) = referer_url.host_str() {
                    let referer_with_port = if let Some(port) = referer_url.port() {
                        format!("{}:{}", referer_host, port)
                    } else {
                        referer_host.to_string()
                    };
                    return referer_with_port == host || referer_host == host;
                }
            }
        }
        return false;
    }

    // No Origin or Referer header - reject for safety
    // (though browsers should always send one for form submissions)
    false
}

async fn create_recipe(
    State(state): State<Arc<AppState>>,
    Host(host): Host,
    headers: HeaderMap,
    Form(form): Form<NewRecipeForm>,
) -> impl IntoResponse {
    // CSRF protection: verify request came from same origin
    if !validate_same_origin(&headers, &host) {
        tracing::warn!("CSRF validation failed for create_recipe request");
        return (StatusCode::FORBIDDEN, "Invalid request origin").into_response();
    }

    let original_filename = form.filename.clone();

    // Validate input before sanitization
    if form.filename.trim().is_empty() {
        return new_page_error(
            &state.url_prefix,
            "Recipe name cannot be empty",
            &original_filename,
        );
    }

    // Sanitize path - allow alphanumeric, space, dash, underscore, and forward slash
    let recipe_path: String = form
        .filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_' || *c == '/')
        .collect();

    // Clean up path: remove leading/trailing slashes, collapse multiple slashes
    let recipe_path = recipe_path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");

    if recipe_path.is_empty() {
        return new_page_error(
            &state.url_prefix,
            "Recipe name cannot be empty",
            &original_filename,
        );
    }

    let file_path = state.base_path.join(format!("{}.cook", recipe_path));

    // Security: Validate path structure before any filesystem operations
    // Check that the constructed path, when normalized, stays within base_path
    let base_path_clone = state.base_path.clone();
    let base_canonical =
        match tokio::task::spawn_blocking(move || base_path_clone.canonicalize_utf8()).await {
            Ok(Ok(p)) => p,
            _ => {
                return new_page_error(
                    &state.url_prefix,
                    "Internal error: invalid base path",
                    &original_filename,
                );
            }
        };

    // Validate parent path components don't escape base_path
    // We do this by checking the joined path doesn't contain .. after normalization
    let normalized_path = file_path.as_str().replace("\\", "/");
    if normalized_path.contains("/../") || normalized_path.ends_with("/..") {
        tracing::warn!("Path traversal attempt detected in: {}", recipe_path);
        return new_page_error(&state.url_prefix, "Invalid recipe path", &original_filename);
    }

    // For the file path, we check the parent directory
    if let Some(parent) = file_path.parent() {
        // Create parent directories if they don't exist
        if !parent.exists() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                tracing::error!("Failed to create directories: {}", e);
                return new_page_error(&state.url_prefix, "Failed to create directory. Check that the recipes folder has write permissions.", &original_filename);
            }
        }

        // Now verify the created parent is under base_path
        let parent_owned = parent.to_owned();
        match tokio::task::spawn_blocking(move || parent_owned.canonicalize_utf8()).await {
            Ok(Ok(parent_canonical)) => {
                if !parent_canonical.starts_with(&base_canonical) {
                    tracing::warn!(
                        "Path traversal attempt: {} not under {}",
                        parent_canonical,
                        base_canonical
                    );
                    // Clean up the created directory if it's outside base_path
                    let _ = tokio::fs::remove_dir_all(parent).await;
                    return new_page_error(
                        &state.url_prefix,
                        "Invalid recipe path",
                        &original_filename,
                    );
                }
            }
            _ => {
                return new_page_error(
                    &state.url_prefix,
                    "Invalid recipe path",
                    &original_filename,
                );
            }
        }
    }

    // Get the recipe name (last component of path) for the title
    let recipe_name = recipe_path
        .split('/')
        .next_back()
        .unwrap_or(&recipe_path)
        .replace(['-', '_'], " ");

    // Create recipe with YAML frontmatter
    let template = format!("---\ntitle: {}\n---\n\n", recipe_name);

    // Use OpenOptions with create_new to atomically check existence and create
    // This prevents TOCTOU race conditions
    use tokio::io::AsyncWriteExt;
    let file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true) // Fails if file exists - atomic check + create
        .open(&file_path)
        .await;

    match file {
        Ok(mut f) => {
            if let Err(e) = f.write_all(template.as_bytes()).await {
                tracing::error!("Failed to write recipe: {}", e);
                return new_page_error(
                    &state.url_prefix,
                    "Failed to write recipe file",
                    &original_filename,
                );
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return new_page_error(
                &state.url_prefix,
                "A recipe with this name already exists",
                &original_filename,
            );
        }
        Err(e) => {
            tracing::error!("Failed to create recipe file: {}", e);
            return new_page_error(
                &state.url_prefix,
                "Failed to create recipe file",
                &original_filename,
            );
        }
    }

    // Redirect to editor
    axum::response::Redirect::to(&format!("{}/edit/{}.cook", state.url_prefix, recipe_path))
        .into_response()
}

async fn shopping_list_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> impl askama_axum::IntoResponse {
    ShoppingListTemplate {
        active: "shopping".to_string(),
        tr: Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        features,
    }
}

async fn pantry_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    // Load pantry configuration
    let pantry_path = state.pantry_path.as_ref();

    let mut sections = Vec::new();

    if let Some(path) = pantry_path {
        if let Ok(content) = tokio::fs::read_to_string(path).await {
            let result = cooklang::pantry::parse_lenient(&content);

            if let Some(pantry_conf) = result.output() {
                // Convert pantry data to template format
                for (section_name, items) in &pantry_conf.sections {
                    let mut pantry_items = Vec::new();

                    for item in items {
                        pantry_items.push(crate::server::templates::PantryItem {
                            name: item.name().to_string(),
                            quantity: item.quantity().map(|q| q.to_string()),
                            bought: item.bought().map(|b| b.to_string()),
                            expire: item.expire().map(|e| e.to_string()),
                            low: item.low().map(|l| l.to_string()),
                        });
                    }

                    sections.push(crate::server::templates::PantrySection {
                        name: section_name.clone(),
                        items: pantry_items,
                    });
                }
            }
        }
    }

    Ok(PantryTemplate {
        active: "pantry".to_string(),
        configured: pantry_path.is_some(),
        sections,
        tr: Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        features,
    })
}

async fn preferences_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> impl askama_axum::IntoResponse {
    #[cfg(feature = "sync")]
    let (sync_logged_in, sync_email, sync_syncing) = state.sync_status().await;
    #[cfg(not(feature = "sync"))]
    let (sync_logged_in, sync_email, sync_syncing) = (false, None, false);

    PreferencesTemplate {
        active: "preferences".to_string(),
        aisle_path: state
            .aisle_path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "Not configured".to_string()),
        pantry_path: state
            .pantry_path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "Not configured".to_string()),
        base_path: state.base_path.to_string(),
        version: format!("{} - in food we trust", env!("CARGO_PKG_VERSION")),
        tr: Tr::new(lang),
        sync_enabled: cfg!(feature = "sync"),
        sync_logged_in,
        sync_email,
        sync_syncing,
        prefix: state.url_prefix.clone(),
        static_mode: false,
        features,
    }
}
