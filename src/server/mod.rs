// This file includes a substantial portion of code from
// https://github.com/Zheoni/cooklang-chef
//
// The original code is licensed under the MIT License, a copy of which
// is provided below in addition to our project's license.
//
//

// MIT License

// Copyright (c) 2023 Francisco J. Sanchez

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::Context;
use anyhow::{bail, Result};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderValue, Method, StatusCode, Uri},
    response::Response,
    routing::{get, post},
    Json, Router,
};

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use clap::Args;
use cooklang::{ingredient_list::IngredientList, CooklangParser};
use cooklang_fs::Error as CooklangError;

use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::SystemTime};

use tower_http::cors::CorsLayer;
use tracing::info;

mod async_index;

use self::async_index::AsyncFsIndex;

#[derive(Debug, Args)]
pub struct ServerArgs {
    /// Directory with recipes
    base_path: Option<Utf8PathBuf>,

    /// Allow external connections
    #[arg(long)]
    host: bool,

    /// Set http server port
    #[arg(long, default_value_t = 9080)]
    port: u16,

    /// Open browser on start
    // #[cfg(feature = "ui")]
    #[arg(long, default_value_t = false)]
    open: bool,
}

impl ServerArgs {
    pub fn get_base_path(&self) -> Option<Utf8PathBuf> {
        self.base_path.clone()
    }
}

#[tokio::main]
pub async fn run(ctx: Context, args: ServerArgs) -> Result<()> {
    let addr = if args.host {
        SocketAddr::from(([0, 0, 0, 0], args.port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], args.port))
    };

    info!("Listening on {addr}");

    // #[cfg(feature = "ui")]
    if args.open {
        let port = args.port;
        let url = format!("http://localhost:{port}");
        info!("Serving web UI on {url}");
        tokio::task::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Err(e) = open::that(url) {
                tracing::error!("Could not open the web browser: {e}");
            }
        });
    }

    let state = build_state(ctx, args)?;

    let app = Router::new().nest("/api", api(&state)?);

    let app = app.merge(ui::ui());

    let app = app.with_state(state).layer(
        CorsLayer::new()
            .allow_origin("*".parse::<HeaderValue>().unwrap())
            .allow_methods([Method::GET, Method::POST]),
    );

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    info!("Server stopped");

    Ok(())
}

fn build_state(ctx: Context, args: ServerArgs) -> Result<Arc<AppState>> {
    ctx.parser()?;
    let Context {
        parser,
        recipe_index,
        base_path,
        ..
    } = ctx;
    let parser = parser.into_inner().unwrap();

    let path = args.base_path.as_ref().unwrap_or(&base_path);

    if path.is_file() {
        bail!("{} is not a directory", path);
    }

    let recipe_index = AsyncFsIndex::new(recipe_index)?;

    Ok(Arc::new(AppState {
        parser,
        base_path: path.clone(),
        recipe_index,
    }))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    };

    info!("Stopping server");
}

// #[cfg(feature = "ui")]f
mod ui {
    use super::*;
    use rust_embed::RustEmbed;

    pub fn ui() -> Router<Arc<AppState>> {
        Router::new().fallback(static_ui)
    }

    #[derive(RustEmbed)]
    #[folder = "./ui/public/"]
    struct Assets;

    async fn static_ui(uri: Uri) -> impl axum::response::IntoResponse {
        use axum::response::IntoResponse;

        const INDEX_HTML: &str = "index.html";

        fn index_html() -> impl axum::response::IntoResponse {
            Assets::get(INDEX_HTML)
                .map(|content| {
                    let body = axum::body::boxed(axum::body::Full::from(content.data));
                    Response::builder()
                        .header(axum::http::header::CONTENT_TYPE, "text/html")
                        .body(body)
                        .unwrap()
                })
                .ok_or(StatusCode::NOT_FOUND)
        }

        let path = uri.path().trim_start_matches('/');

        if path.is_empty() || path == INDEX_HTML {
            return Ok(index_html().into_response());
        }

        match Assets::get(path) {
            Some(content) => {
                let body = axum::body::boxed(axum::body::Full::from(content.data));
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Ok(Response::builder()
                    .header(axum::http::header::CONTENT_TYPE, mime.as_ref())
                    .body(body)
                    .unwrap())
            }
            None => {
                if path.contains('.') {
                    return Err(StatusCode::NOT_FOUND);
                }
                Ok(index_html().into_response())
            }
        }
    }
}

pub struct AppState {
    parser: CooklangParser,
    base_path: Utf8PathBuf,
    recipe_index: AsyncFsIndex,
}

fn api(_state: &AppState) -> Result<Router<Arc<AppState>>> {
    let router = Router::new()
        .route("/shopping_list", post(shopping_list))
        .route("/recipes", get(all_recipes))
        .route("/recipes/*path", get(recipe));

    Ok(router)
}

fn check_path(p: &str) -> Result<(), StatusCode> {
    let path = Utf8Path::new(p);
    if !path
        .components()
        .all(|c| matches!(c, Utf8Component::Normal(_)))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

fn clean_path(p: &Utf8Path, base_path: &Utf8Path) -> Utf8PathBuf {
    let p = p
        .strip_prefix(base_path)
        .expect("dir entry path not relative to base path");
    #[cfg(windows)]
    let p = Utf8PathBuf::from(p.to_string().replace('\\', "/"));
    #[cfg(not(windows))]
    let p = p.to_path_buf();
    p
}

fn images(entry: &cooklang_fs::RecipeEntry, _base_path: &Utf8Path) -> Vec<cooklang_fs::Image> {
    entry.images().to_vec()
}

async fn shopping_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(payload): axum::extract::Json<Vec<String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut list = IngredientList::new();
    let converter = state.parser.converter();

    for entry in payload {
        let (name, servings) = entry
            .trim()
            .rsplit_once('*')
            .map(|(name, servings)| {
                let target = servings
                    .parse::<u32>()
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                Ok::<_, StatusCode>((name, Some(target)))
            })
            .unwrap_or(Ok((entry.as_str(), None)))?;

        let entry = state
            .recipe_index
            .get(name.to_string())
            .await
            .map_err(|_| {
                tracing::error!("Recipe not found: {name}");
                StatusCode::NOT_FOUND
            })?;

        let content = entry.read().map_err(|_| {
            tracing::error!("Failed to read recipe: {name}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let recipe = state
            .parser
            .parse(content.text())
            .into_output()
            .ok_or_else(|| {
                tracing::error!("Failed to parse recipe");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let recipe = if let Some(servings) = servings {
            recipe.scale(servings, converter)
        } else {
            recipe.default_scale()
        };

        list.add_recipe(&recipe, converter);
    }

    let categories = list.categorize(&Default::default());
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

async fn all_recipes(State(state): State<Arc<AppState>>) -> Result<Json<Vec<String>>, StatusCode> {
    let recipes = cooklang_fs::all_recipes(&state.base_path, 5) // TODO set as constant
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|e| {
            clean_path(e.path(), &state.base_path)
                .with_extension("")
                .into_string()
        })
        .collect();
    Ok(Json(recipes))
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(default)]
struct ColorConfig {
    color: bool,
}

#[derive(Deserialize)]
struct RecipeQuery {
    scale: Option<u32>,
    units: Option<cooklang::convert::System>,
}

async fn recipe(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<RecipeQuery>,
    Query(color): Query<ColorConfig>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    check_path(&path)?;

    let entry = state
        .recipe_index
        .get(path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let content = tokio::fs::read_to_string(&entry.path())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let times = get_times(entry.path()).await?;

    let recipe = state.parser.parse(&content).into_output().ok_or_else(|| {
        tracing::error!("Failed to parse recipe");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut recipe = if let Some(servings) = query.scale {
        recipe.scale(servings, state.parser.converter())
    } else {
        recipe.default_scale()
    };

    if let Some(system) = query.units {
        let errors = recipe.convert(system, state.parser.converter());
        if !errors.is_empty() {
            tracing::warn!("Errors converting units: {errors:?}");
        }
    }

    #[derive(Serialize)]
    struct ApiRecipe {
        #[serde(flatten)]
        recipe: cooklang::ScaledRecipe,
        grouped_ingredients: Vec<serde_json::Value>,
        timers_seconds: Vec<Option<cooklang::Value>>,
        filtered_metadata: Vec<serde_json::Value>,
        external_image: Option<String>,
    }

    let grouped_ingredients = recipe
        .group_ingredients(state.parser.converter())
        .into_iter()
        .map(|entry| {
            serde_json::json!({
                "index": entry.index,
                "quantity": entry.quantity.into_vec(),
                "outcome": entry.outcome
            })
        })
        .collect();

    let timers_seconds = recipe
        .timers
        .iter()
        .map(|t| {
            t.quantity.clone().and_then(|mut q| {
                if q.convert("s", state.parser.converter()).is_err() {
                    None
                } else {
                    Some(q.value().clone())
                }
            })
        })
        .collect();

    let filtered_metadata = recipe
        .metadata
        .map_filtered()
        .filter(|(k, _)| k.as_str() != Some("image"))
        .map(|e| serde_json::to_value(e).unwrap())
        .collect();

    let api_recipe = ApiRecipe {
        external_image: recipe
            .metadata
            .map
            .get("image")
            .and_then(|v| v.as_str().map(|s| s.to_owned())),
        recipe,
        grouped_ingredients,
        timers_seconds,
        filtered_metadata,
    };

    let value = serde_json::to_value(api_recipe).unwrap();
    let path = clean_path(entry.path(), &state.base_path);
    let report = Report::from_pass_result(Ok(value), path.as_str(), &content, color.color);
    let value = serde_json::json!({
        "recipe": report,
        "images": images(&entry, &state.base_path),
        "src_path": path,
        "modified": times.modified,
        "created": times.created,
    });

    Ok(Json(value))
}

struct Times {
    modified: Option<u64>,
    created: Option<u64>,
}
async fn get_times(path: &Utf8Path) -> Result<Times, StatusCode> {
    fn f(st: std::io::Result<SystemTime>) -> Option<u64> {
        st.ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let modified = f(metadata.modified());
    let created = f(metadata.created());
    Ok(Times { modified, created })
}

#[derive(Serialize)]
struct Report<T> {
    value: Option<T>,
    warnings: Vec<String>,
    errors: Vec<String>,
    fancy_report: Option<String>,
}

impl<T> Report<T> {
    fn from_pass_result(
        result: Result<T, CooklangError>,
        _file_name: &str,
        _source_code: &str,
        color: bool,
    ) -> Self {
        match result {
            Ok(value) => Report {
                value: Some(value),
                warnings: Vec::new(),
                errors: Vec::new(),
                fancy_report: None,
            },
            Err(e) => {
                let mut report = Report {
                    value: None,
                    warnings: Vec::new(),
                    errors: vec![e.to_string()],
                    fancy_report: None,
                };
                if color {
                    report.fancy_report = Some(e.to_string());
                }
                report
            }
        }
    }
}
