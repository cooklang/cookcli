//! End-to-end tests for creating a menu from the web UI (#538).
//!
//! `POST /new` takes a `kind`: `recipe` (the default) makes a `.cook` file,
//! `menu` a `.menu` file with a first day and meal to fill in. Anything else
//! is refused, and the path checks are the same for both kinds.

#![cfg(feature = "server")]

use reqwest::{redirect, StatusCode};
use std::net::TcpListener;
use std::path::PathBuf;
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
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    fn recipes(&self) -> PathBuf {
        self.dir.path().join("recipes")
    }

    /// `POST /new` as the server's own page would send it, without following
    /// the redirect.
    async fn create(&self, fields: &[(&str, &str)]) -> reqwest::Response {
        client()
            .post(self.url("/new"))
            .header("origin", format!("http://127.0.0.1:{}", self.port))
            .form(fields)
            .send()
            .await
            .expect("new file request")
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

/// A client that reports redirects instead of following them.
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(redirect::Policy::none())
        .build()
        .unwrap()
}

fn location(resp: &reqwest::Response) -> String {
    resp.headers()["location"].to_str().unwrap().to_string()
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
    let recipes = dir.path().join("recipes");
    std::fs::create_dir_all(recipes.join("Plans")).unwrap();
    std::fs::write(recipes.join("Omelette.cook"), "Beat @eggs{3}.\n").unwrap();

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(&recipes)
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

#[tokio::test]
async fn a_menu_is_created_with_a_first_day_and_meal() {
    let server = start_server().await;

    let resp = server
        .create(&[("filename", "Plans/Week 12"), ("kind", "menu")])
        .await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&resp), "/edit/Plans/Week 12.menu");
    let content = std::fs::read_to_string(server.recipes().join("Plans/Week 12.menu"))
        .expect("the menu file must exist");
    assert_eq!(
        content,
        "---\ntitle: Week 12\nservings: 2\n---\n\n== Day 1 ==\n\nBreakfast: \\\n- \n"
    );
    assert!(!server.recipes().join("Plans/Week 12.cook").exists());

    // The editor opens it in menu mode.
    let page = client()
        .get(server.url("/edit/Plans/Week%2012.menu"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains(r#"data-mode="menu""#), "{page}");
    assert!(page.contains(r#"data-action="add-recipe""#));
}

/// The listing links menus without their extension, so the menu page's Edit
/// button asks for `/edit/Week`. The editor must still open the file as a
/// menu, under its full name.
#[tokio::test]
async fn the_editor_redirects_a_bare_name_to_the_file() {
    let server = start_server().await;
    std::fs::write(
        server.recipes().join("Plans/Week 12.menu"),
        "== Day 1 ==\n\nDinner: \\\n- @./Omelette{}\n",
    )
    .unwrap();

    for (bare, full) in [
        ("/edit/Plans/Week%2012", "/edit/Plans/Week%2012.menu"),
        ("/edit/Omelette", "/edit/Omelette.cook"),
    ] {
        let resp = client().get(server.url(bare)).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::SEE_OTHER, "{bare}");
        assert_eq!(location(&resp), full, "{bare}");
    }

    // A full name is served as it is.
    let resp = client()
        .get(server.url("/edit/Plans/Week%2012.menu"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn a_recipe_is_still_the_default() {
    let server = start_server().await;

    for (name, kind) in [("Pancakes", Some("recipe")), ("Waffles", None)] {
        let mut fields = vec![("filename", name)];
        if let Some(kind) = kind {
            fields.push(("kind", kind));
        }
        let resp = server.create(&fields).await;

        assert_eq!(resp.status(), StatusCode::SEE_OTHER, "{name}");
        assert_eq!(location(&resp), format!("/edit/{name}.cook"));
        let content = std::fs::read_to_string(server.recipes().join(format!("{name}.cook")))
            .expect("the recipe file must exist");
        assert_eq!(content, format!("---\ntitle: {name}\n---\n\n"));
        assert!(!server.recipes().join(format!("{name}.menu")).exists());
    }

    let page = client()
        .get(server.url("/edit/Omelette.cook"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains(r#"data-mode="recipe""#));
    assert!(page.contains(r#"data-action="recipe-reference""#));
    assert!(!page.contains(r#"data-action="add-recipe""#));
}

/// Spaces typed around a name or a folder are not part of it: they used to
/// make a folder `Mains ` holding ` Stew .cook`.
#[tokio::test]
async fn spaces_around_names_and_folders_are_dropped() {
    let server = start_server().await;

    for (typed, kind, file) in [
        (" Mains / Beef  Stew ", "recipe", "Mains/Beef  Stew.cook"),
        ("  Plans /Week 13  ", "menu", "Plans/Week 13.menu"),
    ] {
        let resp = server.create(&[("filename", typed), ("kind", kind)]).await;

        assert_eq!(resp.status(), StatusCode::SEE_OTHER, "{typed:?}");
        assert_eq!(location(&resp), format!("/edit/{file}"), "{typed:?}");
        assert!(server.recipes().join(file).is_file(), "{file} must exist");
    }
    assert!(!server.recipes().join("Mains ").exists());
    assert!(!server.recipes().join("Plans ").exists());

    let content = std::fs::read_to_string(server.recipes().join("Mains/Beef  Stew.cook")).unwrap();
    assert_eq!(content, "---\ntitle: Beef  Stew\n---\n\n");

    // Nothing but spaces and slashes is still an empty name.
    let resp = server
        .create(&[("filename", " / "), ("kind", "recipe")])
        .await;
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert!(
        location(&resp).starts_with("/new?error="),
        "{}",
        location(&resp)
    );
}

#[tokio::test]
async fn an_unknown_kind_is_refused() {
    let server = start_server().await;

    for kind in ["report", "", "../menu", "MENU"] {
        let resp = server
            .create(&[("filename", "Mystery"), ("kind", kind)])
            .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "kind {kind:?}");
        let names: Vec<_> = std::fs::read_dir(server.recipes())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert!(
            !names.iter().any(|name| name.starts_with("Mystery")),
            "kind {kind:?} created {names:?}"
        );
    }
}

#[tokio::test]
async fn a_menu_name_cannot_leave_the_collection() {
    let server = start_server().await;

    // Dots are stripped from the name, so this lands inside the collection.
    let resp = server
        .create(&[("filename", "../../Escape"), ("kind", "menu")])
        .await;
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&resp), "/edit/Escape.menu");
    assert!(server.recipes().join("Escape.menu").exists());
    assert!(!server.dir.path().join("Escape.menu").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn a_menu_cannot_be_written_through_a_symlink_out_of_the_collection() {
    let server = start_server().await;
    let outside = server.dir.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, server.recipes().join("Linked")).unwrap();

    let resp = server
        .create(&[("filename", "Linked/Week"), ("kind", "menu")])
        .await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let to = location(&resp);
    assert!(to.starts_with("/new?error="), "{to}");
    // Back to the menu form, not the recipe one.
    assert!(to.ends_with("&kind=menu"), "{to}");
    assert!(!outside.join("Week.menu").exists());
}

#[tokio::test]
async fn errors_send_the_user_back_to_the_menu_form() {
    let server = start_server().await;
    std::fs::write(server.recipes().join("Taken.menu"), "").unwrap();

    let resp = server
        .create(&[("filename", "Taken"), ("kind", "menu")])
        .await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let to = location(&resp);
    assert!(
        to.contains("A%20menu%20with%20this%20name%20already%20exists"),
        "{to}"
    );
    assert!(to.ends_with("&kind=menu"), "{to}");
    assert_eq!(
        std::fs::read_to_string(server.recipes().join("Taken.menu")).unwrap(),
        ""
    );
}

#[tokio::test]
async fn the_form_and_the_listing_offer_menus() {
    let server = start_server().await;

    let form = client()
        .get(server.url("/new?kind=menu"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(form.contains(r#"name="kind" value="menu""#));
    assert!(form.contains(".menu</span>"));

    let form = client()
        .get(server.url("/new"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(form.contains(r#"name="kind" value="recipe""#));
    assert!(form.contains(".cook</span>"));

    // The New menu button keeps the folder, like New Recipe.
    let listing = client()
        .get(server.url("/directory/Plans"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        listing.contains(r#"href="/new?kind=menu&amp;filename=Plans%2F""#)
            || listing.contains(r#"href="/new?kind=menu&filename=Plans%2F""#),
        "{listing}"
    );
}
