//! End-to-end tests for the search results page (#828).
//!
//! `GET /search?q=…` lists every hit of the search box's query as the recipe
//! index's cards, pictures and tags included. The nav search box submits to
//! it, and an empty query goes back to the recipes.

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
    std::fs::create_dir_all(recipes.join("Pies")).unwrap();
    std::fs::write(recipes.join("Leek Soup.cook"), "Simmer @leeks{3}.\n").unwrap();
    // Not a real picture: the page only needs to find it next to the recipe.
    std::fs::write(recipes.join("Leek Soup.jpg"), b"\xff\xd8\xff\xe0").unwrap();
    std::fs::write(
        recipes.join("Pies/Leek Pie.cook"),
        "---\ntags: [dinner, pie]\n---\nBake @leeks{2} in #oven{}.\n",
    )
    .unwrap();
    std::fs::write(recipes.join("Omelette.cook"), "Whisk @eggs{3}.\n").unwrap();
}

/// `free_port` only reserves a port long enough to learn its number, so with
/// several tests booting servers at once another one can claim it first. The
/// server exits 1 on a bound port, so retry with a fresh one.
async fn start_server(args: &[&str]) -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server(args).await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server(args: &[&str]) -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir.path().join("recipes"));

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(dir.path().join("recipes"))
        .arg("--port")
        .arg(port.to_string())
        .args(args);
    let child = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };

    let prefix = args
        .iter()
        .position(|a| *a == "--url-prefix")
        .map_or("", |i| args[i + 1]);
    let client = reqwest::Client::new();
    let url = guard.url(&format!("{prefix}/api/recipes"));
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
async fn lists_every_hit_as_a_card_with_its_picture() {
    let server = start_server(&[]).await;
    let (status, html) = get(&server, "/search?q=leek").await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(html.matches(r#"class="recipe-card""#).count(), 2, "{html}");
    assert!(html.contains(r#"href="/recipe/Leek Soup.cook""#), "{html}");
    assert!(
        html.contains(r#"href="/recipe/Pies/Leek Pie.cook""#),
        "{html}"
    );
    assert!(
        html.contains(r#"src="/api/static/Leek%20Soup.jpg""#),
        "no picture on the soup's card: {html}"
    );
    assert!(
        html.contains(r#"<span class="tag">dinner</span>"#),
        "{html}"
    );
    assert!(!html.contains("Omelette"), "{html}");
    assert!(html.contains("2 recipes"), "{html}");
}

#[tokio::test]
async fn finds_what_the_dropdown_finds() {
    let server = start_server(&[]).await;
    // Every term must match, as in `cook search` and `/api/search` (#425).
    let (_, html) = get(&server, "/search?q=leek%20oven").await;
    assert_eq!(html.matches(r#"class="recipe-card""#).count(), 1, "{html}");
    assert!(html.contains("Leek Pie"), "{html}");

    let resp = client()
        .get(server.url("/api/search?q=leek%20oven"))
        .send()
        .await
        .unwrap();
    let hits: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(hits.len(), 1);
}

#[tokio::test]
async fn says_so_when_nothing_matches() {
    let server = start_server(&[]).await;
    let (status, html) = get(&server, "/search?q=caviar").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!html.contains(r#"class="recipe-card""#), "{html}");
    assert!(html.contains("No recipes found"), "{html}");
    assert!(html.contains("0 recipes"), "{html}");
}

#[tokio::test]
async fn an_empty_query_goes_back_to_the_recipes() {
    let server = start_server(&[]).await;
    for path in ["/search", "/search?q=", "/search?q=%20%20"] {
        let resp = client().get(server.url(path)).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::SEE_OTHER, "GET {path}");
        assert_eq!(resp.headers()["location"], "/", "GET {path}");
    }
}

#[tokio::test]
async fn the_query_is_shown_as_text() {
    let server = start_server(&[]).await;
    let (_, html) = get(&server, "/search?q=%3Cb%3Eleek").await;
    assert!(!html.contains("<b>leek"), "{html}");
    assert!(html.contains("&lt;b&gt;leek"), "{html}");
}

#[tokio::test]
async fn the_search_box_submits_to_the_page() {
    let server = start_server(&[]).await;
    let (_, html) = get(&server, "/").await;
    assert!(
        html.contains(r#"<form action="/search" method="get" role="search">"#),
        "{html}"
    );
    assert!(html.contains(r#"name="q""#), "{html}");
}

#[tokio::test]
async fn recipes_only_guests_may_search() {
    let server = start_server(&["--recipes-only"]).await;
    let (status, html) = get(&server, "/search?q=leek").await;
    assert_eq!(status, StatusCode::OK, "{html}");
    assert_eq!(html.matches(r#"class="recipe-card""#).count(), 2, "{html}");
}

#[tokio::test]
async fn links_and_redirects_keep_the_url_prefix() {
    let server = start_server(&["--url-prefix", "/cook"]).await;
    let (status, html) = get(&server, "/cook/search?q=soup").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        html.contains(r#"href="/cook/recipe/Leek Soup.cook""#),
        "{html}"
    );
    assert!(
        html.contains(r#"src="/cook/api/static/Leek%20Soup.jpg""#),
        "{html}"
    );

    let resp = client()
        .get(server.url("/cook/search?q="))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers()["location"], "/cook/");
}
