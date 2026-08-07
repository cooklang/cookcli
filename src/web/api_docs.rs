//! Content for the `/api-docs` page.
//!
//! Verify each entry against a running server before adding it — the examples
//! here are meant to be real payloads, not plausible ones.
//!
//! When you add or change an API route in `src/server/mod.rs`, update this
//! file: the `every_router_path_is_documented` test fails until the new route
//! has an entry, and `no_documented_path_is_stale` fails if an entry names a
//! route that no longer exists. `extracts_wrapped_routes_from_the_real_router`
//! guards the extractor itself today, ahead of those two tests landing.

use crate::web::templates::ApiSection;

/// The full API reference, in display order.
pub fn sections() -> Vec<ApiSection> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    /// The text of `api()`'s body, braces included, or `None` if it can't be
    /// located. Bounding the scan matters because anything after `api()`
    /// belongs to a different router and must not be given the `/api`
    /// prefix.
    fn api_body(source: &str) -> Option<&str> {
        let start = source.find("fn api(")?;
        let rest = &source[start..];
        let open = rest.find('{')?;

        let mut depth = 0usize;
        for (i, c) in rest[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&rest[open..=open + i]);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Extract every route path registered inside the `api()` function of the
    /// given source text, prefixed with `/api` to match how it is nested.
    ///
    /// Deliberately textual rather than reflective: axum's `Router` does not
    /// expose its registered routes at runtime, so comparing against the
    /// source is the only way to catch a new endpoint that nobody
    /// documented.
    ///
    /// This does not, and cannot, see `.nest_service("/api/static", ...)` on
    /// the *outer* router in `src/server/mod.rs` — it isn't a `.route(` call
    /// and isn't inside `api()`. That endpoint is documented and exempted by
    /// hand via a `NOT_ROUTER_REGISTERED` allowlist (see Task 3); the
    /// omission here is deliberate, not a gap in this scan.
    fn router_paths(source: &str) -> BTreeSet<String> {
        const MARKER: &str = ".route(";

        // Brace-matching bounds the scan to api()'s body so a helper added
        // below api() that happens to call `.route(` is never mistaken for
        // one of its routes and given a wrong `/api` prefix. Naive about
        // braces inside string literals/comments — acceptable because api()
        // contains neither; if it ever breaks, the body comes back
        // truncated or missing and the path set is short or empty — which
        // `extracts_wrapped_routes_from_the_real_router` catches
        // immediately, and `no_documented_path_is_stale` catches again once
        // Task 3 lands.
        let Some(body) = api_body(source) else {
            return BTreeSet::new();
        };

        // Routes registered in a merged or nested sub-router are invisible
        // to a textual scan of api()'s body. Rather than under-report them
        // silently — which would let `every_router_path_is_documented` pass
        // vacuously — refuse to run at all.
        for shape in [".merge(", ".nest(", ".nest_service("] {
            assert!(
                !body.contains(shape),
                "api() uses `{shape}` — routes registered in the merged/nested router are \
                 invisible to this scan, so `every_router_path_is_documented` would pass \
                 vacuously for them. Teach `router_paths` to follow it, or document those \
                 paths via NOT_ROUTER_REGISTERED."
            );
        }

        let mut paths = BTreeSet::new();
        let mut rest = body;

        while let Some(i) = rest.find(MARKER) {
            rest = &rest[i + MARKER.len()..];

            // rustfmt wraps long `.route(...)` calls, putting a newline and
            // indentation between `.route(` and the path literal. Skipping
            // whitespace here is what makes those visible — a contiguous
            // `.route("` match silently drops every wrapped registration.
            let after_marker = rest.trim_start();
            let Some(inside) = after_marker.strip_prefix('"') else {
                let preview: String = after_marker.chars().take(60).collect();
                panic!(
                    "a `.route(` in api() is not followed by a string literal, near: {preview:?}\n\
                     The extractor only understands plain string paths. A const path or a raw \
                     string literal would be skipped silently, letting an undocumented endpoint \
                     through the drift guard. Teach `router_paths` this shape rather than \
                     removing the check."
                );
            };
            let Some(end) = inside.find('"') else {
                unreachable!("unterminated string literal in api() — source would not compile")
            };
            paths.insert(format!("/api{}", &inside[..end]));
            rest = &inside[end..];
        }

        paths
    }

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

    #[test]
    fn ignores_routes_defined_after_the_api_fn() {
        const TRAILING: &str = r#"
fn api(_state: &AppState) -> Result<Router<Arc<AppState>>> {
    let router = Router::new().route("/inside", get(h));
    Ok(router)
}

async fn something_else() {
    other.route("/outside", get(h));
}
"#;
        let paths = router_paths(TRAILING);
        assert!(paths.contains("/api/inside"));
        assert!(
            !paths.contains("/api/outside"),
            "routes outside api() are not nested under /api and must not be collected"
        );
    }

    #[test]
    #[should_panic(expected = "not followed by a string literal")]
    fn rejects_route_shapes_it_cannot_read() {
        const CONST_PATH: &str = r#"
fn api(_state: &AppState) -> Result<Router<Arc<AppState>>> {
    let router = Router::new().route(ROUTES_RECIPES, get(h));
    Ok(router)
}
"#;
        let _ = router_paths(CONST_PATH);
    }

    #[test]
    #[should_panic(expected = "invisible to this scan")]
    fn rejects_merged_sub_routers() {
        const MERGED: &str = r#"
fn api(_s: &A) -> R {
    let router = Router::new()
        .route("/stats", get(h))
        .merge(sync_router());
    Ok(router)
}
fn sync_router() -> Router { Router::new().route("/sync/status", get(h)) }
"#;
        let _ = router_paths(MERGED);
    }
}
