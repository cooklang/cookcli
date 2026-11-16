use anyhow::Result;
use clap::Args;

use crate::sync::SyncSession;
use crate::Context;

#[derive(Debug, Args)]
pub struct LogoutArgs {}

pub fn run(_ctx: &Context, _args: LogoutArgs) -> Result<()> {
    let session_path = cookcli_core::global_config_path("session.json")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(".cook-session.json"));

    match SyncSession::load(&session_path) {
        Ok(Some(_)) => {
            SyncSession::delete(&session_path)?;
            println!("Logged out.");
        }
        // `load` only fails when the file exists but cannot be read,
        // parsed, or turns out to hold a malformed token. Leaving it in
        // place after "Not logged in." meant `cook logout` could never
        // remove a corrupt session file, so every later command kept
        // tripping over it.
        Err(e) => {
            tracing::warn!("Removing unreadable session file: {e:#}");
            SyncSession::delete(&session_path)?;
            println!("Logged out.");
        }
        _ => {
            println!("Not logged in.");
        }
    }
    Ok(())
}
