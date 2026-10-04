//! The one place that decides which requests need a signed-in user, and
//! which role that user needs.

use crate::server::AppState;
use crate::web::language::FeatureFlags;
use crate::web::viewer::{Capability, Viewer};
use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

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

/// Reads that still need a capability: the editor pages and their language
/// server, and the cook.md sync controls, whose status reveals the linked
/// account and a pending device-login code.
const PROTECTED_READS: &[(&str, Capability)] = &[
    ("/new", Capability::EditRecipes),
    ("/edit", Capability::EditRecipes),
    ("/api/ws/lsp", Capability::EditRecipes),
    ("/api/sync", Capability::Administer),
];

/// Writes that need less than [`Capability::Administer`]. Any write not
/// listed here needs that, so a route added later is kept to admins until
/// someone decides otherwise.
const WRITES: &[(&str, Capability)] = &[
    ("/api/shopping_list", Capability::EditLists),
    ("/api/pantry", Capability::EditLists),
    ("/api/recipes", Capability::EditRecipes),
    ("/api/recipe_image", Capability::EditRecipes),
    ("/api/recipe_rename", Capability::EditRecipes),
    ("/api/plans", Capability::EditRecipes),
    ("/new", Capability::EditRecipes),
];

/// Whether `path` is `prefix` or lies below it: `/edit` covers `/edit/Soup`
/// but not `/editor-notes`.
fn under(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

fn lookup(table: &[(&str, Capability)], path: &str) -> Option<Capability> {
    table
        .iter()
        .find(|(prefix, _)| under(path, prefix))
        .map(|&(_, capability)| capability)
}

/// What a request needs the viewer to be allowed to do when sign-in is on,
/// or `None` when anyone may send it.
///
/// Any method that can change something needs a capability by default, so a
/// write route added later is protected without anyone remembering to list
/// it. That leaves GET handlers: they must never change anything, or they
/// have to be listed in [`PROTECTED_READS`].
///
/// `path` is the request path without the `--url-prefix`: the middleware
/// runs inside the prefix nest.
pub fn required_capability(method: &Method, path: &str) -> Option<Capability> {
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        lookup(PROTECTED_READS, path)
    } else if OPEN_WRITES.contains(&path) {
        None
    } else {
        Some(lookup(WRITES, path).unwrap_or(Capability::Administer))
    }
}

/// Works out who sent each request, hands that to the handlers as a
/// [`Viewer`] extension, and turns away anyone whose role does not allow the
/// request. Runs on every route, sign-in on or off, since the page handlers
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

    if let Some(capability) = required_capability(request.method(), request.uri().path()) {
        if !viewer.can(capability) {
            return if viewer.is_signed_in() {
                forbid(&state.url_prefix, &request, viewer)
            } else {
                refuse(&state.url_prefix, &request)
            };
        }
    }

    request.extensions_mut().insert(viewer);
    next.run(request).await
}

fn is_api(path: &str) -> bool {
    under(path, "/api")
}

/// A guest: API clients get a 401 they can act on; a browser asking for an
/// editor page is sent to sign in first and brought back afterwards.
fn refuse(url_prefix: &str, request: &Request) -> Response {
    let path = request.uri().path();
    tracing::debug!(method = %request.method(), path, "refused a request without sign-in");

    if is_api(path) {
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

/// A signed-in user whose role falls short: signing in again would not help,
/// so this is a 403, and a page says so rather than looping back to sign-in.
fn forbid(url_prefix: &str, request: &Request, viewer: Viewer) -> Response {
    let path = request.uri().path();
    tracing::debug!(
        method = %request.method(),
        path,
        role = ?viewer.role(),
        "refused a request the user's role does not allow"
    );

    if is_api(path) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Your role does not allow this change" })),
        )
            .into_response();
    }

    let lang = request
        .extensions()
        .get::<LanguageIdentifier>()
        .cloned()
        .unwrap_or(crate::web::language::EN_US);
    let features = request
        .extensions()
        .get::<FeatureFlags>()
        .copied()
        .unwrap_or_default();
    let message = crate::web::templates::Tr::new(lang.clone()).t("role-forbidden");
    let page = crate::server::ui::error_page(lang, url_prefix, message, features, viewer);
    (StatusCode::FORBIDDEN, page).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Capability::*;

    fn needs(method: Method, path: &str) -> Option<Capability> {
        required_capability(&method, path)
    }

    #[test]
    fn list_writes_need_edit_lists() {
        for (method, path) in [
            (Method::POST, "/api/shopping_list/add"),
            (Method::POST, "/api/shopping_list/add_menu"),
            (Method::POST, "/api/shopping_list/remove"),
            (Method::POST, "/api/shopping_list/check"),
            (Method::POST, "/api/shopping_list/uncheck"),
            (Method::POST, "/api/shopping_list/clear"),
            (Method::POST, "/api/shopping_list/compact"),
            (Method::POST, "/api/shopping_list/"),
            (Method::POST, "/api/pantry/add"),
            (Method::PUT, "/api/pantry/dairy/milk"),
            (Method::DELETE, "/api/pantry/dairy/milk"),
        ] {
            assert_eq!(
                needs(method.clone(), path),
                Some(EditLists),
                "{method} {path}"
            );
        }
    }

    #[test]
    fn recipe_writes_need_edit_recipes() {
        for (method, path) in [
            (Method::PUT, "/api/recipes/Soup.cook"),
            (Method::DELETE, "/api/recipes/Soup.cook"),
            (Method::PUT, "/api/recipes/Week.menu"),
            (Method::PUT, "/api/recipe_image/Soup.cook"),
            (Method::DELETE, "/api/recipe_image/Soup.cook"),
            (Method::POST, "/api/recipe_rename/Soup.cook"),
            (Method::POST, "/api/plans/Week.menu"),
            (Method::POST, "/new"),
        ] {
            assert_eq!(
                needs(method.clone(), path),
                Some(EditRecipes),
                "{method} {path}"
            );
        }
    }

    #[test]
    fn other_writes_need_an_admin() {
        for (method, path) in [
            (Method::POST, "/api/sync/login"),
            (Method::POST, "/api/sync/logout"),
            (Method::POST, "/api/sync/cancel_login"),
            (Method::PATCH, "/api/anything-added-later"),
            (Method::POST, "/api/recipes-bulk"),
            (Method::POST, "/api/recipe_renamex/Soup.cook"),
            (Method::POST, "/api/pantryx"),
            (Method::POST, "/edit/Soup.cook"),
            (Method::POST, "/loginx"),
        ] {
            assert_eq!(
                needs(method.clone(), path),
                Some(Administer),
                "{method} {path}"
            );
        }
    }

    #[test]
    fn open_writes_are_matched_exactly() {
        for path in OPEN_WRITES {
            assert_eq!(needs(Method::POST, path), None, "{path}");
        }
    }

    #[test]
    fn some_reads_need_a_capability() {
        for (path, capability) in [
            ("/new", EditRecipes),
            ("/edit/Soup.cook", EditRecipes),
            ("/api/ws/lsp", EditRecipes),
            ("/api/sync/status", Administer),
        ] {
            assert_eq!(needs(Method::GET, path), Some(capability), "{path}");
            assert_eq!(needs(Method::HEAD, path), Some(capability), "{path}");
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
            "/api/pantry",
            "/api/static/Soup.jpg",
            // Look-alikes of protected paths.
            "/newsletter",
            "/editor-notes",
            "/api/synchronized",
        ] {
            assert_eq!(needs(Method::GET, path), None, "{path}");
        }
    }
}
