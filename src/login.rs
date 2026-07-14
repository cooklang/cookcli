use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Args;
use tokio_util::sync::CancellationToken;

use crate::sync::{device_flow, SyncSession};
use crate::Context;

#[derive(Debug, Args)]
pub struct LoginArgs {}

pub fn run(_ctx: &Context, _args: LoginArgs) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(run_async())
}

async fn run_async() -> Result<()> {
    use std::io::{BufRead, Write};

    let session_path = crate::global_file_path("session.json")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(".cook-session.json"));

    if SyncSession::load(&session_path).ok().flatten().is_some() {
        println!("Already logged in. Run `cook logout` first if you want to switch accounts.");
        return Ok(());
    }

    let client = reqwest::Client::new();
    let name = device_flow::client_name("cli");
    let dc = device_flow::request_device_code(&client, &name).await?;

    println!();
    println!(
        "First open {} in any browser and enter this code:",
        dc.verification_uri
    );
    println!();
    println!("    {}", dc.user_code);
    println!();
    println!("(Press Enter to open it automatically, or Ctrl-C to abort.)");

    let cancel = CancellationToken::new();
    let cancel_for_signal = cancel.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        cancel_for_signal.cancel();
    });

    let stdin_task = tokio::task::spawn_blocking(|| {
        let stdin = std::io::stdin();
        let _ = stdin.lock().lines().next();
    });

    tokio::select! {
        _ = stdin_task => {}
        _ = cancel.cancelled() => {
            println!();
            anyhow::bail!("Cancelled.");
        }
    }

    if let Err(e) = open::that(&dc.verification_uri_complete) {
        eprintln!("Couldn't open browser automatically: {e}");
        eprintln!("Please visit the URL above manually.");
    }

    print!("Waiting for authorization");
    std::io::stdout().flush().ok();

    let expires_at = Instant::now() + Duration::from_secs(dc.expires_in);
    let interval = Duration::from_secs(dc.interval);

    let dot_handle = {
        let cancel = cancel.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        print!(".");
                        let _ = std::io::stdout().flush();
                    }
                }
            }
        })
    };

    let jwt = match device_flow::poll_for_token(
        &client,
        &dc.device_code,
        interval,
        expires_at,
        cancel.clone(),
    )
    .await
    {
        Ok(jwt) => jwt,
        Err(device_flow::DeviceFlowError::AccessDenied) => {
            cancel.cancel();
            dot_handle.abort();
            anyhow::bail!("Authorization denied.");
        }
        Err(device_flow::DeviceFlowError::Expired) => {
            cancel.cancel();
            dot_handle.abort();
            anyhow::bail!("Code expired - try `cook login` again.");
        }
        Err(device_flow::DeviceFlowError::Cancelled) => {
            dot_handle.abort();
            anyhow::bail!("Cancelled.");
        }
        Err(e) => {
            cancel.cancel();
            dot_handle.abort();
            anyhow::bail!("Login failed: {e}");
        }
    };

    cancel.cancel();
    dot_handle.abort();
    println!();

    let session = SyncSession::from_jwt(jwt)?;
    session.save(&session_path)?;

    let email = session
        .email
        .clone()
        .unwrap_or_else(|| "<unknown>".to_string());
    println!("Logged in as {email}");
    println!();
    println!("Note: if `cook server` is running, restart it to pick up the new session.");

    Ok(())
}
