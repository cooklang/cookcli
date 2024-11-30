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
use cooklang::{error::PassResult, ingredient_list::IngredientList, CooklangParser, ScaledRecipe};

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
        recipe_index: recipe_index,
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

fn images(entry: &cooklang_fs::RecipeEntry, base_path: &Utf8Path) -> Vec<cooklang_fs::Image> {
    let mut images = entry.images();
    images
        .iter_mut()
        .for_each(|i| i.path = clean_path(&i.path, base_path));
    images
}

async fn shopping_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(payload): axum::extract::Json<Vec<String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let recipes: Vec<ScaledRecipe> = futures::future::join_all(payload.iter().map(|path| async {
        let entry = state.recipe_index.get(path.clone()).await.unwrap();

        let content = tokio::fs::read_to_string(&entry.path()).await.unwrap();

        let recipe = state
            .parser
            .parse(&content, entry.name())
            .into_output()
            .unwrap()
            .default_scale();

        recipe
    }))
    .await;

    let mut ingredient_list = IngredientList::new();

    for recipe in recipes {
        ingredient_list.add_recipe(&recipe, &state.parser.converter())
    }

    let result: Vec<serde_json::Value> = ingredient_list
        .iter()
        .map(|(name, quantity)| {
            // let grouped = flour.group_quantities(

            serde_json::json!({
                "name": name,
                // "quantity": quantity.to_string(), //doesn't work as it's private
                "quantity": quantity.total().into_vec(),
            })
        })
        .collect();

    let result = serde_json::json!({
        "INGREDIENTS": result
    });

    Ok(Json(result))
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

    let recipe = state
        .parser
        .parse(&content, entry.name())
        .try_map(|recipe| -> Result<_, StatusCode> {
            let mut scaled = if let Some(servings) = query.scale {
                recipe.scale(servings, state.parser.converter())
            } else {
                recipe.default_scale()
            };
            if let Some(system) = query.units {
                let errors = scaled.convert(system, state.parser.converter());
                if !errors.is_empty() {
                    tracing::warn!("Errors converting units: {errors:?}");
                }
            }
            Ok(scaled)
        })?
        .map(|r| {
            #[derive(Serialize)]
            struct ApiRecipe {
                #[serde(flatten)]
                recipe: cooklang::ScaledRecipe,
                grouped_ingredients: Vec<serde_json::Value>,
                timers_seconds: Vec<Option<cooklang::Value>>,
                filtered_metadata: Vec<serde_json::Value>,
                external_image: Option<String>,
            }

            let grouped_ingredients = r
                .group_ingredients(state.parser.converter())
                .into_iter()
                .map(|entry| {
                    serde_json::json!({
                        "index": entry.index,
                        "quantity": entry.quantity.total().into_vec(),
                        "outcome": entry.outcome
                    })
                })
                .collect();
            let timers_seconds = r
                .timers
                .iter()
                .map(|t| {
                    t.quantity.clone().and_then(|mut q| {
                        if q.convert("s", state.parser.converter()).is_err() {
                            None
                        } else {
                            Some(q.value)
                        }
                    })
                })
                .collect();
            let filtered_metadata = r
                .metadata
                .map_filtered()
                .into_iter()
                .filter(|(k, _)| k != "image")
                .map(|e| serde_json::to_value(e).unwrap())
                .collect();

            let api_recipe = ApiRecipe {
                external_image: r.metadata.map.get("image").cloned(),
                recipe: r,
                grouped_ingredients,
                timers_seconds,
                filtered_metadata,
            };

            serde_json::to_value(api_recipe).unwrap()
        });
    let path = clean_path(entry.path(), &state.base_path);
    let report = Report::from_pass_result(recipe, path.as_str(), &content, color.color);
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
    fn from_pass_result<E, W>(
        value: PassResult<T, E, W>,
        file_name: &str,
        source_code: &str,
        color: bool,
    ) -> Self
    where
        E: cooklang::error::RichError,
        W: cooklang::error::RichError,
    {
        let (value, w, e) = value.into_tuple();
        let warnings: Vec<_> = w.iter().map(|w| w.to_string()).collect();
        let errors: Vec<_> = e.iter().map(|e| e.to_string()).collect();
        let fancy_report = if warnings.is_empty() && errors.is_empty() {
            None
        } else {
            let mut buf = Vec::new();
            cooklang::error::Report::new(e, w)
                .write(file_name, source_code, false, color, &mut buf)
                .expect("Write fancy report");
            Some(String::from_utf8_lossy(&buf).into_owned())
        };
        Self {
            value,
            warnings,
            errors,
            fancy_report,
        }
    }
}
