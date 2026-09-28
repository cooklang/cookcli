//! The one place that decides which requests need a signed-in user.

use crate::server::AppState;
use crate::web::viewer::Viewer;
use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use std::sync::Arc;

/// Requests that change nothing even though their method says otherwise.
/// Matched exactly: `/api/shopping_list/add` is not `/api/shopping_list`.
const OPEN_WRITES: &[&str] = &[
    // Builds a shopping list from the recipes in the body and returns it.
    "/api/shopping_list",
    // A no-op kept for old clients.
    "/api/reload",
    "/login",
    "/logout",
];

/// Reads that still need a user: the editor pages, the editor's language
/// server, and the cook.md sync controls, whose status reveals the linked
/// account and a pending device-login code.
const PROTECTED_READS: &[&str] = &["/new", "/edit", "/api/ws/lsp", "/api/sync"];

/// Whether a request needs a signed-in user when sign-in is on.
///
/// Any method that can change something is refused by default, so a write
/// route added later is protected without anyone remembering to list it.
/// That leaves GET handlers: they must never change anything, or they have to
/// be listed in [`PROTECTED_READS`].
///
/// `path` is the request path without the `--url-prefix`: the middleware
/// runs inside the prefix nest.
pub fn requires_user(method: &Method, path: &str) -> bool {
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        PROTECTED_READS.iter().any(|protected| {
            path.strip_prefix(protected)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
        })
    } else {
        !OPEN_WRITES.contains(&path)
    }
}

/// Works out who sent each request, hands that to the handlers as a
/// [`Viewer`] extension, and turns guests away from anything that needs a
/// user. Runs on every route, sign-in on or off, since the page handlers
/// always extract the `Viewer`.
pub async fn middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let viewer = match &state.auth {
        Some(auth) => auth.viewer(request.headers()),
        None => Viewer::open(),
    };

    if !viewer.can_edit() && requires_user(request.method(), request.uri().path()) {
        return refuse(&state.url_prefix, &request);
    }

    request.extensions_mut().insert(viewer);
    next.run(request).await
}

/// API clients get a 401 they can act on; a browser asking for an editor
/// page is sent to sign in first and brought back afterwards.
fn refuse(url_prefix: &str, request: &Request) -> Response {
    let path = request.uri().path();
    tracing::debug!(method = %request.method(), path, "refused a request without sign-in");

    if path == "/api" || path.starts_with("/api/") {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Sign in to make changes" })),
        )
            .into_response();
    }

    let back_to = request
        .uri()
        .path_and_query()
        .map_or(path, |target| target.as_str());
    Redirect::to(&format!(
        "{url_prefix}/login?next={}",
        urlencoding::encode(back_to)
    ))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_need_a_user_by_default() {
        for (method, path) in [
            (Method::PUT, "/api/recipes/Soup.cook"),
            (Method::DELETE, "/api/recipes/Soup.cook"),
            (Method::POST, "/api/shopping_list/add"),
            (Method::POST, "/api/shopping_list/check"),
            (Method::POST, "/api/shopping_list/clear"),
            (Method::POST, "/api/pantry/add"),
            (Method::PUT, "/api/pantry/dairy/milk"),
            (Method::POST, "/api/sync/login"),
            (Method::POST, "/new"),
            (Method::PATCH, "/api/anything-added-later"),
            (Method::POST, "/api/shopping_list/"),
        ] {
            assert!(requires_user(&method, path), "{method} {path}");
        }
    }

    #[test]
    fn open_writes_are_matched_exactly() {
        for path in OPEN_WRITES {
            assert!(!requires_user(&Method::POST, path), "{path}");
        }
        assert!(requires_user(&Method::POST, "/api/shopping_list/add"));
        assert!(requires_user(&Method::POST, "/loginx"));
    }

    #[test]
    fn some_reads_need_a_user() {
        for path in ["/new", "/edit/Soup.cook", "/api/ws/lsp", "/api/sync/status"] {
            assert!(requires_user(&Method::GET, path), "{path}");
            assert!(requires_user(&Method::HEAD, path), "{path}");
        }
    }

    #[test]
    fn plain_reads_do_not() {
        for path in [
            "/",
            "/recipe/Soup.cook",
            "/directory/Mains",
            "/shopping-list",
            "/pantry",
            "/preferences",
            "/login",
            "/api/recipes",
            "/api/recipes/Soup.cook",
            "/api/shopping_list/items",
            "/api/static/Soup.jpg",
            // Look-alikes of protected paths.
            "/newsletter",
            "/editor-notes",
            "/api/synchronized",
        ] {
            assert!(!requires_user(&Method::GET, path), "{path}");
        }
    }
}
