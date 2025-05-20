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

use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::SystemTime};

use tower_http::cors::CorsLayer;
use tracing::info;

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

    info!("Listening on http://{addr}");

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
    let aisle_path = ctx.aisle().clone();
    let Context {
        parser, base_path, ..
    } = ctx;
    let parser = parser.into_inner().unwrap();

    let path = args.base_path.as_ref().unwrap_or(&base_path);

    if path.is_file() {
        bail!("{} is not a directory", path);
    }

    Ok(Arc::new(AppState {
        parser,
        base_path: path.clone(),
        aisle_path,
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
    aisle_path: Option<Utf8PathBuf>,
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

// fn images(entry: &cooklang_find::RecipeEntry, _base_path: &Utf8Path) -> Vec<cooklang_find::Image> {
//     entry.images().to_vec()
// }

async fn shopping_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(payload): axum::extract::Json<Vec<String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut list = IngredientList::new();
    let converter = state.parser.converter();

    for entry in payload {
        let (name, scaling_factor) = entry
            .trim()
            .rsplit_once('@')
            .map(|(name, scaling_factor)| {
                let target = scaling_factor
                    .parse::<f64>()
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                Ok::<_, StatusCode>((name, target))
            })
            .unwrap_or(Ok((entry.as_str(), 1.0)))?;

        let entry = cooklang_find::get_recipe(vec![&state.base_path], &Utf8PathBuf::from(name))
            .map_err(|_| {
                tracing::error!("Recipe not found: {name}");
                StatusCode::NOT_FOUND
            })?;

        let recipe = entry.recipe(scaling_factor);

        list.add_recipe(&recipe, converter);
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

async fn all_recipes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let recipes = cooklang_find::build_tree(&state.base_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let recipes = serde_json::to_value(recipes).unwrap();

    Ok(Json(recipes))
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(default)]
struct ColorConfig {
    color: bool,
}

#[derive(Deserialize)]
struct RecipeQuery {
    scale: Option<f64>,
}

async fn recipe(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<RecipeQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    check_path(&path)?;

    let entry = cooklang_find::get_recipe(vec![&state.base_path], &Utf8PathBuf::from(path))
        .map_err(|_| StatusCode::NOT_FOUND)?;

    tracing::info!("Entry path: {:?}", entry.path());

    let times = get_times(entry.path().as_ref().unwrap()).await?;

    let recipe = entry.recipe(query.scale.unwrap_or(1.0));

    #[derive(Serialize)]
    struct ApiRecipe {
        #[serde(flatten)]
        recipe: Arc<cooklang::ScaledRecipe>,
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
                "quantities": entry.quantity.into_vec(),
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

    let path = clean_path(entry.path().as_ref().unwrap(), &state.base_path);
    let value = serde_json::json!({
        "recipe": api_recipe,
        // TODO: add images
        // "images": images(&entry, &state.base_path),
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
