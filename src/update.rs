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

    println!("Current version: {current_version}");
    println!("Checking for updates...");

    let releases = ReleaseList::configure()
        .repo_owner("cooklang")
        .repo_name("cookcli")
        .build()?
        .fetch()?;

    let latest = releases.first().context("No releases found")?;

    let latest_version = latest.version.trim_start_matches('v');

    if !args.force && !version_is_newer(latest_version, current_version) {
        println!("You are already on the latest version!");
        return Ok(());
    }

    println!("New version available: {latest_version}");

    if args.check_only {
        println!("Run 'cook update' to install the latest version.");
        return Ok(());
    }

    println!("Downloading and installing version {latest_version}...");

    let target = get_target_triple()?;
    let binary_name = format!("cook-{target}");

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
            println!("Already up to date (version {v})");
        }
        self_update::Status::Updated(v) => {
            println!("Successfully updated to version {v}");

            // On macOS, try to remove quarantine attribute from the updated binary
            #[cfg(target_os = "macos")]
            {
                if let Ok(current_exe) = std::env::current_exe() {
                    let _ = std::process::Command::new("xattr")
                        .args(["-d", "com.apple.quarantine"])
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

    if version_is_newer(latest_version, current_version) {
        Ok(Some(latest_version.to_string()))
    } else {
        Ok(None)
    }
}

/// `true` when `latest` sorts after `current` as a version.
///
/// Compared semantically: plain string ordering only agrees while every
/// component keeps its width, so `0.38.10` sorted *before* `0.38.9` and an
/// update across a digit-width boundary was never offered. Unparseable
/// values fall back to the string ordering this replaced — the previous
/// behaviour is the best guess left for those.
fn version_is_newer(latest: &str, current: &str) -> bool {
    match (
        semver::Version::parse(latest),
        semver::Version::parse(current),
    ) {
        (Ok(latest), Ok(current)) => latest > current,
        _ => latest > current,
    }
}

fn get_target_triple() -> Result<String> {
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else if cfg!(target_arch = "x86") {
        "i686"
    } else {
        anyhow::bail!("Unsupported architecture: {}", std::env::consts::ARCH)
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
        anyhow::bail!("Unsupported operating system: {}", std::env::consts::OS)
    };

    let special_arm = if cfg!(target_arch = "arm") && cfg!(target_os = "linux") {
        "eabihf"
    } else {
        ""
    };

    Ok(format!("{arch}-{os}{special_arm}"))
}

#[cfg(test)]
mod tests {
    use super::version_is_newer;

    /// The case string ordering got backwards: `"0.38.9" >= "0.38.10"`.
    #[test]
    fn a_digit_wider_version_is_newer_not_older() {
        assert!(version_is_newer("0.38.10", "0.38.9"));
        assert!(!version_is_newer("0.38.9", "0.38.10"));
    }

    #[test]
    fn equal_versions_are_not_newer() {
        assert!(!version_is_newer("0.38.1", "0.38.1"));
    }

    #[test]
    fn unparseable_versions_fall_back_to_string_order() {
        assert!(version_is_newer("zeta", "alpha"));
        assert!(!version_is_newer("alpha", "zeta"));
    }
}
