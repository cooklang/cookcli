//! End-to-end tests for the `/atom.xml` and `/rss.xml` feeds served by
//! `cook server`.

#![cfg(feature = "server")]

use reqwest::{Client, StatusCode};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

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

fn write_fixture(dir: &TempDir) {
    std::fs::create_dir_all(dir.path().join("Breakfast")).unwrap();
    std::fs::write(
        dir.path().join("Breakfast/Easy Pancakes.cook"),
        "---\ntitle: Fluffy Pancakes\ndate: 2026-01-01\ndescription: Sunday treat\ntags: [sweet]\n---\nMix @flour{100%g}.\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Omelette.cook"),
        "---\ndate: 2026-05-01\n---\nBeat @eggs{2}.\n",
    )
    .unwrap();
}

/// See `cors_test.rs`: a reserved port can be claimed by another test's server
/// before ours binds it, so retry with a fresh one.
async fn start_server(url_prefix: Option<&str>) -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server(url_prefix).await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server(url_prefix: Option<&str>) -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir);
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(dir.path())
        .arg("--port")
        .arg(port.to_string());
    if let Some(prefix) = url_prefix {
        cmd.arg("--url-prefix").arg(prefix);
    }
    let child = cmd
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };
    let url = guard.url(&format!("{}/api/menus", url_prefix.unwrap_or("")));
    let client = Client::new();
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

async fn get_text(url: String, headers: &[(&str, &str)]) -> (StatusCode, String, String) {
    let mut req = Client::new().get(url);
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    let resp = req.send().await.unwrap();
    let status = resp.status();
    let content_type = resp
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    (status, content_type, resp.text().await.unwrap())
}

#[tokio::test]
async fn serves_atom_feed_with_absolute_links() {
    let server = start_server(None).await;
    let (status, content_type, body) = get_text(server.url("/atom.xml"), &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.starts_with("application/atom+xml"));

    let base = server.url("/");
    assert!(body.contains(&format!("<id>{base}</id>")), "{body}");
    assert!(body.contains(&format!("<id>{base}recipe/Breakfast/Easy%20Pancakes</id>")));
    assert!(body.contains("<title>Fluffy Pancakes</title>"));
    assert!(body.contains("<summary>Sunday treat</summary>"));
    // Newest first: Omelette (May) before Pancakes (January).
    assert!(body.find("Omelette").unwrap() < body.find("Fluffy Pancakes").unwrap());
}

#[tokio::test]
async fn serves_rss_feed() {
    let server = start_server(None).await;
    let (status, content_type, body) = get_text(server.url("/rss.xml"), &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.starts_with("application/rss+xml"));
    assert!(body.contains("<rss version=\"2.0\""));
    assert_eq!(body.matches("<item>").count(), 2);
    let omelette = server.url("/recipe/Omelette");
    assert!(body.contains(&format!("<link>{omelette}</link>")), "{body}");
}

#[tokio::test]
async fn feed_links_follow_url_prefix_and_forwarded_proto() {
    let server = start_server(Some("/cook")).await;
    let (status, _, body) = get_text(
        server.url("/cook/atom.xml"),
        &[("x-forwarded-proto", "https")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let base = format!("https://127.0.0.1:{}/cook/", server.port);
    assert!(body.contains(&format!("<id>{base}</id>")), "{body}");
    assert!(body.contains(&format!("{base}atom.xml")));
    assert!(body.contains(&format!("{base}recipe/Omelette")));
}

#[tokio::test]
async fn feed_title_follows_accept_language() {
    let server = start_server(None).await;
    let (_, _, body) = get_text(server.url("/atom.xml"), &[("accept-language", "fr-FR")]).await;
    assert!(body.contains("xml:lang=\"fr-FR\""), "{body}");
    assert!(body.contains("<title>Toutes les Recettes</title>"));
}

#[tokio::test]
async fn pages_advertise_the_feeds() {
    let server = start_server(None).await;
    let (_, _, html) = get_text(server.url("/"), &[]).await;
    assert!(html.contains(r#"type="application/atom+xml""#));
    assert!(html.contains(r#"href="/atom.xml""#));
    assert!(html.contains(r#"href="/rss.xml""#));
}
