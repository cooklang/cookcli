//! `COOK_CONFIG_DIR` replaces the global configuration directory.
//!
//! # Why this matters more than it looks
//!
//! Every global lookup `cook` makes — `aisle.conf`, `pantry.conf`,
//! `session.json`, `sync.db` — goes through `cookcli_core::global_config_path`,
//! which asks `directories` for the platform configuration directory. On
//! Windows that resolves through the Known Folder API, so `HOME` and
//! `XDG_CONFIG_HOME` do nothing: a test suite that isolates itself by setting
//! those is isolated on Linux and macOS and not isolated at all on Windows.
//!
//! The consequence was not merely flaky output. `cook server` loads
//! `session.json` on boot and, when it finds one, starts syncing whichever
//! directory it was pointed at against the global `sync.db` — the database
//! tracking the developer's own recipe folder. On a Windows machine where
//! someone had run `cook login`, `cargo test` could upload fixture recipes to
//! their real cook.md account, or index a temp directory into that database
//! and propagate the deletions.
//!
//! `COOK_CONFIG_DIR` is the fix, and these tests are what keeps it working.
//! They exercise the real binary on the real operating system, because the
//! failure being guarded against is precisely one that a `directories` call
//! inside a unit test would not reproduce.
//!
//! # These tests never write outside their own temp directories
//!
//! Deliberately. The obvious way to prove the override — plant a session, run
//! `cook logout`, assert the file is gone — deletes the developer's real
//! session if the override ever regresses, which is a poor property for the
//! test guarding against exactly that. Everything below observes a *read*
//! instead: what the command reports having found.

use assert_cmd::Command as AssertCommand;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// A recipe directory with no `config/` of its own, so the only configuration
/// in play is whatever the override points at.
fn recipe_dir() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    fs::write(
        dir.path().join("Soup.cook"),
        "Simmer @leek{2} in @stock{1%l}.\n",
    )
    .unwrap();
    assert!(
        !dir.path().join("config").exists(),
        "the fixture must have no local config, or it would shadow the override"
    );
    dir
}

/// An `aisle.conf` and a `pantry.conf` that no real machine would have, so an
/// assertion on them cannot pass by accident against a global file.
fn config_dir() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("aisle.conf"), "[isolated aisle]\nleek\n").unwrap();
    // `pantry.conf` is TOML, where a bare key cannot contain a space — an
    // `[isolated pantry]` heading parses as garbage and the file is silently
    // treated as empty, which would make the assertion below pass for the
    // wrong reason.
    fs::write(
        dir.path().join("pantry.conf"),
        "[isolated-pantry]\nstock = \"5%l\"\n",
    )
    .unwrap();
    dir
}

fn cook(config: &Path) -> AssertCommand {
    let mut cmd = AssertCommand::cargo_bin("cook").expect("cook binary");
    cmd.env(cookcli_core::CONFIG_DIR_ENV, config);
    cmd
}

/// Both halves of `Context::discover`'s global fallback, in one run: the
/// override's `aisle.conf` supplies the category heading, and its `pantry.conf`
/// covers `stock` in full so the ingredient drops out of the list entirely.
#[test]
fn aisle_and_pantry_are_read_from_the_config_dir_override() {
    let recipes = recipe_dir();
    let config = config_dir();

    let output = cook(config.path())
        .current_dir(recipes.path())
        .arg("shopping-list")
        .arg("Soup.cook")
        .output()
        .expect("run cook shopping-list");

    assert!(
        output.status.success(),
        "shopping-list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[isolated aisle]"),
        "the override's aisle.conf should have supplied the category: {stdout}"
    );
    assert!(
        stdout.contains("leek"),
        "leek is not in the pantry and should still be listed: {stdout}"
    );
    assert!(
        !stdout.contains("stock"),
        "the override's pantry.conf covers stock in full, so it should be gone: {stdout}"
    );
}

/// The override is what decides, not `HOME`. Setting `HOME` and
/// `XDG_CONFIG_HOME` somewhere else entirely must not change the answer —
/// which is the whole reason the variable exists, since on Windows they were
/// never consulted in the first place.
#[test]
fn the_config_dir_override_wins_over_home() {
    let recipes = recipe_dir();
    let config = config_dir();
    let elsewhere = TempDir::new().expect("temp dir");

    let output = cook(config.path())
        .current_dir(recipes.path())
        .env("HOME", elsewhere.path())
        .env("XDG_CONFIG_HOME", elsewhere.path().join(".config"))
        .arg("shopping-list")
        .arg("Soup.cook")
        .output()
        .expect("run cook shopping-list");

    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("[isolated aisle]"),
        "HOME must not override COOK_CONFIG_DIR"
    );
}

/// A local `./config/` still wins, as it always has. The override replaces the
/// global tier of the search order, it does not collapse it.
#[test]
fn a_local_config_still_wins_over_the_config_dir_override() {
    let recipes = recipe_dir();
    let config = config_dir();

    let local = recipes.path().join("config");
    fs::create_dir_all(&local).unwrap();
    fs::write(local.join("aisle.conf"), "[local aisle]\nleek\n").unwrap();
    fs::write(local.join("pantry.conf"), "").unwrap();

    let output = cook(config.path())
        .current_dir(recipes.path())
        .arg("shopping-list")
        .arg("Soup.cook")
        .output()
        .expect("run cook shopping-list");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[local aisle]") && !stdout.contains("[isolated aisle]"),
        "the local config should have won: {stdout}"
    );
    assert!(
        stdout.contains("stock"),
        "the empty local pantry should shadow the override's: {stdout}"
    );
}

/// The session file is the dangerous one: it is what makes `cook server` start
/// syncing. These two tests pin both directions.
#[cfg(feature = "sync")]
mod session {
    use super::*;

    /// Payload: `{"uid":"isolated-test-user","exp":4102444800,"email":"isolated@example.invalid"}`
    /// base64url-encoded without padding, between a `{}` header and a stub
    /// signature — the three parts `decode_jwt_claims` splits on. Nothing
    /// verifies the signature; `exp` is 2100-01-01, so the session never reads
    /// as expired.
    ///
    /// `uid` is a string rather than a number on purpose. `SyncSession` accepts
    /// either, but `start_sync` parses `user_id` as an `i32` and bails before
    /// it spawns anything, so a server booted on this session can report being
    /// logged in without any sync task, database or network request existing.
    const FAKE_JWT: &str = concat!(
        "e30.",
        "eyJ1aWQiOiJpc29sYXRlZC10ZXN0LXVzZXIiLCJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoi",
        "aXNvbGF0ZWRAZXhhbXBsZS5pbnZhbGlkIn0",
        ".not-a-real-signature"
    );

    fn write_session(config: &Path) {
        fs::write(
            config.join("session.json"),
            format!(
                r#"{{"jwt":"{FAKE_JWT}","user_id":"isolated-test-user","email":"isolated@example.invalid"}}"#
            ),
        )
        .unwrap();
    }

    /// `cook login` reports an existing session and exits without contacting
    /// anything, so this observes the session file being read and nothing else.
    #[test]
    fn a_session_in_the_config_dir_override_is_found() {
        let config = TempDir::new().expect("temp dir");
        write_session(config.path());

        cook(config.path())
            .arg("login")
            .assert()
            .success()
            .stdout(predicates::str::contains("Already logged in"));
    }

    /// The guarantee the whole change exists for: with the override pointed at
    /// an empty directory, no session is found — whatever the developer running
    /// the suite has in their real one.
    ///
    /// Without a session `cook login` starts a device-code flow, so the
    /// endpoints are aimed at a closed loopback port. It fails there, before
    /// the prompt, and the assertion is on what it did *not* report.
    #[test]
    fn an_empty_config_dir_override_hides_any_real_session() {
        let config = TempDir::new().expect("temp dir");
        let dead = format!("http://127.0.0.1:{}", closed_port());

        let output = cook(config.path())
            .env("COOK_API_ENDPOINT", format!("{dead}/api"))
            .env("COOK_SYNC_ENDPOINT", &dead)
            .arg("login")
            .output()
            .expect("run cook login");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("Already logged in"),
            "a real session leaked past the override: {stdout}"
        );
        assert!(
            !output.status.success(),
            "the device flow should have failed against a closed port: {stdout}"
        );
    }

    /// The call site the whole thing turns on: `build_state` in
    /// `src/server/mod.rs`, which loads `session.json` and, on finding one,
    /// starts syncing the directory it was given against the global `sync.db`.
    ///
    /// Asserted in the *negative* direction on purpose. Pointed at an empty
    /// configuration directory the server must report no session, and a server
    /// with no session starts no sync, opens no database and sends nothing —
    /// so if the override ever regresses this test fails without having done
    /// any of the damage it exists to prevent. It is also only on a machine
    /// where someone has run `cook login` that it can fail at all, which is
    /// exactly the machine at risk.
    #[tokio::test]
    async fn a_server_with_an_isolated_config_dir_has_no_session() {
        let recipes = recipe_dir();
        let config = TempDir::new().expect("temp dir");
        let server = start_server(recipes.path(), config.path()).await;

        let status: serde_json::Value = reqwest::get(server.url("/api/sync/status"))
            .await
            .expect("request sync status")
            .json()
            .await
            .expect("sync status json");

        assert_eq!(
            status["logged_in"],
            serde_json::Value::Bool(false),
            "the server found a session outside its configuration directory: {status}"
        );
        assert_eq!(
            status["syncing"],
            serde_json::Value::Bool(false),
            "no session means no sync task: {status}"
        );
    }

    /// Kills the spawned server when the test ends, pass or panic.
    struct ServerGuard {
        child: std::process::Child,
        port: u16,
    }

    impl ServerGuard {
        fn url(&self, path: &str) -> String {
            format!("http://127.0.0.1:{}{}", self.port, path)
        }
    }

    impl Drop for ServerGuard {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    /// `closed_port` only reserves a port long enough to learn its number, so
    /// another test can claim it first. The server exits 1 on a bound port, so
    /// retry with a fresh one.
    async fn start_server(recipes: &Path, config: &Path) -> ServerGuard {
        for _ in 0..5 {
            if let Some(server) = try_start_server(recipes, config).await {
                return server;
            }
        }
        panic!("could not start cook server on a free port after 5 attempts");
    }

    async fn try_start_server(recipes: &Path, config: &Path) -> Option<ServerGuard> {
        let port = closed_port();
        let child = std::process::Command::new(assert_cmd::cargo::cargo_bin("cook"))
            .arg("server")
            .arg(recipes)
            .arg("--port")
            .arg(port.to_string())
            .env(cookcli_core::CONFIG_DIR_ENV, config)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn cook server");

        let mut guard = ServerGuard { child, port };
        let url = guard.url("/api/sync/status");
        for _ in 0..600 {
            if guard.child.try_wait().expect("poll server").is_some() {
                // Port was taken between reserving and binding it.
                return None;
            }
            if let Ok(resp) = reqwest::get(&url).await {
                if resp.status().is_success() {
                    return Some(guard);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("cook server on port {port} never became ready");
    }

    /// A port nothing is listening on: bound to learn its number, then
    /// released. Another process could claim it in between, which would only
    /// make the request above fail differently — the assertions do not depend
    /// on how it fails.
    fn closed_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .expect("bind ephemeral port")
            .local_addr()
            .expect("local addr")
            .port()
    }
}
