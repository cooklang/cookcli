//! Integration tests for `POST /api/recipe_rename/{*path}`: renaming a recipe
//! or menu from the web editor.
//!
//! A rename touches several files at once — the recipe, its pictures, the
//! recipes and menus naming it — so each test boots `cook server` on a
//! collection that also holds files it must never touch, and compares the
//! whole directory, byte for byte, before and after.

#![cfg(feature = "server")]

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

const PASTA: &str = "---\ntitle: Pasta\n---\n\nBoil @pasta{200%g} in @water{2%l}.\n";
const WEEK: &str = "== Monday ==\n\nDinner:\n- @./Pasta{2}\n- @./Soup{}\n\n\
                    -- @./Pasta{} stays: a comment\n";
const SALAD: &str = "Serve with @../Pasta{1}(warm).\n";
const ELSEWHERE: &str = "A @./Sides/Pasta{} that is not ours, and @./Pastas{}.\n";

/// The fixture: what the tests rename, what names it, and what must be left
/// alone whatever happens.
const FILES: [(&str, &[u8]); 16] = [
    ("Pasta.cook", PASTA.as_bytes()),
    ("Pasta.jpg", b"title picture"),
    ("Pasta.1.jpg", b"step 1"),
    ("Pasta.1.2.png", b"section 1 step 2"),
    // Pasta.2.jpg is the title picture of Pasta.2.cook as much as step 2 of
    // Pasta.cook, so it stays.
    ("Pasta.2.cook", b"Fry @eggs{2}.\n"),
    ("Pasta.2.jpg", b"shared picture"),
    ("Soup.cook", b"Simmer @leeks{3}.\n"),
    ("Brunch.menu", b"Sunday:\n- @./Pasta.2{}\n"),
    ("Plans/Week.menu", WEEK.as_bytes()),
    ("Sides/Salad.cook", SALAD.as_bytes()),
    ("Sides/Elsewhere.cook", ELSEWHERE.as_bytes()),
    ("config/aisle.conf", b"[pasta]\npasta\n"),
    (".shopping-list", b"./Pasta{1}\n"),
    (".git/config", b"[core]\n\tbare = false\n"),
    ("notes.txt", b"Remember @./Pasta{} for Monday.\n"),
    ("Plans/notes.md", b"@./Pasta{}\n"),
];

struct ServerGuard {
    child: Child,
    port: u16,
    dir: TempDir,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.path(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
    }

    fn exists(&self, name: &str) -> bool {
        self.path(name).symlink_metadata().is_ok()
    }

    /// Every file under the collection with its bytes.
    fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    let name = path.strip_prefix(root).unwrap();
                    let name = name.to_string_lossy().replace('\\', "/");
                    out.insert(name, std::fs::read(&path).unwrap_or_default());
                }
            }
        }
        let mut files = BTreeMap::new();
        walk(self.dir.path(), self.dir.path(), &mut files);
        files
    }

    async fn rename(&self, path: &str, name: &str) -> (reqwest::StatusCode, Value) {
        let response = reqwest::Client::new()
            .post(self.url(&format!("/api/recipe_rename/{path}")))
            .header("Content-Type", "application/json")
            .body(json!({ "name": name }).to_string())
            .send()
            .await
            .expect("rename request");
        let status = response.status();
        let body = response.text().await.unwrap();
        (
            status,
            serde_json::from_str(&body).unwrap_or(Value::String(body)),
        )
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

fn write_fixture(dir: &Path) {
    for (name, contents) in FILES {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }
}

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
    write_fixture(dir.path());

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(dir.path())
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
    for _ in 0..600 {
        if guard.child.try_wait().expect("poll server").is_some() {
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
async fn a_rename_moves_the_pictures_and_rewrites_the_references() {
    let server = start_server().await;
    let before = server.snapshot();

    let (status, body) = server.rename("Pasta.cook", "Fresh Noodles").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["path"], "Fresh Noodles.cook");
    assert_eq!(
        body["references_updated"],
        json!(["Plans/Week.menu", "Sides/Salad.cook"])
    );
    assert_eq!(body["references_skipped"], json!([]));
    assert_eq!(body["references_failed"], json!([]));
    assert_eq!(body["shopping_list_stale"], true);

    let mut expected = before.clone();
    for (old, new) in [
        ("Pasta.cook", "Fresh Noodles.cook"),
        ("Pasta.jpg", "Fresh Noodles.jpg"),
        ("Pasta.1.jpg", "Fresh Noodles.1.jpg"),
        ("Pasta.1.2.png", "Fresh Noodles.1.2.png"),
    ] {
        let contents = expected.remove(old).unwrap();
        expected.insert(new.to_string(), contents);
    }
    expected.insert(
        "Plans/Week.menu".to_string(),
        WEEK.replacen("@./Pasta{2}", "@./Fresh Noodles{2}", 1)
            .into_bytes(),
    );
    expected.insert(
        "Sides/Salad.cook".to_string(),
        SALAD
            .replace("@../Pasta{1}", "@../Fresh Noodles{1}")
            .into_bytes(),
    );
    assert_eq!(server.snapshot(), expected);

    // The renamed recipe is served under its new name, and the menu reaches it.
    let recipe = reqwest::get(server.url("/api/recipes/Fresh%20Noodles"))
        .await
        .unwrap();
    assert_eq!(recipe.status(), 200);
    let menu: Value = reqwest::get(server.url("/api/menus/Plans/Week.menu"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(menu.to_string().contains("Fresh Noodles"), "{menu}");
}

#[tokio::test]
async fn a_menu_is_renamed_as_a_menu() {
    let server = start_server().await;

    let (status, body) = server.rename("Plans/Week", "Week 2.menu").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["path"], "Plans/Week 2.menu");
    assert_eq!(server.read("Plans/Week 2.menu"), WEEK);
    assert!(!server.exists("Plans/Week.menu"));

    let (status, body) = server.rename("Plans/Week 2.menu", "Week 3.cook").await;
    assert_eq!(status, 400, "{body}");
    assert!(server.exists("Plans/Week 2.menu"));
}

#[tokio::test]
async fn names_that_are_not_a_plain_file_name_are_refused_and_nothing_changes() {
    let server = start_server().await;
    let before = server.snapshot();

    for name in [
        "",
        "../Pasta",
        "../../escaped",
        "Sides/Pasta",
        "Sides\\Pasta",
        "/tmp/Pasta",
        ".hidden",
        ".shopping-list",
        "config/aisle.conf",
        "Pasta.menu",
        "CON",
        "nul.cook",
        "Pasta.",
        "Pasta{2}",
        "Pasta -- quick",
        "Pas\u{202E}ta",
        "Pasta",
        "Pasta.cook",
    ] {
        let (status, body) = server.rename("Pasta.cook", name).await;
        assert_eq!(status, 400, "{name:?}: {body}");
    }
    assert_eq!(server.snapshot(), before);
}

#[tokio::test]
async fn a_rename_never_overwrites_anything() {
    let server = start_server().await;
    // A picture with no recipe yet: Pasta.1.jpg would land on it.
    std::fs::write(server.path("Stew.1.jpg"), b"taken").unwrap();
    // A recipe whose title picture Pasta.1.jpg would become.
    std::fs::write(server.path("Bake.1.cook"), b"Bake.\n").unwrap();
    let before = server.snapshot();

    for (name, why) in [
        ("Soup", "a recipe of that name"),
        ("Pasta.2", "Pasta.2.cook"),
        ("Brunch", "a menu that @./Brunch{} would stop reaching"),
        ("Stew", "Stew.1.jpg is taken"),
        ("Bake", "Bake.1.jpg would be Bake.1.cook's title picture"),
    ] {
        let (status, body) = server.rename("Pasta.cook", name).await;
        assert_eq!(status, 409, "{name:?} ({why}): {body}");
    }
    assert_eq!(server.snapshot(), before);
}

#[tokio::test]
async fn only_existing_recipe_files_can_be_renamed() {
    let server = start_server().await;
    let before = server.snapshot();

    for (path, expected) in [
        ("Missing.cook", 404),
        ("notes.txt", 404),
        ("config/aisle.conf", 404),
        ("Plans", 404),
        (".git/config", 400),
        (".shopping-list", 400),
        ("..%2F..%2Fetc%2Fpasswd", 400),
    ] {
        let (status, body) = server.rename(path, "Renamed").await;
        assert_eq!(status, expected, "{path}: {body}");
    }
    assert_eq!(server.snapshot(), before);
}

#[tokio::test]
async fn references_that_cannot_be_rewritten_exactly_are_reported_not_guessed() {
    let server = start_server().await;
    std::fs::write(
        server.path("Sides/Odd.cook"),
        "Escaped @./Pas\\ta{}.\nSpaced @./Pasta  {}.\nCased @./pasta{}.\n",
    )
    .unwrap();

    let (status, body) = server.rename("Pasta.cook", "Penne").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        server.read("Sides/Odd.cook"),
        "Escaped @./Pas\\ta{}.\nSpaced @./Penne  {}.\nCased @./pasta{}.\n",
        "trailing spaces before the braces are not part of the name"
    );
    assert_eq!(
        body["references_skipped"],
        json!([
            { "file": "Sides/Odd.cook", "line": 1 },
            { "file": "Sides/Odd.cook", "line": 3 },
        ])
    );
}

#[cfg(unix)]
#[tokio::test]
async fn links_are_never_renamed_or_written_through() {
    let server = start_server().await;
    let outside = TempDir::new().unwrap();
    let outside_menu = outside.path().join("Shared.menu");
    std::fs::write(&outside_menu, "- @./Pasta{}\n").unwrap();
    std::os::unix::fs::symlink(server.path("Soup.cook"), server.path("Alias.cook")).unwrap();
    std::os::unix::fs::symlink(&outside_menu, server.path("Shared.menu")).unwrap();
    std::os::unix::fs::symlink(outside.path(), server.path("Outside")).unwrap();
    std::fs::write(outside.path().join("Far.cook"), "Far away.\n").unwrap();

    let (status, body) = server.rename("Alias.cook", "Other").await;
    assert_eq!(status, 400, "{body}");
    let (status, body) = server.rename("Outside/Far.cook", "Near").await;
    assert_eq!(status, 400, "{body}");
    assert!(outside.path().join("Far.cook").exists());

    let (status, body) = server.rename("Pasta.cook", "Penne").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        std::fs::read_to_string(&outside_menu).unwrap(),
        "- @./Pasta{}\n",
        "a file outside the collection is never written"
    );
    assert!(
        body["references_skipped"]
            .as_array()
            .unwrap()
            .iter()
            .any(|skipped| skipped["file"] == "Shared.menu"),
        "{body}"
    );
}

#[tokio::test]
async fn references_are_found_in_every_file_whatever_its_title() {
    let server = start_server().await;
    // Two recipes with one title, and a recipe beside a folder of its name: a
    // walk keyed by title, as `cooklang_find::build_tree` is, sees one of each.
    for (name, text) in [
        ("Twins/A.cook", "---\ntitle: Same\n---\n\nUse @./Pasta{}.\n"),
        ("Twins/B.cook", "---\ntitle: Same\n---\n\nUse @./Pasta{}.\n"),
        ("Plans.cook", "Use @./Pasta{}.\n"),
    ] {
        let path = server.path(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    let (status, body) = server.rename("Pasta.cook", "Penne").await;
    assert_eq!(status, 200, "{body}");
    for name in [
        "Twins/A.cook",
        "Twins/B.cook",
        "Plans.cook",
        "Plans/Week.menu",
    ] {
        assert!(server.read(name).contains("@./Penne{"), "{name}");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_collection_that_cannot_be_read_in_full_is_not_renamed() {
    use std::os::unix::fs::PermissionsExt;

    let server = start_server().await;
    let locked = server.path("Locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::write(locked.join("Uses.cook"), "Use @./Pasta{}.\n").unwrap();
    let before = server.snapshot();

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read_dir(&locked).is_ok() {
        // Running as root: nothing is unreadable.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let (status, body) = server.rename("Pasta.cook", "Penne").await;
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(status, 500, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("Nothing was renamed"),
        "{body}"
    );
    assert_eq!(server.snapshot(), before);
}
