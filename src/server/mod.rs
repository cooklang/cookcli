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
use anyhow::{bail, Result};
use axum::{
    http::{HeaderValue, Method},
    routing::{get, post},
    Router,
};
use camino::Utf8PathBuf;
use clap::Args;
use cooklang::CooklangParser;
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing::info;

mod handlers;
mod ui;

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

pub struct AppState {
    parser: CooklangParser,
    base_path: Utf8PathBuf,
    aisle_path: Option<Utf8PathBuf>,
}

fn api(_state: &AppState) -> Result<Router<Arc<AppState>>> {
    let router = Router::new()
        .route("/shopping_list", post(handlers::shopping_list))
        .route("/recipes", get(handlers::all_recipes))
        .route("/recipes/{*path}", get(handlers::recipe))
        .route("/search", get(handlers::search));

    Ok(router)
}
