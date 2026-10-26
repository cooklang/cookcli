//! End-to-end tests for browsing by course and by tag.
//!
//! With the `browse_by=course` cookie the front page lists the courses
//! recipes declare instead of the top-level folders. `/course/{course}`,
//! `/no-course` and `/tag/{tag}` list the matching recipes from every folder.

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
    let write = |path: &str, text: &str| {
        let path = recipes.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "Mains/Curry.cook",
        "---\ncourse: dinner\ntags: [quick, spicy]\n---\nSimmer @chickpeas{400%g}.\n",
    );
    write(
        "Books/Cakes/Brownies.cook",
        "---\ntitle: Fudgy Brownies\ncourse: dessert\ntags: [chocolate]\n---\nBake @cocoa{50%g}.\n",
    );
    write(
        "Sides/Sorbet.cook",
        "---\ncourse: Dessert\ntags: [quick]\n---\nFreeze @lemon{2}.\n",
    );
    write(
        "Soup/Rice and Peas.cook",
        "---\ncourse: side & soup\n---\nBoil @rice{200%g}.\n",
    );
    write("Toast.cook", "Toast @bread{2%slices}.\n");
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

/// GET `path`, sending `cookie` when there is one.
async fn get(server: &ServerGuard, path: &str, cookie: Option<&str>) -> (StatusCode, String) {
    let client = reqwest::Client::builder()
        .redirect(redirect::Policy::none())
        .build()
        .unwrap();
    let mut request = client.get(server.url(path));
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    let resp = request.send().await.unwrap();
    let status = resp.status();
    (status, resp.text().await.unwrap())
}

#[tokio::test]
async fn the_front_page_lists_folders_unless_the_viewer_browses_by_course() {
    let server = start_server().await;

    let (status, page) = get(&server, "/", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains(r#"href="/directory/Mains""#), "{page}");
    assert!(!page.contains(r#"href="/course/"#), "{page}");

    let (status, page) = get(&server, "/", Some("browse_by=course")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!page.contains(r#"href="/directory/Mains""#), "{page}");
    // One card per course, A to Z, whatever case the recipes wrote it in,
    // then the recipes without one.
    let order: Vec<usize> = [
        r#"href="/course/dessert""#,
        r#"href="/course/dinner""#,
        r#"href="/course/side%20%26%20soup""#,
        r#"href="/no-course""#,
    ]
    .iter()
    .map(|href| {
        page.find(href)
            .unwrap_or_else(|| panic!("{href} in {page}"))
    })
    .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{page}");
    assert!(page.contains("Dessert"), "{page}");
    assert!(page.contains("2 recipes"), "{page}");
    assert!(page.contains("No course"), "{page}");
}

#[tokio::test]
async fn a_folder_opened_by_its_url_still_lists_the_folder() {
    let server = start_server().await;

    let (status, page) = get(&server, "/directory/Mains", Some("browse_by=course")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains(r#"href="/recipe/Mains/Curry""#), "{page}");
}

#[tokio::test]
async fn a_course_lists_its_recipes_from_every_folder() {
    let server = start_server().await;

    let (status, page) = get(&server, "/course/DESSERT", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        page.contains("<h1 class=\"text-display font-bold text-text mb-2\">DESSERT</h1>"),
        "{page}"
    );
    assert!(page.contains("2 recipes"), "{page}");
    assert!(
        page.contains(r#"href="/recipe/Books/Cakes/Brownies.cook""#),
        "{page}"
    );
    assert!(page.contains("Fudgy Brownies"), "{page}");
    assert!(page.contains("📁 Books/Cakes"), "{page}");
    assert!(
        page.contains(r#"href="/recipe/Sides/Sorbet.cook""#),
        "{page}"
    );
    assert!(!page.contains("Curry"), "{page}");

    let (status, page) = get(&server, "/course/side%20%26%20soup", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("Rice and Peas"), "{page}");
}

#[tokio::test]
async fn no_course_lists_the_recipes_declaring_none() {
    let server = start_server().await;

    let (status, page) = get(&server, "/no-course", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("1 recipe"), "{page}");
    assert!(page.contains(r#"href="/recipe/Toast.cook""#), "{page}");
}

#[tokio::test]
async fn a_tag_lists_its_recipes_and_tags_link_to_their_lists() {
    let server = start_server().await;

    let (status, page) = get(&server, "/tag/Quick", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("#Quick"), "{page}");
    assert!(page.contains("2 recipes"), "{page}");
    assert!(
        page.contains(r#"href="/recipe/Mains/Curry.cook""#),
        "{page}"
    );
    assert!(
        page.contains(r#"href="/recipe/Sides/Sorbet.cook""#),
        "{page}"
    );
    // The cards' tags lead on to their own lists.
    assert!(
        page.contains(r#"<a href="/tag/spicy" class="tag tag-link">spicy</a>"#),
        "{page}"
    );
}

#[tokio::test]
async fn a_recipe_links_its_course_and_tags() {
    let server = start_server().await;

    let (status, page) = get(&server, "/recipe/Mains/Curry.cook", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains(r#"href="/course/dinner""#), "{page}");
    assert!(page.contains(r#"href="/tag/quick""#), "{page}");
}

#[tokio::test]
async fn an_unknown_course_or_tag_is_an_empty_list() {
    let server = start_server().await;

    for path in ["/course/brunch", "/tag/vegan"] {
        let (status, page) = get(&server, path, None).await;
        assert_eq!(status, StatusCode::OK, "GET {path}");
        assert!(page.contains("No recipes found."), "{page}");
        assert!(!page.contains(r#"class="recipe-card""#), "{page}");
    }
}

#[tokio::test]
async fn the_preference_is_offered_and_shows_the_current_choice() {
    let server = start_server().await;

    let (_, page) = get(&server, "/preferences", Some("browse_by=course")).await;
    assert!(page.contains(r#"id="browse-by-course""#), "{page}");
    let course_button = &page[page.find(r#"id="browse-by-course""#).unwrap()..];
    assert!(
        course_button
            .trim_start_matches(r#"id="browse-by-course""#)
            .trim_start()
            .starts_with(r#"aria-pressed="true""#),
        "{page}"
    );
}
