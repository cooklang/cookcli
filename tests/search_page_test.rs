//! End-to-end tests for the search results page.
//!
//! `GET /search?q=` renders every recipe and menu matching the query as
//! cards, the same hits `/api/search` gives the dropdown. An empty query
//! goes back to the recipe list.

#![cfg(feature = "server")]

use reqwest::{redirect, StatusCode};
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
    #[allow(dead_code)] // keeps the fixture directory alive for the server's lifetime
    dir: TempDir,
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

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local addr").port()
}

fn write_fixture(recipes: &Path) {
    std::fs::create_dir_all(recipes.join("Mains/Rice Dishes")).unwrap();
    std::fs::write(
        recipes.join("Mains/Rice Dishes/Chicken Rice.cook"),
        "---\ntags: [dinner, quick]\n---\nSimmer @chicken{500%g} with @rice{300%g}.\n",
    )
    .unwrap();
    std::fs::write(
        recipes.join("Mains/Roast Chicken.cook"),
        "Roast @chicken{1} with @thyme{}.\n",
    )
    .unwrap();
    std::fs::write(
        recipes.join("Pancakes.cook"),
        "Whisk @flour{200%g} and @milk{300%ml}.\n",
    )
    .unwrap();
    std::fs::write(
        recipes.join("Week.menu"),
        "== Monday ==\n\nDinner:\n- @./Mains/Roast Chicken{}\n",
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
    write_fixture(&dir.path().join("recipes"));

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

/// A client that reports redirects instead of following them.
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(redirect::Policy::none())
        .build()
        .unwrap()
}

async fn get(server: &ServerGuard, path: &str) -> (StatusCode, String) {
    let resp = client().get(server.url(path)).send().await.unwrap();
    let status = resp.status();
    (status, resp.text().await.unwrap())
}

#[tokio::test]
async fn lists_every_match_with_its_folder() {
    let server = start_server().await;

    let (status, page) = get(&server, "/search?q=chicken").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("3 recipes match"), "{page}");
    assert!(
        page.contains(r#"href="/recipe/Mains/Rice%20Dishes/Chicken%20Rice.cook""#),
        "{page}"
    );
    assert!(page.contains("📁 Mains/Rice Dishes"), "{page}");
    assert!(page.contains(r#"href="/recipe/Mains/Roast%20Chicken.cook""#));
    assert!(
        page.contains(r#"<span class="tag">dinner</span>"#),
        "{page}"
    );
    // The menu mentions the roast, and is marked as a menu.
    assert!(page.contains(r#"href="/recipe/Week.menu""#), "{page}");
    assert!(page.contains("tag mb-2"), "{page}");
    assert!(!page.contains("Pancakes"), "{page}");
}

#[tokio::test]
async fn every_term_must_match() {
    let server = start_server().await;

    let (status, page) = get(&server, "/search?q=chicken+rice").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("1 recipe matches"), "{page}");
    assert!(page.contains("Chicken Rice"), "{page}");
    assert!(!page.contains("Roast Chicken"), "{page}");
}

#[tokio::test]
async fn keeps_the_query_in_the_search_box() {
    let server = start_server().await;

    let (_, page) = get(&server, "/search?q=%22%3E%3Cb%3Ex").await;
    assert!(page.contains(r#"value="&quot;&gt;&lt;b&gt;x""#), "{page}");
    assert!(!page.contains("<b>x"), "{page}");
}

#[tokio::test]
async fn says_when_nothing_matches() {
    let server = start_server().await;

    let (status, page) = get(&server, "/search?q=xylophone").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("No recipes found"), "{page}");
    assert!(!page.contains(r#"class="recipe-card""#), "{page}");
}

#[tokio::test]
async fn an_empty_search_goes_to_the_recipe_list() {
    let server = start_server().await;

    for path in ["/search", "/search?q=", "/search?q=+++"] {
        let resp = client().get(server.url(path)).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::SEE_OTHER, "GET {path}");
        assert_eq!(resp.headers()["location"], "/", "GET {path}");
    }
}
