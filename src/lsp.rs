use anyhow::Result;
use clap::Args;
use cooklang_language_server::Backend;
use tower_lsp::{LspService, Server};
use tracing::info;

use crate::Context;

#[derive(Debug, Args)]
#[command()]
pub struct LspArgs {}

/// Start the Cooklang Language Server Protocol server.
///
/// The Context parameter is included for consistency with other commands
/// and future extensibility (e.g., accessing aisle.conf, pantry.conf, or base_path).
#[tokio::main]
pub async fn run(_ctx: &Context, _args: LspArgs) -> Result<()> {
    info!("Starting Cooklang LSP server");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);

    // Use tokio::select! to handle both the LSP server and shutdown signals
    tokio::select! {
        _ = Server::new(stdin, stdout, socket).serve(service) => {
            info!("LSP server stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal, stopping LSP server");
        }
    }

    Ok(())
}
