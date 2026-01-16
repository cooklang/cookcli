use anyhow::Result;
use clap::Args;
use cooklang_language_server::Backend;
use tower_lsp::{LspService, Server};
use tracing::{debug, info};

use crate::Context;

#[derive(Debug, Args)]
#[command()]
pub struct LspArgs {}

/// Start the Cooklang Language Server Protocol server.
///
/// Note: The LSP server receives workspace configuration from the editor during
/// the LSP initialization handshake. The editor sends workspace folders, and the
/// server loads aisle.conf from those locations. The Context paths logged below
/// are for debugging and future extensibility when the upstream cooklang-language-server
/// crate supports custom configuration paths.
#[tokio::main]
pub async fn run(ctx: &Context, _args: LspArgs) -> Result<()> {
    info!("Starting Cooklang LSP server");

    // Log available configuration paths for debugging
    // Currently the LSP server uses workspace paths from the editor,
    // but these could be used in the future for global config support
    debug!("Base path: {}", ctx.base_path());
    if let Some(aisle) = ctx.aisle() {
        debug!("Aisle config: {}", aisle);
    }
    if let Some(pantry) = ctx.pantry() {
        debug!("Pantry config: {}", pantry);
    }

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
