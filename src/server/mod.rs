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

use crate::util::extract_ingredients;
use crate::util::resolve_to_absolute_path;
use crate::Context;
use anyhow::{bail, Result};
use axum::{
    body::Body,
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
use std::collections::BTreeMap;
use std::{net::SocketAddr, sync::Arc};
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

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app)
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
    let absolute_path = resolve_to_absolute_path(path)?;

    if absolute_path.is_file() {
        bail!("Base path {} is not a directory", absolute_path);
    }

    tracing::info!("Using absolute base path: {:?}", absolute_path);

    Ok(Arc::new(AppState {
        parser,
        base_path: absolute_path,
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
    #[folder = "ui/public/"]
    struct Assets;

    async fn static_ui(uri: Uri) -> impl axum::response::IntoResponse {
        use axum::response::IntoResponse;

        const INDEX_HTML: &str = "index.html";

        fn index_html() -> impl axum::response::IntoResponse {
            Assets::get(INDEX_HTML)
                .map(|content| {
                    let body = Body::from(content.data);
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
                let body = Body::from(content.data);
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
        .route("/recipes/{*path}", get(recipe));

    Ok(router)
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

async fn shopping_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(payload): axum::extract::Json<Vec<String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut list = IngredientList::new();
    let mut seen = BTreeMap::new();

    for entry in payload {
        extract_ingredients(
            &entry,
            &mut list,
            &mut seen,
            &state.base_path,
            state.parser.converter(),
            false,
        )
        .map_err(|e| {
            tracing::error!("Error processing recipe: {}", e);
            StatusCode::BAD_REQUEST
        })?;
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
