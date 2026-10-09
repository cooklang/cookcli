use anyhow::{bail, Context, Result};
use clap::Args;
use self_update::backends::github::ReleaseList;
use self_update::update::ReleaseAsset;
use self_update::{cargo_crate_version, Download, Extract};
use sha2::{Digest, Sha256};
use std::fs;

const RELEASES_URL: &str = "https://github.com/cooklang/cookcli/releases";

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

    // Pick the archive by its exact name: a substring match would also take
    // its `.sha256`, depending on the order GitHub lists the assets in.
    let name = asset_name(latest_version)?;
    let find = |name: &str| latest.assets.iter().find(|asset| asset.name == name);
    let Some(archive) = find(&name) else {
        bail!(
            "Release {latest_version} has no {name} for this platform. \
             Download it by hand from {RELEASES_URL}"
        );
    };

    let tmp_dir = self_update::TempDir::new()?;
    let archive_path = tmp_dir.path().join(&archive.name);
    download(archive, fs::File::create(&archive_path)?, true)?;

    match find(&format!("{name}.sha256")) {
        Some(checksum) => {
            let mut expected = Vec::new();
            download(checksum, &mut expected, false)?;
            let expected = String::from_utf8_lossy(&expected);
            let expected = expected.split_whitespace().next().unwrap_or_default();
            let actual = sha256_hex(&fs::read(&archive_path)?);
            if !actual.eq_ignore_ascii_case(expected) {
                bail!("Checksum mismatch for {name}: expected {expected}, got {actual}");
            }
        }
        None => bail!("Release {latest_version} has no checksum for {name}"),
    }

    let bin_name = if cfg!(windows) { "cook.exe" } else { "cook" };
    Extract::from_source(&archive_path)
        .extract_file(tmp_dir.path(), bin_name)
        .with_context(|| format!("Could not extract {bin_name} from {name}"))?;
    self_update::self_replace::self_replace(tmp_dir.path().join(bin_name))?;

    println!("Successfully updated to version {latest_version}");

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

fn download(asset: &ReleaseAsset, dest: impl std::io::Write, show_progress: bool) -> Result<()> {
    // `download_url` is the API URL of the asset, which serves the file itself
    // only when asked for octet-stream.
    Download::from_url(&asset.download_url)
        .set_header(reqwest::header::ACCEPT, "application/octet-stream".parse()?)
        .show_progress(show_progress)
        .download_to(dest)
        .with_context(|| format!("Could not download {}", asset.name))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The release asset holding this build, `cook-<version>-<platform>.<ext>`,
/// as `github_build` in `.github/workflows/release.yaml` names it.
fn asset_name(version: &str) -> Result<String> {
    let (platform, ext) = release_platform()?;
    Ok(format!("cook-{version}-{platform}.{ext}"))
}

/// This build's `platform` and `ext` in the `github_build` release matrix, or
/// an error on a platform the release does not build for.
fn release_platform() -> Result<(String, &'static str)> {
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "arm") {
        "armhf"
    } else if cfg!(target_arch = "x86") {
        "i686"
    } else {
        bail!("Unsupported architecture: {}", std::env::consts::ARCH)
    };

    let platform = if cfg!(target_os = "linux") {
        let libc = if cfg!(target_env = "musl") {
            "musl"
        } else {
            "gnu"
        };
        format!("linux-{arch}-{libc}")
    } else if cfg!(target_os = "macos") {
        format!("macos-{arch}")
    } else if cfg!(target_os = "windows") {
        format!("windows-{arch}")
    } else if cfg!(target_os = "freebsd") {
        format!("freebsd-{arch}")
    } else {
        bail!("Unsupported operating system: {}", std::env::consts::OS)
    };

    let ext = if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    };

    Ok((platform, ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_name_has_version_and_platform() {
        let name = asset_name("1.2.3").unwrap();
        let (platform, ext) = release_platform().unwrap();
        assert_eq!(name, format!("cook-1.2.3-{platform}.{ext}"));
        assert!(!name.contains("unknown"), "{name}");
    }

    /// The updater looks for the asset the release workflow builds for this
    /// platform; renaming one side without the other strands `cook update`.
    #[test]
    fn release_workflow_builds_this_platform() {
        let workflow = include_str!("../.github/workflows/release.yaml");
        let (platform, ext) = release_platform().unwrap();
        let platform_line = format!("platform: {platform}");
        let ext_line = workflow
            .lines()
            .map(str::trim)
            .skip_while(|line| *line != platform_line)
            .find(|line| line.starts_with("ext:"))
            .unwrap_or_else(|| panic!("release.yaml's matrix has no `{platform_line}`"));
        assert_eq!(ext_line, format!("ext: {ext}"));
    }

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
