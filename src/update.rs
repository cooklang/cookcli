use anyhow::{Context, Result};
use clap::Args;
use self_update::backends::github::{ReleaseList, Update};
use self_update::cargo_crate_version;

#[derive(Debug, Args)]
pub struct UpdateArgs {
    #[arg(long, help = "Only check for updates without installing")]
    check_only: bool,

    #[arg(long, help = "Force update even if current version is latest")]
    force: bool,
}

pub fn run(args: UpdateArgs) -> Result<()> {
    let current_version = cargo_crate_version!();

    println!("Current version: {}", current_version);
    println!("Checking for updates...");

    let releases = ReleaseList::configure()
        .repo_owner("cooklang")
        .repo_name("cookcli")
        .build()?
        .fetch()?;

    let latest = releases.first().context("No releases found")?;

    let latest_version = latest.version.trim_start_matches('v');

    if !args.force && current_version >= latest_version {
        println!("You are already on the latest version!");
        return Ok(());
    }

    println!("New version available: {}", latest_version);

    if args.check_only {
        println!("Run 'cook update' to install the latest version.");
        return Ok(());
    }

    println!("Downloading and installing version {}...", latest_version);

    let target = get_target_triple();
    let binary_name = format!("cook-{}", target);

    let status = Update::configure()
        .repo_owner("cooklang")
        .repo_name("cookcli")
        .bin_name("cook")
        .target(&target)
        .identifier(&binary_name)
        .current_version(current_version)
        .no_confirm(true)
        .show_download_progress(true)
        .build()?
        .update()?;

    match status {
        self_update::Status::UpToDate(v) => {
            println!("Already up to date (version {})", v);
        }
        self_update::Status::Updated(v) => {
            println!("Successfully updated to version {}", v);

            // On macOS, try to remove quarantine attribute from the updated binary
            #[cfg(target_os = "macos")]
            {
                if let Ok(current_exe) = std::env::current_exe() {
                    let _ = std::process::Command::new("xattr")
                        .args(&["-d", "com.apple.quarantine"])
                        .arg(&current_exe)
                        .output();
                }
            }

            println!("Please restart cook to use the new version.");
        }
    }

    Ok(())
}

pub fn check_for_updates() -> Result<Option<String>> {
    let current_version = cargo_crate_version!();

    let releases = ReleaseList::configure()
        .repo_owner("cooklang")
        .repo_name("cookcli")
        .build()?
        .fetch()?;

    let latest = releases.first().context("No releases found")?;

    let latest_version = latest.version.trim_start_matches('v');

    if current_version < latest_version {
        Ok(Some(latest_version.to_string()))
    } else {
        Ok(None)
    }
}

fn get_target_triple() -> String {
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else if cfg!(target_arch = "x86") {
        "i686"
    } else {
        panic!("Unsupported architecture")
    };

    let os = if cfg!(target_os = "linux") {
        if cfg!(target_env = "musl") {
            "unknown-linux-musl"
        } else {
            "unknown-linux-gnu"
        }
    } else if cfg!(target_os = "macos") {
        "apple-darwin"
    } else if cfg!(target_os = "windows") {
        "pc-windows-msvc"
    } else if cfg!(target_os = "freebsd") {
        "unknown-freebsd"
    } else {
        panic!("Unsupported operating system")
    };

    let special_arm = if cfg!(target_arch = "arm") && cfg!(target_os = "linux") {
        "eabihf"
    } else {
        ""
    };

    format!("{}-{}{}", arch, os, special_arm)
}
