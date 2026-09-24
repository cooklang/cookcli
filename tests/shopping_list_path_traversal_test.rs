//! End-to-end tests that the shopping list endpoints taking a recipe path
//! cannot reach outside the recipe directory, and that a menu reference which
//! steps up is resolved rather than refused.
//!
//! `POST /api/shopping_list`, `/api/shopping_list/add` and
//! `/api/shopping_list/add_menu` all take a path from the request body and hand
//! it to `cooklang-find`, which joins it to the directory the server was
//! started on and asks the filesystem. `..`, an absolute path, a drive letter
//! or a UNC share each escape that directory — and on Windows merely looking a
//! UNC path up hands the host the user's NTLM hash, so the check has to happen
//! before the lookup, not after it.
//!
//! **The requests carry no `Origin` header on purpose.** `cors::write_guard`
//! refuses a cross-origin write, so a page on another site cannot reach these
//! today; a client that sends no origin at all can, and that is every local
//! process, every LAN client when the server runs with `--host`, and a
//! cross-site page once the server runs with `--cors-origin` or
//! `--no-csrf-check`. `reqwest` sends none, which is exactly the client this
//! has to hold against.
//!
//! The other half is resolution. A reference is allowed to step up —
//! `@../Shared/Sauce{}` — and where it lands back inside the recipe directory
//! it must still work, with the *resolved* path stored, so nothing later has to
//! resolve a reference a second time to use what is on the list.

#![cfg(feature = "server")]

use reqwest::StatusCode;
use serde_json::{json, Value};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

/// Appears in a response only if the server read the recipe outside the recipe
/// directory.
const SECRET_MARKER: &str = "leaked-secret-marker";

/// The outside recipe, one level up from the directory being served.
const SECRET_NAME: &str = "Outside-Secret";

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
    fn outside_dir(&self) -> std::path::PathBuf {
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
///
/// Inside it, `Menus/` holds the two menus that matter to resolution: one whose
/// reference steps up into `Shared/`, and one whose reference steps up twice
/// and lands on the secret.
fn write_fixture(dir: &TempDir) {
    let root = dir.path();
    std::fs::write(
        root.join(format!("{SECRET_NAME}.cook")),
        format!("Add @{SECRET_MARKER}{{1}}.\n"),
    )
    .unwrap();

    let recipes = root.join("recipes");
    std::fs::create_dir_all(recipes.join("Shared")).unwrap();
    std::fs::create_dir_all(recipes.join("Menus")).unwrap();

    std::fs::write(recipes.join("Inside.cook"), "Mix @inside-marker{1}.\n").unwrap();
    std::fs::write(
        recipes.join("Shared").join("Sauce.cook"),
        "Simmer @shared-marker{2}.\n",
    )
    .unwrap();

    // Steps up out of `Menus/` and back down into `Shared/`, so it names a
    // recipe that is in the collection.
    std::fs::write(
        recipes.join("Menus").join("Sunday.menu"),
        "Sunday: \\\n- @../Shared/Sauce{}\n",
    )
    .unwrap();

    // Steps up twice, which from `Menus/` is one level above the recipe
    // directory: the secret.
    std::fs::write(
        recipes.join("Menus").join("Escape.menu"),
        format!("Sunday: \\\n- @../../{SECRET_NAME}{{}}\n- @./Inside{{}}\n"),
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

/// A `POST` with a JSON body and **no `Origin` header** — see the module docs.
async fn post(server: &ServerGuard, path: &str, body: &Value) -> (StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(server.url(path))
        .json(body)
        .send()
        .await
        .unwrap_or_else(|e| panic!("post {path}: {e}"));
    let status = resp.status();
    let text = resp
        .text()
        .await
        .unwrap_or_else(|e| panic!("{path} returned an unreadable body: {e}"));
    (status, text)
}

async fn get_json(server: &ServerGuard, path: &str) -> Value {
    let text = reqwest::get(server.url(path))
        .await
        .unwrap_or_else(|e| panic!("get {path}: {e}"))
        .text()
        .await
        .unwrap_or_else(|e| panic!("{path} returned an unreadable body: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} is not JSON ({e}): {text}"))
}

fn assert_refused(what: &str, status: StatusCode, body: &str) {
    assert_eq!(status, StatusCode::BAD_REQUEST, "{what}: {body}");
    assert!(
        !body.contains(SECRET_MARKER),
        "{what} reached outside the recipe directory:\n{body}"
    );
    assert!(
        body.contains("Invalid path"),
        "{what} was not refused as an invalid path:\n{body}"
    );
}

/// Every spelling that leaves the recipe directory, for one endpoint.
///
/// The absolute and UNC forms only mean anything on Windows, but they are sent
/// everywhere: elsewhere they are merely odd names that must not resolve
/// either.
fn escaping_paths(server: &ServerGuard) -> Vec<String> {
    let mut paths = vec![
        format!("../{SECRET_NAME}"),
        format!("Shared/../../{SECRET_NAME}"),
        server
            .outside_dir()
            .join(SECRET_NAME)
            .to_str()
            .expect("utf-8 temp path")
            .to_string(),
    ];
    if cfg!(windows) {
        paths.push(format!(r"..\{SECRET_NAME}"));
        paths.push(r"\\127.0.0.1\share\x.cook".to_string());
        paths.push("C:/Windows/win.ini".to_string());
    }
    paths
}

#[tokio::test]
async fn shopping_list_stays_inside_the_recipe_directory() {
    let server = start_server().await;

    let (status, body) = post(
        &server,
        "/api/shopping_list",
        &json!([{"recipe": "Inside"}]),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains("inside-marker"),
        "a recipe inside the directory must still be listed:\n{body}"
    );

    for path in escaping_paths(&server) {
        let (status, body) = post(
            &server,
            "/api/shopping_list",
            &json!([{ "recipe": path.clone() }]),
        )
        .await;
        assert_refused(&format!("POST /api/shopping_list {path:?}"), status, &body);
    }
}

#[tokio::test]
async fn add_stays_inside_the_recipe_directory() {
    let server = start_server().await;

    for path in escaping_paths(&server) {
        let (status, body) = post(
            &server,
            "/api/shopping_list/add",
            &json!({ "path": path.clone(), "scale": 1.0 }),
        )
        .await;
        assert_refused(
            &format!("POST /api/shopping_list/add {path:?}"),
            status,
            &body,
        );
    }

    // Nothing that was refused may have been written to `.shopping-list`: it is
    // read back and resolved later, and a stored escape is the same escape on a
    // delay.
    let items = get_json(&server, "/api/shopping_list/items").await;
    assert_eq!(
        items.as_array().map(Vec::len),
        Some(0),
        "a refused path must not be stored: {items}"
    );

    let (status, body) = post(
        &server,
        "/api/shopping_list/add",
        &json!({ "path": "Inside.cook", "scale": 1.0 }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "a path inside is still stored: {body}"
    );
}

#[tokio::test]
async fn add_menu_stays_inside_the_recipe_directory() {
    let server = start_server().await;

    for path in escaping_paths(&server) {
        let (status, body) = post(
            &server,
            "/api/shopping_list/add_menu",
            &json!({ "path": path.clone(), "scale": 1.0 }),
        )
        .await;
        assert_refused(
            &format!("POST /api/shopping_list/add_menu {path:?}"),
            status,
            &body,
        );
    }

    let items = get_json(&server, "/api/shopping_list/items").await;
    assert_eq!(
        items.as_array().map(Vec::len),
        Some(0),
        "a refused menu must not be stored: {items}"
    );
}

/// A menu reference is allowed to step up. `Menus/Sunday.menu` writes
/// `@../Shared/Sauce{}`, which from `Menus/` is `Shared/Sauce` — inside the
/// collection — so it goes on the list, stored as the path it resolves to
/// rather than as the `../Shared/Sauce` it was written as.
///
/// Storing the authored spelling was the bug behind the whole of this: the
/// shopping list resolves a stored path from the recipe directory, so a stored
/// `../Shared/Sauce` was looked for *beside* that directory, outside the
/// collection.
#[tokio::test]
async fn a_menu_reference_that_steps_up_is_stored_resolved() {
    let server = start_server().await;

    let (status, body) = post(
        &server,
        "/api/shopping_list/add_menu",
        &json!({ "path": "Menus/Sunday.menu", "scale": 1.0 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let items = get_json(&server, "/api/shopping_list/items").await;
    let stored = items[0]["recipes"][0]["path"]
        .as_str()
        .unwrap_or_else(|| panic!("expected a nested menu recipe: {items}"));
    assert_eq!(
        stored, "Shared/Sauce",
        "the stored path must be the resolved one: {items}"
    );

    // And it round-trips: the page posts stored paths back to
    // `/api/shopping_list`, which is where a `../` spelling used to be refused
    // by the very check this adds.
    let (status, body) = post(
        &server,
        "/api/shopping_list",
        &json!([{ "recipe": stored }]),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains("shared-marker"),
        "the resolved reference must make a list:\n{body}"
    );
}

/// A reference that steps up past the recipe directory names nothing in the
/// collection, so it is left off the menu's entry — and the references beside
/// it still go on.
#[tokio::test]
async fn a_menu_reference_climbing_out_is_left_off_the_list() {
    let server = start_server().await;

    let (status, body) = post(
        &server,
        "/api/shopping_list/add_menu",
        &json!({ "path": "Menus/Escape.menu", "scale": 1.0 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let items = get_json(&server, "/api/shopping_list/items").await;
    let stored = items[0]["recipes"]
        .as_array()
        .unwrap_or_else(|| panic!("expected nested menu recipes: {items}"))
        .iter()
        .map(|r| r["path"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        stored,
        vec!["Inside".to_string()],
        "only the reference inside the collection goes on: {items}"
    );

    let (status, body) = post(
        &server,
        "/api/shopping_list",
        &json!([{"recipe": "Menus/Escape.menu"}]),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        !body.contains(SECRET_MARKER),
        "expanding the menu must not read outside the recipe directory:\n{body}"
    );
}
