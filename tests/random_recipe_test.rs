//! End-to-end tests for the random recipe button (#544).
//!
//! `GET /random` and `GET /random/{*path}` redirect to a `.cook` recipe picked
//! from the folder and everything below it, never a menu. The listing only
//! shows the button when there is something to pick.

#![cfg(feature = "server")]

use reqwest::{redirect, StatusCode};
use std::collections::HashSet;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

const RECIPE: &str = "Mix @flour{200%g}.\n";
const MENU: &str = "== Monday ==\n\nDinner:\n- @./Omelette{}\n";

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
    for dir in ["Mains/Pasta", "Desserts", "Menus"] {
        std::fs::create_dir_all(recipes.join(dir)).unwrap();
    }
    for file in [
        "Omelette.cook",
        "Mains/Steak Frites.cook",
        "Mains/Pasta/Carbonara.cook",
        "Desserts/Crème brûlée.cook",
    ] {
        std::fs::write(recipes.join(file), RECIPE).unwrap();
    }
    for file in ["Week.menu", "Mains/Week.menu", "Menus/Week.menu"] {
        std::fs::write(recipes.join(file), MENU).unwrap();
    }
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

/// `Location` of a `303` from `path`.
async fn pick(server: &ServerGuard, path: &str) -> String {
    let resp = client().get(server.url(path)).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER, "GET {path}");
    resp.headers()["location"].to_str().unwrap().to_string()
}

/// Every distinct `Location` seen over enough picks that missing one of a
/// handful of candidates by chance is out of the question.
async fn picks(server: &ServerGuard, path: &str) -> HashSet<String> {
    let mut seen = HashSet::new();
    for _ in 0..100 {
        seen.insert(pick(server, path).await);
    }
    seen
}

async fn get(server: &ServerGuard, path: &str) -> (StatusCode, String) {
    let resp = client().get(server.url(path)).send().await.unwrap();
    let status = resp.status();
    (status, resp.text().await.unwrap())
}

#[tokio::test]
async fn picks_from_the_folder_and_its_subfolders_only() {
    let server = start_server().await;
    let expected: HashSet<String> = [
        "/recipe/Mains/Steak%20Frites",
        "/recipe/Mains/Pasta/Carbonara",
    ]
    .map(String::from)
    .into();
    assert_eq!(picks(&server, "/random/Mains").await, expected);
}

#[tokio::test]
async fn picks_from_the_whole_collection_at_the_root_but_never_a_menu() {
    let server = start_server().await;
    let expected: HashSet<String> = [
        "/recipe/Omelette",
        "/recipe/Mains/Steak%20Frites",
        "/recipe/Mains/Pasta/Carbonara",
        "/recipe/Desserts/Cr%C3%A8me%20br%C3%BBl%C3%A9e",
    ]
    .map(String::from)
    .into();
    assert_eq!(picks(&server, "/random").await, expected);
}

#[tokio::test]
async fn the_pick_opens_as_a_recipe_page() {
    let server = start_server().await;
    let location = pick(&server, "/random/Desserts").await;
    let (status, _) = get(&server, &location).await;
    assert_eq!(status, StatusCode::OK, "GET {location}");
}

#[tokio::test]
async fn listing_links_the_button_to_the_current_folder() {
    let server = start_server().await;
    for (page, link) in [
        ("/", r#"href="/random""#),
        ("/directory/Mains", r#"href="/random/Mains""#),
        ("/directory/Mains/Pasta", r#"href="/random/Mains/Pasta""#),
    ] {
        let (status, html) = get(&server, page).await;
        assert_eq!(status, StatusCode::OK, "GET {page}");
        assert!(
            html.contains(r#"id="random-recipe""#),
            "no button on {page}"
        );
        assert!(html.contains(link), "{page} does not link {link}");
    }
}

#[tokio::test]
async fn a_folder_of_menus_has_nothing_to_pick() {
    let server = start_server().await;
    let (status, html) = get(&server, "/directory/Menus").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!html.contains(r#"id="random-recipe""#), "{html}");

    let (status, _) = get(&server, "/random/Menus").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn refuses_folders_outside_the_collection_or_missing() {
    let server = start_server().await;
    // `..%2F` reaches the server as `../`; a literal `/../` would be resolved
    // by the client before sending.
    let (status, _) = get(&server, "/random/..%2F").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = get(&server, "/random/Soups").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
