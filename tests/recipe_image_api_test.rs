//! Integration tests for `/api/recipe_image/{*path}`: uploading, reading and
//! removing a recipe's title picture from the web editor (#520).
//!
//! Each test boots `cook server` against a temporary recipe directory on its
//! own port and checks what lands on disk, since the point of the endpoint is
//! the file `cooklang-find` picks up next to the recipe.

#![cfg(feature = "server")]

#[path = "common/mod.rs"]
mod common;

use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use serde_json::Value;
use std::io::Cursor;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

    fn file(&self, name: &str) -> std::path::PathBuf {
        self.dir.path().join(name)
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
    std::fs::write(
        dir.join("Pancakes.cook"),
        "---\ntitle: Pancakes\n---\n\nWhisk @flour{125%g} and @milk{250%ml}.\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Linked.cook"),
        "---\ntitle: Linked\nimage: https://example.com/linked.jpg\n---\n\nBoil @water{1%l}.\n",
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

fn png(width: u32, height: u32) -> Vec<u8> {
    let picture = RgbImage::from_fn(width, height, |x, y| {
        Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
    });
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(picture)
        .write_to(&mut out, ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

/// Pixel noise barely compresses, so the PNG is about as large as its raw
/// pixels: 900 × 900 × 3 is some 2.4 MB.
fn noisy_png() -> Vec<u8> {
    let mut state: u32 = 0x2545_f491;
    let picture = RgbImage::from_fn(900, 900, |_, _| {
        let mut channel = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as u8
        };
        Rgb([channel(), channel(), channel()])
    });
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(picture)
        .write_to(&mut out, ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

async fn put(server: &ServerGuard, recipe: &str, body: Vec<u8>) -> (reqwest::StatusCode, Value) {
    let resp = reqwest::Client::new()
        .put(server.url(&format!("/api/recipe_image/{recipe}")))
        .header("content-type", "application/octet-stream")
        .body(body)
        .send()
        .await
        .expect("PUT request");
    let status = resp.status();
    (status, resp.json().await.unwrap_or(Value::Null))
}

async fn get(server: &ServerGuard, recipe: &str) -> (reqwest::StatusCode, Value) {
    let resp = reqwest::get(server.url(&format!("/api/recipe_image/{recipe}")))
        .await
        .expect("GET request");
    let status = resp.status();
    (status, resp.json().await.unwrap_or(Value::Null))
}

async fn delete(server: &ServerGuard, recipe: &str) -> (reqwest::StatusCode, Value) {
    let resp = reqwest::Client::new()
        .delete(server.url(&format!("/api/recipe_image/{recipe}")))
        .send()
        .await
        .expect("DELETE request");
    let status = resp.status();
    (status, resp.json().await.unwrap_or(Value::Null))
}

fn assert_is_jpeg(file: &Path) {
    let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    assert_eq!(
        image::guess_format(&bytes).unwrap(),
        ImageFormat::Jpeg,
        "{} must hold JPEG bytes",
        file.display()
    );
}

#[tokio::test]
async fn an_upload_is_stored_as_the_recipes_jpeg_and_reported() {
    let server = start_server().await;

    let (status, body) = put(&server, "Pancakes", png(64, 32)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["source"], "file");
    assert_eq!(body["image"], "/api/static/Pancakes.jpg");
    assert_is_jpeg(&server.file("Pancakes.jpg"));

    let (status, body) = get(&server, "Pancakes.cook").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["source"], "file");
    assert_eq!(body["image"], "/api/static/Pancakes.jpg");
}

/// A JPEG that needs no resizing is still rebuilt from its pixels, so what
/// rode along after the image never reaches the recipe folder.
#[tokio::test]
async fn a_jpeg_is_stored_re_encoded_not_as_sent() {
    const SMUGGLED: &[u8] = b"<script>alert(1)</script>";
    let server = start_server().await;

    let mut sent = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(RgbImage::from_pixel(32, 32, Rgb([200, 120, 40])))
        .write_to(&mut sent, ImageFormat::Jpeg)
        .unwrap();
    let mut sent = sent.into_inner();
    sent.extend_from_slice(SMUGGLED);

    let (status, body) = put(&server, "Pancakes", sent.clone()).await;
    assert_eq!(status, 200, "{body}");

    let stored = std::fs::read(server.file("Pancakes.jpg")).unwrap();
    assert_ne!(stored, sent);
    assert!(
        !stored.windows(SMUGGLED.len()).any(|w| w == SMUGGLED),
        "the trailing data reached the disk"
    );
    assert_is_jpeg(&server.file("Pancakes.jpg"));
}

#[tokio::test]
async fn an_upload_replaces_older_pictures_but_not_step_pictures() {
    let server = start_server().await;
    for name in ["Pancakes.png", "Pancakes.webp", "Pancakes.1.jpg"] {
        std::fs::write(server.file(name), b"older picture").unwrap();
    }

    let (status, body) = put(&server, "Pancakes", png(16, 16)).await;
    assert_eq!(status, 200, "{body}");

    assert_is_jpeg(&server.file("Pancakes.jpg"));
    assert!(
        !server.file("Pancakes.png").exists(),
        "Pancakes.png left behind"
    );
    assert!(
        !server.file("Pancakes.webp").exists(),
        "Pancakes.webp left behind"
    );
    assert_eq!(
        std::fs::read(server.file("Pancakes.1.jpg")).unwrap(),
        b"older picture",
        "a step picture must not be touched"
    );
}

/// Every other route is held to 1 MiB. A photo the editor has already scaled
/// down can still pass that, and another client may send the original, so
/// this route carries its own limit, and it has to win.
#[tokio::test]
async fn a_picture_over_one_megabyte_is_accepted() {
    let server = start_server().await;
    let body = noisy_png();
    assert!(
        body.len() > 2 * 1024 * 1024,
        "fixture is only {} bytes",
        body.len()
    );

    let (status, body) = put(&server, "Pancakes", body).await;
    assert_eq!(status, 200, "{body}");
    assert_is_jpeg(&server.file("Pancakes.jpg"));
}

/// The server reads a body until it passes the limit, then answers 413 and
/// hangs up. Exactly one byte over, over a raw socket, so every byte sent has
/// been read by then: anything left unread would make the server's close a
/// reset, which can swallow the response before the test reads it.
#[tokio::test]
async fn a_body_over_the_upload_limit_is_refused() {
    const LIMIT: usize = 10 * 1024 * 1024;
    let server = start_server().await;

    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", server.port))
        .await
        .expect("connect");
    let request = format!(
        "PUT /api/recipe_image/Pancakes HTTP/1.1\r\n\
         Host: 127.0.0.1:{}\r\n\
         Content-Type: image/jpeg\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        server.port,
        LIMIT + 1
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("send headers");
    stream
        .write_all(&vec![0; LIMIT + 1])
        .await
        .expect("send body");

    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(60), stream.read_to_end(&mut response))
        .await
        .expect("the server must answer")
        .expect("read response");
    let response = String::from_utf8_lossy(&response);
    assert!(
        response.starts_with("HTTP/1.1 413"),
        "expected 413, got: {}",
        response.lines().next().unwrap_or_default()
    );
    assert!(!server.file("Pancakes.jpg").exists());
}

#[tokio::test]
async fn unreadable_uploads_are_refused_and_nothing_is_written() {
    let server = start_server().await;

    let (status, body) = put(&server, "Pancakes", b"not a picture".to_vec()).await;
    assert_eq!(status, 415, "{body}");
    assert_eq!(body["code"], "unsupported");

    let mut heic = b"\0\0\0\x18ftypheic\0\0\0\0mif1heic".to_vec();
    heic.extend_from_slice(&[0; 64]);
    let (status, body) = put(&server, "Pancakes", heic).await;
    assert_eq!(status, 415, "{body}");
    assert_eq!(body["code"], "heif");
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("Most Compatible"),
        "the HEIC refusal should say how to get a JPEG: {body}"
    );

    let (status, body) = put(&server, "Pancakes", Vec::new()).await;
    assert_eq!(status, 400, "{body}");

    for name in [
        "Pancakes.jpg",
        "Pancakes.jpeg",
        "Pancakes.png",
        "Pancakes.webp",
    ] {
        assert!(
            !server.file(name).exists(),
            "{name} must not have been written"
        );
    }
}

#[tokio::test]
async fn a_missing_recipe_or_an_escaping_path_is_refused() {
    let server = start_server().await;

    let (status, _) = put(&server, "Waffles", png(8, 8)).await;
    assert_eq!(status, 404);
    let (status, _) = get(&server, "Waffles").await;
    assert_eq!(status, 404);
    assert!(!server.file("Waffles.jpg").exists());

    let (status, _) = put(&server, "..%2FPancakes", png(8, 8)).await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn delete_removes_the_picture_then_reports_there_is_none() {
    let server = start_server().await;
    std::fs::write(server.file("Pancakes.jpg"), b"picture").unwrap();
    std::fs::write(server.file("Pancakes.1.jpg"), b"step").unwrap();

    let (status, body) = delete(&server, "Pancakes").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["image"], Value::Null);
    assert_eq!(body["source"], Value::Null);
    assert!(!server.file("Pancakes.jpg").exists());
    assert!(
        server.file("Pancakes.1.jpg").exists(),
        "a step picture must not be removed"
    );

    let (status, _) = delete(&server, "Pancakes").await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn a_picture_named_in_the_metadata_is_reported_as_such() {
    let server = start_server().await;

    let (status, body) = get(&server, "Linked").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["source"], "metadata");
    assert_eq!(body["image"], "https://example.com/linked.jpg");
}

/// A picture replaced under the same name keeps its URL, so without this the
/// browser may go on showing the old one from its cache.
#[tokio::test]
async fn recipe_files_are_served_for_revalidation() {
    let server = start_server().await;

    let resp = reqwest::get(server.url("/api/static/Pancakes.cook"))
        .await
        .expect("GET static file");
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("no-cache")
    );
}
