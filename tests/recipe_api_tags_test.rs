//! Integration tests for the shape of `tags` in the JSON API.
//!
//! YAML frontmatter accepts `tags: a, b` as well as `tags: [a, b]`, and the
//! parser keeps whichever type was written. Serialising that value verbatim
//! leaks the difference to clients: the comma form came back as one string and,
//! on the menu endpoint, the list form came back as `null`. Every endpoint that
//! reports metadata must report `tags` as an array of strings.
//!
//! The normalisation lives in the private `server::handlers` module, so these
//! drive the real HTTP endpoints. Each test boots `cook server` against a
//! temporary recipe directory on its own port.

#![cfg(feature = "server")]

use serde_json::Value;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    /// Held so the recipe directory outlives the server.
    #[allow(dead_code)]
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

/// One recipe and one menu per `tags` spelling: the comma-separated string and
/// the YAML sequence. Both must reach clients as the same array.
fn write_fixture(dir: &TempDir) {
    let root = dir.path();

    std::fs::write(
        root.join("String Tags.cook"),
        "---\ntitle: String Tags\ntags: tag1, tag2\n---\n\nMix @flour{100%g}.\n",
    )
    .unwrap();

    std::fs::write(
        root.join("List Tags.cook"),
        "---\ntitle: List Tags\ntags: [tag1, tag2]\n---\n\nMix @flour{100%g}.\n",
    )
    .unwrap();

    // Surrounding spaces, an empty entry and a duplicate, all of which the
    // parser drops when it reads a comma-separated `tags` string.
    std::fs::write(
        root.join("Messy Tags.cook"),
        "---\ntitle: Messy Tags\ntags: ' a ,, b , a'\n---\n\nMix @flour{100%g}.\n",
    )
    .unwrap();

    std::fs::write(
        root.join("String Tags.menu"),
        "---\ntitle: String Tags Menu\ntags: tag1, tag2\n---\n\n\
         ==Day 1==\n\nBreakfast: @./String Tags{1}\n",
    )
    .unwrap();

    std::fs::write(
        root.join("List Tags.menu"),
        "---\ntitle: List Tags Menu\ntags: [tag1, tag2]\n---\n\n\
         ==Day 1==\n\nBreakfast: @./String Tags{1}\n",
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
    let child = Command::new(assert_cmd::cargo::cargo_bin("cook"))
        .arg("server")
        .arg(dir.path())
        .arg("--port")
        .arg(port.to_string())
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

async fn get_json(server: &ServerGuard, path: &str) -> Value {
    let url = server.url(path);
    reqwest::get(&url)
        .await
        .unwrap_or_else(|e| panic!("request {path}: {e}"))
        .error_for_status()
        .unwrap_or_else(|e| panic!("{path} responded with an error: {e}"))
        .json()
        .await
        .unwrap_or_else(|e| panic!("{path} returned invalid json: {e}"))
}

/// The strings under `tags`, failing when the value is anything but an array of
/// strings — that shape is the whole point of these tests.
fn tags_of(metadata: &Value) -> Vec<String> {
    let tags = metadata
        .get("tags")
        .unwrap_or_else(|| panic!("no tags in metadata: {metadata}"));
    tags.as_array()
        .unwrap_or_else(|| panic!("tags was not an array: {tags}"))
        .iter()
        .map(|t| {
            t.as_str()
                .unwrap_or_else(|| panic!("tag was not a string: {t}"))
                .to_string()
        })
        .collect()
}

#[tokio::test]
async fn recipe_endpoint_reports_string_tags_as_an_array() {
    let server = start_server().await;
    let body = get_json(&server, "/api/recipes/String%20Tags.cook").await;

    assert_eq!(
        tags_of(&body["recipe"]["metadata"]["map"]),
        ["tag1", "tag2"]
    );
}

#[tokio::test]
async fn recipe_endpoint_leaves_list_tags_alone() {
    let server = start_server().await;
    let body = get_json(&server, "/api/recipes/List%20Tags.cook").await;

    assert_eq!(
        tags_of(&body["recipe"]["metadata"]["map"]),
        ["tag1", "tag2"]
    );
}

/// The tree endpoint carries a metadata map per recipe node, and it had the
/// same leak as the single-recipe endpoint.
#[tokio::test]
async fn tree_endpoint_reports_string_tags_as_an_array() {
    let server = start_server().await;
    let body = get_json(&server, "/api/recipes").await;

    let children = &body["children"];
    assert_eq!(
        tags_of(&children["String Tags"]["recipe"]["metadata"]),
        ["tag1", "tag2"]
    );
    assert_eq!(
        tags_of(&children["List Tags"]["recipe"]["metadata"]),
        ["tag1", "tag2"]
    );
}

/// The menu endpoint stringifies every metadata value, which turned the comma
/// form into one string and the list form into `null`.
#[tokio::test]
async fn menu_endpoint_reports_both_tag_spellings_as_an_array() {
    let server = start_server().await;

    let string_form = get_json(&server, "/api/menus/String%20Tags.menu").await;
    assert_eq!(tags_of(&string_form["metadata"]), ["tag1", "tag2"]);

    let list_form = get_json(&server, "/api/menus/List%20Tags.menu").await;
    assert_eq!(tags_of(&list_form["metadata"]), ["tag1", "tag2"]);
}

/// Duplicates, surrounding spaces and empty entries are dropped, matching how
/// the parser itself reads a comma-separated `tags` string.
#[tokio::test]
async fn string_tags_are_trimmed_and_deduplicated() {
    let server = start_server().await;
    let body = get_json(&server, "/api/recipes/Messy%20Tags.cook").await;

    assert_eq!(tags_of(&body["recipe"]["metadata"]["map"]), ["a", "b"]);
}
