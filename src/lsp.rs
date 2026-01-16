use anyhow::Result;
use clap::Args;
use cooklang_language_server::Backend;
use tower_lsp::{LspService, Server};

#[derive(Debug, Args)]
#[command()]
pub struct LspArgs {}

pub fn run(_args: LspArgs) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run_server())
}

async fn run_server() -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
