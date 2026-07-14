use anyhow::Result;
use clap::Args;

use crate::sync::SyncSession;
use crate::Context;

#[derive(Debug, Args)]
pub struct LogoutArgs {}

pub fn run(_ctx: &Context, _args: LogoutArgs) -> Result<()> {
    let session_path = crate::global_file_path("session.json")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(".cook-session.json"));

    match SyncSession::load(&session_path) {
        Ok(Some(_)) => {
            SyncSession::delete(&session_path)?;
            println!("Logged out.");
        }
        _ => {
            println!("Not logged in.");
        }
    }
    Ok(())
}
