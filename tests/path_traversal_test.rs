//! End-to-end tests that the web routes taking a path cannot reach outside the
//! recipe directory.
//!
//! `/recipe/{*path}`, `/directory/{*path}`, `/edit/{*path}` and
//! `/api/recipes/{*path}` join a path from the URL to the directory the server
//! was started on. axum percent-decodes it first, so `..%2F` arrives as `../`
//! and an encoded `C:\…` arrives as an absolute path. Under the default CORS
//! policy any web page can read a `GET` response, so a route that follows such
//! a path hands out files from anywhere on the disk.
//!
//! Each test boots `cook server` on `<tmp>/recipes`, with a recipe sitting next
//! to that directory rather than in it, and checks the route both refuses to
//! reach it and still serves what is inside.
//!
//! The URLs spell `..` as `..%2F`: the client resolves a literal `/../` — and
//! `%2e%2e`, which the URL standard treats the same — before the request is
//! sent, so neither would ever reach the server.

#![cfg(feature = "server")]

use reqwest::StatusCode;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

/// Appears in a response only if the server read the recipe outside the
/// recipe directory.
const SECRET_MARKER: &str = "leaked-secret-marker";

/// The name the outside recipe is listed under, were its directory listed.
const SECRET_NAME: &str = "Outside-Secret";

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    /// Holds the recipe directory and the recipe next to it for the server's
    /// lifetime.
    dir: TempDir,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    /// The directory holding the served `recipes/` and the outside recipe.
    fn outside_dir(&self) -> PathBuf {
        self.dir.path().to_path_buf()
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

/// `<tmp>/recipes/` is what the server serves; `<tmp>/Outside-Secret.cook` is
/// one level up, outside it.
fn write_fixture(dir: &TempDir) {
    let root = dir.path();
    std::fs::write(
        root.join(format!("{SECRET_NAME}.cook")),
        format!("Add @{SECRET_MARKER}{{1}}.\n"),
    )
    .unwrap();

    let recipes = root.join("recipes");
    std::fs::create_dir_all(recipes.join("Sub")).unwrap();
    std::fs::write(recipes.join("Inside.cook"), "Mix @inside-marker{1}.\n").unwrap();
    std::fs::write(
        recipes.join("Sub").join("Nested.cook"),
        "Stir @nested-marker{1}.\n",
    )
    .unwrap();
}

/// `free_port` only reserves a port long enough to learn its number, so with
/// several tests booting servers at once another one can claim it first. The
/// server exits 1 on a bound port, so retry with a fresh one.
async fn start_server() -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server().await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server() -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir);

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(dir.path().join("recipes"))
        .arg("--port")
        .arg(port.to_string());
    let child = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };

    let client = reqwest::Client::new();
    let url = guard.url("/api/recipes");
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

async fn get(server: &ServerGuard, path: &str) -> (StatusCode, String) {
    let resp = reqwest::get(server.url(path))
        .await
        .unwrap_or_else(|e| panic!("request {path}: {e}"));
    let status = resp.status();
    let body = resp
        .text()
        .await
        .unwrap_or_else(|e| panic!("{path} returned an unreadable body: {e}"));
    (status, body)
}

/// `path` percent-encoded as a single URL segment, separators included.
fn encoded(path: &std::path::Path) -> String {
    urlencoding::encode(path.to_str().expect("utf-8 temp path")).into_owned()
}

/// The UI routes answer a bad path with the error page, so the refusal shows
/// in the body.
fn assert_refused(path: &str, body: &str, leaked: &str) {
    assert!(
        !body.contains(leaked),
        "{path} reached outside the recipe directory:\n{body}"
    );
    assert!(
        body.contains("Invalid path"),
        "{path} was not refused as an invalid path:\n{body}"
    );
}

#[tokio::test]
async fn recipe_route_stays_inside_the_recipe_directory() {
    let server = start_server().await;

    let (_, body) = get(&server, "/recipe/Inside").await;
    assert!(
        body.contains("inside-marker"),
        "a recipe inside the directory must still render:\n{body}"
    );

    let parent = format!("/recipe/..%2F{SECRET_NAME}");
    let (_, body) = get(&server, &parent).await;
    assert_refused(&parent, &body, SECRET_MARKER);

    let absolute = format!(
        "/recipe/{}",
        encoded(&server.outside_dir().join(SECRET_NAME))
    );
    let (_, body) = get(&server, &absolute).await;
    assert_refused(&absolute, &body, SECRET_MARKER);
}

#[tokio::test]
async fn directory_route_stays_inside_the_recipe_directory() {
    let server = start_server().await;

    let (_, body) = get(&server, "/directory/Sub").await;
    assert!(
        body.contains("Nested"),
        "a directory inside the recipe directory must still be listed:\n{body}"
    );

    let parent = "/directory/..%2F";
    let (_, body) = get(&server, parent).await;
    assert_refused(parent, &body, SECRET_NAME);

    let absolute = format!("/directory/{}", encoded(&server.outside_dir()));
    let (_, body) = get(&server, &absolute).await;
    assert_refused(&absolute, &body, SECRET_NAME);
}

/// The editor already refused these; this guards its move to the shared check.
#[tokio::test]
async fn edit_route_stays_inside_the_recipe_directory() {
    let server = start_server().await;

    let (_, body) = get(&server, "/edit/Inside.cook").await;
    assert!(
        body.contains("inside-marker"),
        "a recipe inside the directory must still open in the editor:\n{body}"
    );

    let parent = format!("/edit/..%2F{SECRET_NAME}.cook");
    let (_, body) = get(&server, &parent).await;
    assert_refused(&parent, &body, SECRET_MARKER);
}

/// The JSON API already refused these; this guards its move to the shared check.
#[tokio::test]
async fn api_recipe_route_stays_inside_the_recipe_directory() {
    let server = start_server().await;

    let (status, body) = get(&server, "/api/recipes/Inside.cook").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("inside-marker"), "{body}");

    let parent = format!("/api/recipes/..%2F{SECRET_NAME}.cook");
    let (status, body) = get(&server, &parent).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{parent}: {body}");
    assert!(!body.contains(SECRET_MARKER), "{parent}: {body}");
}
