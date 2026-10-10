//! What the server sends never says where it keeps its files.
//!
//! Errors from the filesystem, the recipe finder, the parser and the pantry
//! name the file they are about by its absolute path. The server writes those
//! paths relative to the recipe directory, or as `…/file` for a configuration
//! folder, before an error leaves it — for guests of a `--recipes-only` server
//! as for everyone else.

#![cfg(feature = "server")]

use reqwest::StatusCode;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    dir: TempDir,
}

impl ServerGuard {
    /// The answer to `GET path`, after checking it does not name the
    /// temporary directory the recipes and configuration live in.
    async fn get(&self, path: &str) -> (StatusCode, String) {
        let resp = reqwest::get(format!("http://127.0.0.1:{}{}", self.port, path))
            .await
            .expect("GET");
        let status = resp.status();
        let text = resp.text().await.expect("text body");
        let root = self.dir.path().to_str().unwrap();
        assert!(!text.contains(root), "GET {path} names {root}: {text}");
        (status, text)
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local addr").port()
}

/// Where each test's files go, and which pantry file is broken.
#[derive(Clone, Copy)]
enum Pantry {
    /// `config/pantry.conf` in the recipe directory.
    Local,
    /// `pantry.conf` in the global configuration folder.
    Global,
}

/// A server for a recipe directory holding a recipe the parser refuses and a
/// pantry file that is not TOML. `free_port` only reserves a port long enough
/// to learn its number, so retry when another test claims it first.
async fn start_server(pantry: Pantry, args: &[&str]) -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server(pantry, args).await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server(pantry: Pantry, args: &[&str]) -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    let recipes = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes).unwrap();
    std::fs::write(recipes.join("Omelette.cook"), "Beat @eggs{3}.\n").unwrap();
    std::fs::write(recipes.join("Broken.cook"), "Add @salt{1/0}.\n").unwrap();
    let config = match pantry {
        Pantry::Local => recipes.join("config"),
        // Where `common::with_isolated_config` points `COOK_CONFIG_DIR`.
        Pantry::Global => dir.path().join(".cook-config"),
    };
    write(&config.join("pantry.conf"), "not = [valid toml\n");

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(&recipes)
        .arg("--port")
        .arg(port.to_string())
        .args(args);
    let child = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/recipes");
    for _ in 0..200 {
        if guard.child.try_wait().expect("poll server").is_some() {
            // Port was taken between reserving and binding it.
            return None;
        }
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Some(guard);
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("cook server on port {port} never became ready");
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

#[tokio::test]
async fn a_folder_that_does_not_exist_is_named_as_asked() {
    let server = start_server(Pantry::Local, &[]).await;

    let (_, page) = server.get("/directory/Soups").await;
    assert!(page.contains("Directory does not exist: Soups"), "{page}");

    let (status, page) = server.get("/random/Soups").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(page.contains("Directory does not exist: Soups"), "{page}");

    let (_, page) = server.get("/directory/Omelette.cook").await;
    assert!(page.contains("Omelette.cook"), "{page}");
}

#[tokio::test]
async fn a_recipe_that_does_not_parse_is_named_relative_to_the_recipes() {
    let server = start_server(Pantry::Local, &[]).await;

    let (status, body) = server.get("/api/recipes/Broken.cook").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let error: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = error["error"].as_str().unwrap();
    assert!(
        error.starts_with("Failed to parse recipe 'Broken.cook'"),
        "{error}"
    );
    // The parser's report still points at the line.
    assert!(error.contains("[Broken.cook]"), "{error}");
    assert!(error.contains("Add @salt{1/0}."), "{error}");

    let (_, page) = server.get("/recipe/Broken.cook").await;
    assert!(page.contains("Broken.cook"), "{page}");

    let (status, _) = server.get("/api/recipe_image/Broken.cook?step=1").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn guests_of_a_recipes_only_server_learn_no_paths_either() {
    let server = start_server(Pantry::Local, &["--recipes-only"]).await;

    for path in [
        "/directory/Soups",
        "/random/Soups",
        "/recipe/Broken.cook",
        "/api/recipes/Broken.cook",
    ] {
        server.get(path).await;
    }
}

#[tokio::test]
async fn a_broken_pantry_file_in_the_recipes_is_named_relative_to_them() {
    let server = start_server(Pantry::Local, &[]).await;

    let (status, page) = server.get("/pantry").await;
    assert_eq!(status, StatusCode::OK);
    // `config\pantry.conf` on Windows.
    let relative = Path::new("config").join("pantry.conf");
    assert!(
        page.contains(&format!("invalid configuration at {}", relative.display())),
        "{page}"
    );
}

#[tokio::test]
async fn a_broken_global_pantry_file_is_named_without_its_folder() {
    let server = start_server(Pantry::Global, &[]).await;

    let (status, page) = server.get("/pantry").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        page.contains("invalid configuration at …/pantry.conf"),
        "{page}"
    );
}

#[tokio::test]
async fn the_editor_is_not_told_where_the_recipes_are() {
    let server = start_server(Pantry::Local, &[]).await;

    let (status, page) = server.get("/edit/Omelette.cook").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("rootUri: null"), "{page}");
}
