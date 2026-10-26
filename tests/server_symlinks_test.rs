//! Requests that reach a file through a link out of the recipe directory
//! are refused: the path checks are lexical, so `Link/Secret.cook` through a
//! `Link -> /outside` symlink passes them and every handler that then opened
//! the file read, wrote or removed the file the link pointed to (#698).
//!
//! Links that stay inside the collection still resolve: the containment check
//! runs on the canonical path, not on whether a link is involved.

#![cfg(all(feature = "server", unix))]

#[path = "common/mod.rs"]
mod common;

use serde_json::Value;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    /// Holds `recipes/` and `outside/`.
    dir: TempDir,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    fn recipes(&self) -> PathBuf {
        self.dir.path().join("recipes")
    }

    fn outside(&self) -> PathBuf {
        self.dir.path().join("outside")
    }

    fn link(&self, to: &Path, name: &str) {
        std::os::unix::fs::symlink(to, self.recipes().join(name)).unwrap();
    }

    async fn get(&self, path: &str) -> reqwest::StatusCode {
        reqwest::get(self.url(path)).await.expect("GET").status()
    }

    /// The body of `GET path`: the recipe page answers its errors with a
    /// page, not a status, so what matters is what it shows.
    async fn body(&self, path: &str) -> String {
        reqwest::get(self.url(path))
            .await
            .expect("GET")
            .text()
            .await
            .expect("body")
    }

    async fn send(&self, method: reqwest::Method, path: &str) -> (reqwest::StatusCode, Value) {
        let resp = reqwest::Client::new()
            .request(method, self.url(path))
            .body("Pwned.\n")
            .send()
            .await
            .expect("request");
        let status = resp.status();
        (status, resp.json().await.unwrap_or(Value::Null))
    }

    async fn post_json(&self, path: &str, body: Value) -> (reqwest::StatusCode, Value) {
        let resp = reqwest::Client::new()
            .post(self.url(path))
            .json(&body)
            .send()
            .await
            .expect("request");
        let status = resp.status();
        (status, resp.json().await.unwrap_or(Value::Null))
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
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&recipes).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(recipes.join("Pancakes.cook"), "Whisk @flour{125%g}.\n").unwrap();
    std::fs::write(outside.join("Secret.cook"), "The secret recipe.\n").unwrap();
    std::fs::write(outside.join("secret.jpg"), b"the secret bytes").unwrap();

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
async fn reads_through_a_linked_folder_are_refused() {
    let server = start_server().await;
    server.link(&server.outside(), "Linked");

    for path in [
        "/api/recipes/Linked/Secret.cook",
        "/api/recipes/raw/Linked/Secret.cook",
        "/api/recipe_image/Linked/Secret.cook",
        "/api/static/Linked/secret.jpg",
    ] {
        let status = server.get(path).await;
        // Reads answer like a file that is not there at all: another status
        // would reveal there is something at the link's far end.
        assert_eq!(status, 404, "GET {path}");
    }

    // The recipe page answers with an error page, not a status — the same
    // page a missing recipe gets.
    let page = server.body("/recipe/Linked/Secret.cook").await;
    assert!(page.contains("Recipe not found"), "{page}");
    assert!(!page.contains("The secret recipe"), "{page}");
}

#[tokio::test]
async fn reads_through_a_linked_file_are_refused() {
    let server = start_server().await;
    server.link(&server.outside().join("Secret.cook"), "Linked.cook");
    server.link(&server.outside().join("secret.jpg"), "Linked.jpg");

    for path in [
        "/api/recipes/Linked.cook",
        "/api/recipes/raw/Linked.cook",
        "/api/recipe_image/Linked.cook",
        "/api/static/Linked.jpg",
    ] {
        let status = server.get(path).await;
        assert_eq!(status, 404, "GET {path}");
    }

    let page = server.body("/recipe/Linked.cook").await;
    assert!(page.contains("Recipe not found"), "{page}");
    assert!(!page.contains("The secret recipe"), "{page}");
}

#[tokio::test]
async fn writes_through_a_linked_folder_change_nothing_outside() {
    let server = start_server().await;
    server.link(&server.outside(), "Linked");

    let (status, body) = server
        .send(reqwest::Method::PUT, "/api/recipes/Linked/Pwned.cook")
        .await;
    assert_eq!(status, 400, "{body}");
    let (status, body) = server
        .send(reqwest::Method::PUT, "/api/recipe_image/Linked/Secret.cook")
        .await;
    assert_eq!(status, 400, "{body}");
    let (status, body) = server
        .send(reqwest::Method::DELETE, "/api/recipes/Linked/Secret.cook")
        .await;
    assert_eq!(status, 400, "{body}");
    let (status, body) = server
        .send(
            reqwest::Method::DELETE,
            "/api/recipe_image/Linked/Secret.cook",
        )
        .await;
    assert_eq!(status, 400, "{body}");

    let written: Vec<_> = std::fs::read_dir(server.outside())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        written.len(),
        2,
        "only the fixture files may sit outside: {written:?}"
    );
    assert_eq!(
        std::fs::read_to_string(server.outside().join("Secret.cook")).unwrap(),
        "The secret recipe.\n",
        "the outside file must be untouched"
    );
}

#[tokio::test]
async fn links_staying_inside_the_collection_still_resolve() {
    let server = start_server().await;
    let shared = server.recipes().join("Shared");
    std::fs::create_dir_all(&shared).unwrap();
    std::fs::write(shared.join("Soup.cook"), "Boil @water{1%l}.\n").unwrap();
    server.link(&shared, "Alias");
    server.link(&shared.join("Soup.cook"), "Alias.cook");

    // Both the linked folder and the linked file resolve inside.
    for path in [
        "/api/recipes/raw/Alias/Soup.cook",
        "/api/recipes/raw/Alias.cook",
    ] {
        let status = server.get(path).await;
        assert_eq!(status, 200, "GET {path}");
    }
}

#[tokio::test]
async fn menus_through_a_linked_folder_are_refused() {
    let server = start_server().await;
    std::fs::write(server.outside().join("Week.menu"), "= Week\n").unwrap();
    server.link(&server.outside(), "Linked");

    // Reads answer like a missing menu; writes stay a plain refusal.
    assert_eq!(server.get("/api/menus/Linked/Week.menu").await, 404);
    let (status, body) = server
        .post_json(
            "/api/plans/Linked/Week.menu",
            serde_json::json!({
                "version": "1",
                "op": "remove",
                "date": "2025-01-01", "meal": "dinner", "index": 0, "text": "x"
            }),
        )
        .await;
    assert_eq!(status, 400, "{body}");
    let (status, body) = server
        .post_json(
            "/api/shopping_list/add_menu",
            serde_json::json!({"path": "Linked/Week.menu", "scale": 1.0}),
        )
        .await;
    assert_eq!(status, 400, "{body}");
}

#[tokio::test]
async fn the_shopping_list_neither_posts_nor_stores_linked_recipes() {
    let server = start_server().await;
    server.link(&server.outside(), "Linked");

    // `POST /api/shopping_list` is guest-reachable; a linked recipe gets the
    // same refusal a missing one gets.
    let (missing, _) = server
        .post_json(
            "/api/shopping_list",
            serde_json::json!([{"recipe": "Missing.cook"}]),
        )
        .await;
    let (status, body) = server
        .post_json(
            "/api/shopping_list",
            serde_json::json!([{"recipe": "Linked/Secret.cook"}]),
        )
        .await;
    assert_eq!(status, missing, "{body}");
    assert!(!body.to_string().contains("secret recipe"), "{body}");

    // Stored paths are resolved later, so the refusal happens at the door.
    let (status, body) = server
        .post_json(
            "/api/shopping_list/add",
            serde_json::json!({"path": "Linked/Secret.cook", "scale": 1.0}),
        )
        .await;
    assert_eq!(status, 400, "{body}");
}

/// A name without an extension skips `check_path`'s joined-path check:
/// `base.join("Leak")` is nothing until cooklang-find tries `.cook` on it —
/// and `Leak.cook` is a link. The resolved entry is checked instead (#698).
#[tokio::test]
async fn extensionless_names_through_a_linked_file_are_refused() {
    let server = start_server().await;
    server.link(&server.outside().join("Secret.cook"), "Leak.cook");

    let (missing_status, missing_body) = server
        .post_json(
            "/api/shopping_list",
            serde_json::json!([{"recipe": "Missing.cook"}]),
        )
        .await;
    for name in ["Leak", "Leak.cook"] {
        let (status, body) = server
            .post_json("/api/shopping_list", serde_json::json!([{"recipe": name}]))
            .await;
        assert_eq!(status, missing_status, "{name}: {body}");
        // Not just the status — the same words, so a link out answers
        // exactly like a missing file and reveals nothing (#698).
        let normalize = |body: &str, name: &str| body.replace(name, "NAME");
        assert_eq!(
            normalize(&body.to_string(), name),
            normalize(&missing_body.to_string(), "Missing.cook"),
            "{name}: {body}"
        );
        assert!(
            !body.to_string().contains("secret recipe"),
            "{name}: {body}"
        );

        let (status, body) = server
            .post_json(
                "/api/shopping_list/add",
                serde_json::json!({"path": name, "scale": 1.0}),
            )
            .await;
        assert_eq!(status, 400, "{name}: {body}");
    }
}

/// A recipe — or a menu — that *references* a recipe through a link out of
/// the collection gets the same answer as one referencing a missing recipe
/// (#698).
#[tokio::test]
async fn references_through_links_expand_to_nothing() {
    let server = start_server().await;
    std::fs::write(
        server.outside().join("Secret.cook"),
        "---\ntitle: Outside Title\n---\n\nTake @seeds{9%tsp}.\n",
    )
    .unwrap();
    server.link(&server.outside(), "Linked");
    std::fs::write(
        server.recipes().join("Stuffed.cook"),
        "Fill with @./Linked/Secret{}.\n",
    )
    .unwrap();
    std::fs::write(
        server.recipes().join("Week.menu"),
        "= Week\n\nDinner: \\\n- @./Stuffed{}\n",
    )
    .unwrap();

    for entry in ["Stuffed.cook", "Week.menu"] {
        let (status, body) = server
            .post_json("/api/shopping_list", serde_json::json!([{"recipe": entry}]))
            .await;
        assert!(
            !status.is_success() && !body.to_string().contains("seeds"),
            "{entry}: {status} {body}"
        );
    }
}

/// A `.shopping-list` stored before the checks — or written by another
/// client — can still carry a reference that leaves the collection. Its
/// title is not the server's to show (#698).
#[tokio::test]
async fn stored_references_through_links_name_nothing_outside() {
    let server = start_server().await;
    std::fs::write(
        server.outside().join("Secret.cook"),
        "---\ntitle: Outside Title\n---\n\nTake @seeds{9%tsp}.\n",
    )
    .unwrap();
    server.link(&server.outside(), "Linked");
    std::fs::write(
        server.recipes().join(".shopping-list"),
        "./Pancakes.cook\n  ./Linked/Secret\n",
    )
    .unwrap();

    let body = server.body("/api/shopping_list/items").await;
    assert!(!body.contains("Outside Title"), "{body}");
}

#[tokio::test]
async fn listings_search_and_stats_do_not_publish_linked_files() {
    let server = start_server().await;
    std::fs::write(server.outside().join("Week.menu"), "= Week\n").unwrap();
    server.link(&server.outside(), "Linked");

    for path in [
        "/api/recipes",
        "/api/menus",
        "/api/search?q=secret",
        "/search?q=secret",
    ] {
        let body = server.body(path).await;
        assert!(
            !body.contains("Secret") && !body.contains("Linked"),
            "{path}: {body}"
        );
    }

    let stats: Value = server.body("/api/stats").await.parse().expect("stats JSON");
    assert_eq!(stats["recipe_count"], 1, "{stats}");
    assert_eq!(stats["menu_count"], 0, "{stats}");

    // The directory page of a linked folder is empty, not an outside listing.
    let page = server.body("/directory/Linked").await;
    assert!(!page.contains("Secret"), "{page}");
}

/// `GET /api/aisles/uncategorized` scans the collection through a walk that
/// follows links — an outside folder's recipes must not reach the answer
/// (#698).
#[tokio::test]
async fn uncategorized_ingredients_do_not_name_linked_files() {
    let server = start_server().await;
    std::fs::write(
        server.outside().join("Secret.cook"),
        "Take @xylophonix{2%tsp}.\n",
    )
    .unwrap();
    std::fs::write(
        server.outside().join("Leak.cook"),
        "Take @zebrafoo{2%tsp}.\n",
    )
    .unwrap();
    server.link(&server.outside(), "Linked");
    server.link(&server.outside().join("Leak.cook"), "Leak.cook");

    let body = server.body("/api/aisles/uncategorized").await;
    assert!(
        !body.contains("xylophonix") && !body.contains("zebrafoo"),
        "{body}"
    );
    assert!(!body.contains("Linked"), "{body}");
    // The recipe inside is still scanned.
    assert!(body.contains("flour"), "{body}");
}

/// A menu naming a linked-out recipe must not store that file's own
/// references as sub-entries — `resolve_recipe_info` reading it is the
/// leak (#698).
#[tokio::test]
async fn a_menu_entry_through_a_link_keeps_none_of_its_references() {
    let server = start_server().await;
    std::fs::write(
        server.outside().join("Secret.cook"),
        "Mix @./HiddenRef{}.\n",
    )
    .unwrap();
    server.link(&server.outside(), "Linked");
    std::fs::write(
        server.recipes().join("Plan.menu"),
        "---\ntitle: Plan\n---\n\n==Day 1==\n\nDinner: \\\n- @./Linked/Secret{}\n",
    )
    .unwrap();

    let (status, _) = server
        .post_json(
            "/api/shopping_list/add_menu",
            serde_json::json!({"path": "Plan.menu", "scale": 1.0}),
        )
        .await;
    assert!(status.is_success(), "{status}");

    let body = server.body("/api/shopping_list/items").await;
    assert!(!body.contains("HiddenRef"), "{body}");
}

/// Reference scaling reads the target's `servings` — through a link that is
/// an outside file's metadata answerable to a guest (#698). The reference is
/// then a raw multiplier instead.
#[tokio::test]
async fn a_reference_scaled_by_an_outside_recipes_servings_leaks_nothing() {
    let server = start_server().await;
    std::fs::write(
        server.outside().join("Secret.cook"),
        "---\nservings: 7\n---\n\nTake @seeds{1%tsp}.\n",
    )
    .unwrap();
    server.link(&server.outside(), "Linked");
    std::fs::write(
        server.recipes().join("Plan.menu"),
        "---\ntitle: Plan\n---\n\n==Day 1==\n\nDinner: \\\n- @./Linked/Secret{14%servings}\n",
    )
    .unwrap();

    let body = server.body("/api/menus/Plan.menu").await;
    // `14%servings` against `servings: 7` would compute a 2.0 scale — the
    // outside file's number made visible. Anything else means it was never
    // read: the unit is dropped and the quantity is a raw multiplier.
    let menu: Value = body.parse().expect("menu JSON");
    let scale = menu["sections"][0]["meals"][0]["items"][0]["scale"]
        .as_f64()
        .unwrap_or(f64::NAN);
    assert_eq!(scale, 14.0, "{body}");
}
