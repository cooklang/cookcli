//! End-to-end tests for the tag pages (#785).
//!
//! `/tags` lists every tag with its count and `/tag/<slug>` every recipe and
//! menu carrying one, from any folder. `cook build web` writes the same pages
//! as `tags.html` and `tag/<slug>.html`.

#![cfg(feature = "server")]

use reqwest::StatusCode;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

fn write_fixture(recipes: &Path) {
    for dir in ["Mains/Pasta", "Desserts", "Menus"] {
        std::fs::create_dir_all(recipes.join(dir)).unwrap();
    }
    for (file, tags) in [
        ("Omelette.cook", Some("[Breakfast, quick]")),
        ("Plain.cook", None),
        ("Mains/Pasta/Carbonara.cook", Some("dinner, pasta, Quick")),
        // The same tag twice still lists the recipe once.
        ("Mains/Steak Frites.cook", Some("[dinner, Dinner]")),
        ("Desserts/Crème brûlée.cook", Some("[crème]")),
    ] {
        let front = tags.map_or(String::new(), |t| format!("---\ntags: {t}\n---\n"));
        std::fs::write(recipes.join(file), format!("{front}Mix @flour{{200%g}}.\n")).unwrap();
    }
    std::fs::write(
        recipes.join("Menus/Week.menu"),
        "---\ntags: [dinner]\n---\n== Monday ==\n\nDinner:\n- @./Omelette{}\n",
    )
    .unwrap();
}

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

    let client = reqwest::Client::new();
    let url = guard.url("/tags");
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
    let resp = reqwest::get(server.url(path)).await.unwrap();
    let status = resp.status();
    (status, resp.text().await.unwrap())
}

/// The `<a>` element whose `href` is `href`, up to its closing tag.
fn link<'a>(html: &'a str, href: &str) -> &'a str {
    let start = html
        .find(&format!(r#"href="{href}""#))
        .unwrap_or_else(|| panic!("no link to {href}"));
    let end = html[start..].find("</a>").expect("closing tag") + start;
    &html[start..end]
}

/// The text of every card title on a page, in page order.
fn card_titles(html: &str) -> Vec<&str> {
    html.split(r#"<h2 class="recipe-card-title">"#)
        .skip(1)
        .map(|rest| &rest[..rest.find("</h2>").unwrap()])
        .collect()
}

#[tokio::test]
async fn the_index_lists_every_tag_with_its_count() {
    let server = start_server(&[]).await;
    let (status, html) = get(&server, "/tags").await;
    assert_eq!(status, StatusCode::OK);
    for (href, name, count) in [
        ("/tag/breakfast", "#Breakfast", 1),
        ("/tag/quick", "#Quick", 2),
        ("/tag/dinner", "#dinner", 3),
        ("/tag/pasta", "#pasta", 1),
        ("/tag/cr%C3%A8me", "#crème", 1),
    ] {
        let a = link(&html, href);
        assert!(a.contains(name), "{href} is not named {name}: {a}");
        assert!(
            a.contains(&format!(">{count}<")),
            "{href} is not {count}: {a}"
        );
    }
}

#[tokio::test]
async fn a_tag_page_gathers_recipes_and_menus_from_every_folder() {
    let server = start_server(&[]).await;
    let (status, html) = get(&server, "/tag/dinner").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(card_titles(&html), ["Carbonara", "Steak Frites", "Week"]);
    for href in [
        "/recipe/Mains/Pasta/Carbonara",
        "/recipe/Mains/Steak Frites",
        "/recipe/Menus/Week",
    ] {
        link(&html, href);
    }
    for folder in ["Mains/Pasta/", "Mains/", "Menus/"] {
        assert!(html.contains(&format!(">{folder}<")), "no folder {folder}");
    }
}

#[tokio::test]
async fn tags_match_whatever_their_case() {
    let server = start_server(&[]).await;
    for path in ["/tag/quick", "/tag/QUICK"] {
        let (status, html) = get(&server, path).await;
        assert_eq!(status, StatusCode::OK, "GET {path}");
        assert_eq!(card_titles(&html), ["Carbonara", "Omelette"], "GET {path}");
    }
}

#[tokio::test]
async fn a_tag_nothing_carries_is_a_404_page() {
    let server = start_server(&[]).await;
    let (status, html) = get(&server, "/tag/nothing").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(html.contains("#nothing"));
    assert!(html.contains("No recipe has this tag."));
}

#[tokio::test]
async fn a_recipe_links_its_tags_to_their_pages() {
    let server = start_server(&[]).await;
    let (status, html) = get(&server, "/recipe/Desserts/Cr%C3%A8me%20br%C3%BBl%C3%A9e").await;
    assert_eq!(status, StatusCode::OK);
    link(&html, "/tag/cr%C3%A8me");
    let (status, html) = get(&server, "/tag/cr%C3%A8me").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(card_titles(&html), ["Crème brûlée"]);
}

#[tokio::test]
async fn the_listing_links_the_tag_index() {
    let server = start_server(&[]).await;
    for page in ["/", "/directory/Mains"] {
        let (status, html) = get(&server, page).await;
        assert_eq!(status, StatusCode::OK, "GET {page}");
        link(&html, "/tags");
    }
}

#[tokio::test]
async fn guests_of_a_recipes_only_server_see_the_tag_pages() {
    let server = start_server(&["--recipes-only"]).await;
    for path in ["/tags", "/tag/dinner"] {
        let (status, _) = get(&server, path).await;
        assert_eq!(status, StatusCode::OK, "GET {path}");
    }
}

#[test]
fn the_static_site_has_the_same_tag_pages() {
    let dir = TempDir::new().unwrap();
    let recipes = dir.path().join("recipes");
    write_fixture(&recipes);
    let out = dir.path().join("_site");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("build")
        .arg("web")
        .arg(&out)
        .arg("--base-path")
        .arg(&recipes);
    let status = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .status()
        .expect("run cook build web");
    assert!(status.success());

    let index = std::fs::read_to_string(out.join("tags.html")).unwrap();
    link(&index, "./tag/dinner.html");
    link(&index, "./tag/cr%C3%A8me.html");

    let dinner = std::fs::read_to_string(out.join("tag").join("dinner.html")).unwrap();
    assert_eq!(card_titles(&dinner), ["Carbonara", "Steak Frites", "Week"]);
    link(&dinner, "../recipe/Mains/Pasta/Carbonara.html");
    link(&dinner, "../menu/Menus/Week.html");
    link(&dinner, "../tags.html");

    assert!(out.join("tag").join("crème.html").is_file());
    let creme = std::fs::read_to_string(
        out.join("recipe")
            .join("Desserts")
            .join("Crème brûlée.html"),
    )
    .unwrap();
    link(&creme, "../../tag/cr%C3%A8me.html");

    let home = std::fs::read_to_string(out.join("index.html")).unwrap();
    link(&home, "./tags.html");
}
