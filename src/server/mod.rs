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

use crate::util::resolve_to_absolute_path;
use crate::Context;
use anyhow::{bail, Context as _, Result};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path},
    http::{header, HeaderValue, Method, Response, StatusCode},
    routing::{get, post},
    Router,
};
use camino::Utf8PathBuf;
use clap::Args;
use rust_embed::RustEmbed;
use std::{net::IpAddr, net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::{error, info};

mod handlers;
mod i18n;
mod language;
mod lsp_bridge;
mod shopping_list_store;
mod templates;
mod ui;

// Embed static files at compile time
#[derive(RustEmbed)]
#[folder = "static/"]
struct StaticFiles;

#[derive(Debug, Args)]
pub struct ServerArgs {
    /// Root directory containing your recipe files
    ///
    /// The server will recursively scan this directory for .cook files
    /// and make them available through the web interface. Defaults to
    /// the current directory if not specified.
    #[arg(value_hint = clap::ValueHint::DirPath)]
    base_path: Option<Utf8PathBuf>,

    /// Allow connections from external hosts (not just localhost)
    ///
    /// By default, the server only accepts connections from localhost
    /// for security. Use this flag to allow access from other devices
    /// on your network. If an IP address is provided the server will
    /// only listen on that address. Be cautious when using this flag
    /// on public networks.
    #[arg(long, num_args = 0..=1, value_name = "ADDRESS")]
    host: Option<Option<IpAddr>>,

    /// Port number for the HTTP server
    ///
    /// The server will listen on this port. Make sure the port is not
    /// already in use by another application.
    #[arg(short = 'p', long, default_value_t = 9080)]
    port: u16,

    /// Automatically open the web interface in your default browser
    ///
    /// When enabled, the server will launch your default web browser
    /// and navigate to the server URL after startup.
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
    let addr = match args.host {
        Some(Some(addr)) => addr,
        Some(None) => "::".parse()?,
        None => [127, 0, 0, 1].into(),
    };
    let addr = SocketAddr::from((addr, args.port));

    println!("Listening on http://{addr}");

    // #[cfg(feature = "ui")]
    if args.open {
        let url = format!("http://{addr}");
        println!("Serving Web UI on {url}");
        tokio::task::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Err(e) = open::that(url) {
                tracing::error!("Could not open the web browser: {e}");
            }
        });
    }

    let state = build_state(ctx, args)?;

    println!("Serving recipe files from: {:?}", &state.base_path);

    // Maximum request body size: 1MB (reasonable for recipe files)
    const MAX_BODY_SIZE: usize = 1024 * 1024;

    let app = Router::new()
        .nest("/api", api(&state)?)
        .merge(ui::ui())
        .route("/static/*file", get(serve_static))
        .nest_service("/api/static", ServeDir::new(&state.base_path));

    let app = app
        .with_state(state)
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(axum::middleware::from_fn(language::language_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]),
        );

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                error!("Port {} is already in use. Please stop the existing server or use a different port with --port", addr.port());
                return Err(anyhow::anyhow!("Port {} is already in use", addr.port()));
            } else {
                error!("Failed to bind to {}: {}", addr, e);
                return Err(anyhow::anyhow!("Failed to bind to {}: {}", addr, e));
            }
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server error")?;

    info!("Server stopped");

    Ok(())
}

fn build_state(ctx: Context, args: ServerArgs) -> Result<Arc<AppState>> {
    let Context { base_path } = ctx;

    let path = args.base_path.as_ref().unwrap_or(&base_path);
    let absolute_path = resolve_to_absolute_path(path)?;

    if absolute_path.is_file() {
        bail!("Base path {} is not a directory", absolute_path);
    }

    tracing::info!("Using absolute base path: {:?}", absolute_path);

    // Create a new Context with the actual base path to properly search for config files
    let server_ctx = Context::new(absolute_path.clone());
    let aisle_path = server_ctx.aisle();
    let pantry_path = server_ctx.pantry();

    tracing::info!("Aisle configuration: {:?}", aisle_path);
    tracing::info!("Pantry configuration: {:?}", pantry_path);

    Ok(Arc::new(AppState {
        base_path: absolute_path,
        aisle_path,
        pantry_path,
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

pub struct AppState {
    pub base_path: Utf8PathBuf,
    pub aisle_path: Option<Utf8PathBuf>,
    pub pantry_path: Option<Utf8PathBuf>,
}

fn api(_state: &AppState) -> Result<Router<Arc<AppState>>> {
    let router = Router::new()
        .route("/shopping_list", post(handlers::shopping_list))
        .route(
            "/shopping_list/items",
            get(handlers::get_shopping_list_items),
        )
        .route("/shopping_list/add", post(handlers::add_to_shopping_list))
        .route(
            "/shopping_list/remove",
            post(handlers::remove_from_shopping_list),
        )
        .route("/shopping_list/clear", post(handlers::clear_shopping_list))
        .route("/pantry", get(handlers::get_pantry))
        .route("/pantry/add", post(handlers::add_pantry_item))
        .route(
            "/pantry/:section/:name",
            axum::routing::delete(handlers::remove_pantry_item),
        )
        .route(
            "/pantry/:section/:name",
            axum::routing::put(handlers::update_pantry_item),
        )
        .route("/recipes", get(handlers::all_recipes))
        .route("/recipes/raw/*path", get(handlers::recipe_raw)) // More specific route must come first
        .route(
            "/recipes/*path",
            get(handlers::recipe)
                .put(handlers::recipe_save)
                .delete(handlers::recipe_delete),
        )
        .route("/search", get(handlers::search))
        .route("/reload", get(handlers::reload).post(handlers::reload))
        .route("/ws/lsp", get(lsp_bridge::lsp_websocket));

    Ok(router)
}

async fn serve_static(Path(path): Path<String>) -> impl axum::response::IntoResponse {
    let path = path.trim_start_matches('/');

    StaticFiles::get(path)
        .map(|content| {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data))
                .unwrap()
        })
        .unwrap_or_else(|| {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("404 Not Found"))
                .unwrap()
        })
}
