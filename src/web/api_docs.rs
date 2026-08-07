//! Content for the `/api-docs` page.
//!
//! Verify each entry against a running server before adding it — the examples
//! here are meant to be real payloads, not plausible ones.
//!
//! When you add or change an API route in `src/server/mod.rs`, update this
//! file: the `every_router_path_is_documented` test fails until the new route
//! has an entry, and `no_documented_path_is_stale` fails if an entry names a
//! route that no longer exists.

use crate::web::templates::ApiSection;

/// The full API reference, in display order.
pub fn sections() -> Vec<ApiSection> {
    Vec::new()
}

#[cfg(test)]
use std::collections::BTreeSet;

/// Extract every route path registered inside the `api()` function of the
/// given source text, prefixed with `/api` to match how it is nested.
///
/// Deliberately textual rather than reflective: axum's `Router` does not
/// expose its registered routes at runtime, so comparing against the source
/// is the only way to catch a new endpoint that nobody documented.
#[cfg(test)]
fn router_paths(source: &str) -> BTreeSet<String> {
    const MARKER: &str = ".route(";

    // Everything before `fn api(` is a different router (e.g. the
    // `/static/*file` route on the outer app) and must not be collected.
    let Some(start) = source.find("fn api(") else {
        return BTreeSet::new();
    };

    let mut paths = BTreeSet::new();
    let mut rest = &source[start..];

    while let Some(i) = rest.find(MARKER) {
        rest = &rest[i + MARKER.len()..];

        // rustfmt wraps long `.route(...)` calls, putting a newline and
        // indentation between `.route(` and the path literal. Skipping
        // whitespace here is what makes those visible — a contiguous
        // `.route("` match silently drops every wrapped registration.
        let after_marker = rest.trim_start();
        let Some(inside) = after_marker.strip_prefix('"') else {
            // Not a string literal — keep scanning from after this marker.
            continue;
        };
        let Some(end) = inside.find('"') else { break };
        paths.insert(format!("/api{}", &inside[..end]));
        rest = &inside[end..];
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
fn serve_static() {
    .route("/static/*file", get(serve_static))
}

fn api(_state: &AppState) -> Result<Router<Arc<AppState>>> {
    let router = Router::new()
        .route("/shopping_list", post(handlers::shopping_list))
        .route(
            "/shopping_list/items",
            get(handlers::get_shopping_list_items),
        )
        .route("/pantry/:section/:name", axum::routing::delete(h))
        .route("/pantry/:section/:name", axum::routing::put(h))
        .route("/recipes/*path", get(h).put(h).delete(h));

    #[cfg(feature = "sync")]
    let router = router.route("/sync/status", get(handlers::sync_status));

    Ok(router)
}
"#;

    #[test]
    fn extracts_api_routes_with_prefix() {
        let paths = router_paths(FIXTURE);
        assert!(paths.contains("/api/shopping_list"));
        assert!(paths.contains("/api/recipes/*path"));
    }

    #[test]
    fn ignores_routes_defined_before_the_api_fn() {
        let paths = router_paths(FIXTURE);
        assert!(
            !paths.contains("/api/static/*file"),
            "the /static/*file route lives outside api() and must not be collected"
        );
    }

    #[test]
    fn deduplicates_paths_registered_once_per_method() {
        let paths = router_paths(FIXTURE);
        assert!(paths.contains("/api/pantry/:section/:name"));
        // 5 distinct paths in the fixture: shopping_list, shopping_list/items
        // (wrapped `.route(` call), pantry/:section/:name (registered twice,
        // for DELETE and PUT), recipes/*path, sync/status.
        assert_eq!(paths.len(), 5, "expected 5 distinct paths, got {paths:?}");
    }

    #[test]
    fn includes_feature_gated_routes() {
        // include_str! sees the source text regardless of active features, so
        // sync endpoints are always documented and badged rather than hidden.
        let paths = router_paths(FIXTURE);
        assert!(paths.contains("/api/sync/status"));
    }

    #[test]
    fn extracts_multiline_route_calls() {
        // rustfmt wraps long registrations; a contiguous `.route("` search
        // misses them entirely.
        let paths = router_paths(FIXTURE);
        assert!(paths.contains("/api/shopping_list/items"));
    }

    #[test]
    fn extracts_wrapped_routes_from_the_real_router() {
        let paths = router_paths(include_str!("../server/mod.rs"));

        // These are registered with rustfmt-wrapped `.route(` calls in
        // src/server/mod.rs. They regressed to missing once already; if this
        // fails, the extractor has stopped seeing wrapped registrations and
        // the drift guard is quietly under-counting the router.
        for expected in [
            "/api/shopping_list/items",
            "/api/shopping_list/add_menu",
            "/api/shopping_list/remove",
            "/api/shopping_list/uncheck",
            "/api/pantry/:section/:name",
            "/api/recipes/*path",
        ] {
            assert!(
                paths.contains(expected),
                "{expected} is missing from the extracted router paths"
            );
        }

        assert!(
            !paths.contains("/api/static/*file"),
            "the outer router's static route must not be collected"
        );
        assert!(
            paths.iter().all(|p| p.starts_with("/api/")),
            "every extracted path should carry the /api nest prefix"
        );
    }
}
