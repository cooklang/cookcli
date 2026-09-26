//! End-to-end tests that the recipe file endpoints only touch recipe files.
//!
//! `GET /api/recipes/raw/{*path}`, `PUT /api/recipes/{*path}` and
//! `DELETE /api/recipes/{*path}` used to take the joined path as it was
//! whenever something existed there, so they read, overwrote and deleted any
//! file under the recipe directory: `config/aisle.conf`, `.shopping-list`, a
//! checkout's `.git/config` (#545). The path check in front of them only
//! stopped paths leaving the directory, not what they named inside it.
//!
//! Each test boots `cook server` on a directory holding a recipe and a menu
//! next to configuration, state and dot-files, then checks the endpoints still
//! work on the first two and leave the rest alone. The requests carry no
//! `Origin`, as `curl` from another machine would, so the cross-site write
//! guard does not stand in the way and the file rules are all that is tested.

#![cfg(feature = "server")]

use reqwest::StatusCode;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

const RECIPE: &str = "Mix @flour{200%g}.\n";
const MENU: &str = "== Monday ==\n\nDinner:\n- @./Pasta{}\n";

/// Every file in the fixture that is not a recipe or a menu, with what it
/// holds and a marker from that content. None of them may be read, changed or
/// removed through the API.
///
/// The marker is what a leak is looked for: a page may escape the file's line
/// breaks, but not a single word.
const OTHER_FILES: [(&str, &str, &str); 6] = [
    (
        "config/aisle.conf",
        "[aisle-marker]\nsalt\n",
        "aisle-marker",
    ),
    (
        "config/pantry.conf",
        "[pantry-marker]\nsalt = \"1%kg\"\n",
        "pantry-marker",
    ),
    (".shopping-list", "./list-marker{1}\n", "list-marker"),
    (
        ".git/config",
        "[core]\n\tfsmonitor = git-config-marker\n",
        "git-config-marker",
    ),
    (".env.local", "TOKEN=env-marker\n", "env-marker"),
    ("notes.txt", "notes-marker\n", "notes-marker"),
];

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    /// Holds the served `recipes/` directory for the server's lifetime.
    dir: TempDir,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    fn recipes(&self) -> PathBuf {
        self.dir.path().join("recipes")
    }

    fn read(&self, file: &str) -> Option<String> {
        std::fs::read_to_string(self.recipes().join(file)).ok()
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
    std::fs::create_dir_all(recipes.join("config")).unwrap();
    std::fs::create_dir_all(recipes.join(".git")).unwrap();
    std::fs::write(recipes.join("Pasta.cook"), RECIPE).unwrap();
    std::fs::write(recipes.join("Week.menu"), MENU).unwrap();
    for (file, content, _) in OTHER_FILES {
        std::fs::write(recipes.join(file), content).unwrap();
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

async fn send(
    server: &ServerGuard,
    method: reqwest::Method,
    path: &str,
    body: Option<&str>,
) -> (StatusCode, String) {
    let mut request = reqwest::Client::new().request(method.clone(), server.url(path));
    if let Some(body) = body {
        request = request
            .header(reqwest::header::CONTENT_TYPE, "text/plain")
            .body(body.to_string());
    }
    let resp = request
        .send()
        .await
        .unwrap_or_else(|e| panic!("{method} {path}: {e}"));
    let status = resp.status();
    let text = resp
        .text()
        .await
        .unwrap_or_else(|e| panic!("{method} {path} returned an unreadable body: {e}"));
    (status, text)
}

/// `file` percent-encoded as a single URL segment, the way the editor sends
/// its recipe path.
fn encoded(file: &str) -> String {
    urlencoding::encode(file).into_owned()
}

/// A path the endpoints must refuse outright: 400 for anything hidden, 404
/// for a visible file that is not a recipe.
fn expected_refusal(file: &str) -> StatusCode {
    if file.split('/').any(|component| component.starts_with('.')) {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::NOT_FOUND
    }
}

#[tokio::test]
async fn raw_reads_only_recipes_and_menus() {
    let server = start_server().await;

    for (path, expected) in [
        ("Pasta.cook", RECIPE),
        ("Pasta", RECIPE),
        ("Week.menu", MENU),
        ("Week", MENU),
    ] {
        let (status, body) = send(
            &server,
            reqwest::Method::GET,
            &format!("/api/recipes/raw/{}", encoded(path)),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{path}: {body}");
        assert_eq!(body, expected, "{path}");
    }

    for (file, _, marker) in OTHER_FILES {
        let url = format!("/api/recipes/raw/{}", encoded(file));
        let (status, body) = send(&server, reqwest::Method::GET, &url, None).await;
        assert_eq!(status, expected_refusal(file), "{url}: {body}");
        assert!(!body.contains(marker), "{url} read {file}:\n{body}");
    }
}

#[tokio::test]
async fn save_writes_only_recipes_and_menus() {
    let server = start_server().await;
    let updated = "Boil @water{1%l}.\n";

    for file in ["Pasta.cook", "Week.menu"] {
        let url = format!("/api/recipes/{}", encoded(file));
        let (status, body) = send(&server, reqwest::Method::PUT, &url, Some(updated)).await;
        assert_eq!(status, StatusCode::OK, "{url}: {body}");
        assert_eq!(server.read(file).as_deref(), Some(updated), "{file}");
    }

    // A new recipe keeps the extension it is given, and gets `.cook` without.
    for (path, created) in [
        ("Soup", "Soup.cook"),
        ("Stew.cook", "Stew.cook"),
        ("Plan.menu", "Plan.menu"),
    ] {
        let url = format!("/api/recipes/{}", encoded(path));
        let (status, body) = send(&server, reqwest::Method::PUT, &url, Some(updated)).await;
        assert_eq!(status, StatusCode::OK, "{url}: {body}");
        assert_eq!(server.read(created).as_deref(), Some(updated), "{created}");
    }

    for (file, content, _) in OTHER_FILES {
        let url = format!("/api/recipes/{}", encoded(file));
        let (status, body) = send(&server, reqwest::Method::PUT, &url, Some(updated)).await;
        assert_eq!(
            server.read(file).as_deref(),
            Some(content),
            "PUT {url} overwrote {file} ({status}: {body})"
        );
        if expected_refusal(file) == StatusCode::BAD_REQUEST {
            assert_eq!(status, StatusCode::BAD_REQUEST, "{url}: {body}");
        }
    }
}

#[tokio::test]
async fn delete_removes_only_recipes_and_menus() {
    let server = start_server().await;

    for (file, content, _) in OTHER_FILES {
        let url = format!("/api/recipes/{}", encoded(file));
        let (status, body) = send(&server, reqwest::Method::DELETE, &url, None).await;
        assert_eq!(status, expected_refusal(file), "{url}: {body}");
        assert_eq!(
            server.read(file).as_deref(),
            Some(content),
            "DELETE {url} removed {file}"
        );
    }

    for (path, file) in [("Pasta", "Pasta.cook"), ("Week.menu", "Week.menu")] {
        let url = format!("/api/recipes/{}", encoded(path));
        let (status, body) = send(&server, reqwest::Method::DELETE, &url, None).await;
        assert_eq!(status, StatusCode::OK, "{url}: {body}");
        assert_eq!(server.read(file), None, "{file} must be gone");
    }
}

/// The editor page is the front end of the save endpoint, so it opens only
/// what that endpoint would save.
#[tokio::test]
async fn editor_opens_only_recipes_and_menus() {
    let server = start_server().await;

    let (_, body) = send(&server, reqwest::Method::GET, "/edit/Pasta.cook", None).await;
    assert!(
        body.contains("flour"),
        "a recipe must still open in the editor:\n{body}"
    );

    for (file, _, marker) in OTHER_FILES {
        let url = format!("/edit/{file}");
        let (_, body) = send(&server, reqwest::Method::GET, &url, None).await;
        assert!(!body.contains(marker), "{url} opened {file}:\n{body}");
    }
}

/// `cooklang-find` opens any existing file whose name has an extension, so the
/// parsed recipe endpoint used to hand back `.env.local` as recipe steps.
#[tokio::test]
async fn parsed_recipe_endpoint_refuses_hidden_paths() {
    let server = start_server().await;

    let (status, body) = send(&server, reqwest::Method::GET, "/api/recipes/Pasta", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let url = format!("/api/recipes/{}", encoded(".env.local"));
    let (status, body) = send(&server, reqwest::Method::GET, &url, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{url}: {body}");
    assert!(!body.contains("env-marker"), "{url}: {body}");
}
