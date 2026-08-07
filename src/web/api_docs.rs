//! Content for the `/api-docs` page.
//!
//! Verify each entry against a running server before adding it — the examples
//! here are meant to be real payloads, not plausible ones.
//!
//! When you add or change an API route in `src/server/mod.rs`, update this
//! file: `no_documented_path_is_stale` fails if an entry names a route that
//! no longer exists in the router, and `exemptions_are_all_documented` fails
//! if a `NOT_ROUTER_REGISTERED` exemption outlives the doc entry it excuses.
//! `extracts_wrapped_routes_from_the_real_router` guards the path extractor
//! itself.
//!
//! The check compares paths only; a wrong *method* on a real path is not
//! caught. `POST /api/recipes/*path` — which does not exist — would pass
//! `no_documented_path_is_stale` cleanly, because axum registers verbs as
//! `.route(path, get(h).put(h).delete(h))` and the textual extractor never
//! sees them.
//!
//! Descriptions may use Markdown-style `inline code` spans; the template
//! renders them through the `inline_code` filter in `web::templates`.
//!
//! Examples are written as `r#"..."#` raw strings. If one ever needs to
//! contain the literal sequence `"#` (a URL fragment, say), bump to
//! `r##"..."##` rather than escaping — a plain `r#"..."#` would terminate
//! early and produce a confusing compile error far from the actual mistake.

use crate::web::templates::{ApiSection, EndpointDoc, ParamDoc};

fn section(id: &str, title: &str, description: &str, endpoints: Vec<EndpointDoc>) -> ApiSection {
    ApiSection {
        id: id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        endpoints,
    }
}

fn ep(method: &str, path: &str, summary: &str, description: &str) -> EndpointDoc {
    EndpointDoc {
        method: method.to_string(),
        path: path.to_string(),
        summary: summary.to_string(),
        description: description.to_string(),
        params: Vec::new(),
        request_example: None,
        response_example: None,
        feature: None,
    }
}

fn param(name: &str, kind: &str, type_name: &str, required: bool, description: &str) -> ParamDoc {
    ParamDoc {
        name: name.to_string(),
        kind: kind.to_string(),
        required,
        type_name: type_name.to_string(),
        description: description.to_string(),
    }
}

/// A path segment. Always required, always a string.
fn path_param(name: &str, description: &str) -> ParamDoc {
    param(name, "path", "string", true, description)
}

/// Private authoring builders for `EndpointDoc`, kept off the type's public
/// surface in `templates.rs` — see the doc comment on the struct there and
/// `method_classes`, its one public method.
impl EndpointDoc {
    fn params(mut self, params: Vec<ParamDoc>) -> Self {
        self.params = params;
        self
    }

    fn request(mut self, json: &str) -> Self {
        self.request_example = Some(json.trim().to_string());
        self
    }

    fn response(mut self, json: &str) -> Self {
        self.response_example = Some(json.trim().to_string());
        self
    }

    #[expect(dead_code, reason = "first used by the sync section, which lands last")]
    fn requires(mut self, feature: &str) -> Self {
        self.feature = Some(feature.to_string());
        self
    }
}

/// The full API reference, in display order.
pub fn sections() -> Vec<ApiSection> {
    vec![recipes()]
}

fn recipes() -> ApiSection {
    section(
        "recipes",
        "Recipes",
        "Browse, read, write and delete `.cook` files under the server's recipe directory. \
         Paths are relative to that directory and may include subdirectories.",
        vec![
            ep(
                "GET",
                "/api/recipes",
                "List every recipe as a directory tree",
                "Returns the recipe tree rooted at the server's base path. Every node — \
                 root, directory, and file alike — carries the same four keys: `children`, \
                 `name`, `path`, `recipe`. `recipe` is `null` for a directory and non-null \
                 for a file; that is the discriminator, not the key's presence. Menus \
                 (`.menu`) appear in the same tree as recipes.",
            )
            .response(
                r#"
{
  "children": {
    "Breakfast": {
      "children": {
        "Easy Pancakes": {
          "children": {},
          "name": "Easy Pancakes",
          "path": "/absolute/path/to/seed/Breakfast/Easy Pancakes.cook",
          "recipe": {
            "metadata": {
              "author": "CookCLI Team",
              "servings": 2,
              "description": "Simple crepes that are perfect for a lazy weekend breakfast."
            },
            "source": {
              "path": "/absolute/path/to/seed/Breakfast/Easy Pancakes.cook",
              "source_type": "Path"
            }
          }
        }
      },
      "name": "Breakfast",
      "path": "/absolute/path/to/seed/Breakfast",
      "recipe": null
    }
  },
  "name": "seed",
  "path": "/absolute/path/to/seed",
  "recipe": null
}
"#,
            ),
            ep(
                "GET",
                "/api/recipes/*path",
                "Read one parsed recipe",
                "Parses the recipe and returns its ingredients, cookware, timers and steps. \
                 `grouped_ingredients` aggregates repeated ingredients and indexes back into \
                 `ingredients`. `inline_quantities` is also present alongside them at the top \
                 level of `recipe`. The `image` field is a URL under `/api/static/` when the \
                 recipe has a title image, otherwise null.",
            )
            .params(vec![
                path_param("path", "Recipe path relative to the recipe directory, e.g. `Breakfast/Easy Pancakes.cook`. The `.cook` extension is optional — the server tries the bare path first, then `.cook`, then `.menu`."),
                param("scale", "query", "number", false, "Scaling factor applied during parsing. Defaults to 1. A non-numeric value returns a plain-text 400 (\"Failed to deserialize query string: ...\") from axum's query deserializer, not the page's usual JSON error envelope."),
            ])
            .response(
                r#"
{
  "image": "/api/static/Breakfast/Easy Pancakes.jpg",
  "scale": 2.0,
  "recipe": {
    "metadata": {
      "map": {
        "author": "CookCLI Team",
        "servings": 4,
        "description": "Simple crepes that are perfect for a lazy weekend breakfast."
      }
    },
    "ingredients": [
      {
        "name": "eggs",
        "alias": null,
        "note": null,
        "modifiers": "",
        "quantity": {
          "scalable": true,
          "unit": null,
          "value": { "type": "number", "value": { "type": "regular", "value": 6.0 } }
        },
        "reference": null,
        "relation": {
          "reference_target": null,
          "relation": {
            "defined_in_step": true,
            "referenced_from": [],
            "type": "definition"
          }
        }
      }
    ],
    "grouped_ingredients": [
      {
        "index": 0,
        "quantities": [
          {
            "scalable": true,
            "unit": null,
            "value": { "type": "number", "value": { "type": "regular", "value": 6.0 } }
          }
        ]
      }
    ],
    "cookware": [],
    "timers": [],
    "sections": [
      {
        "content": [
          {
            "type": "step",
            "value": {
              "items": [
                { "type": "text", "value": "Crack the " },
                { "type": "ingredient", "index": 0 },
                { "type": "text", "value": " into a blender." }
              ]
            }
          }
        ]
      }
    ]
  }
}
"#,
            ),
            ep(
                "GET",
                "/api/recipes/raw/*path",
                "Read the unparsed Cooklang source",
                "Returns the file's text verbatim with content type `text/plain`, including \
                 YAML frontmatter. The `.cook` and `.menu` extensions are optional in the path — \
                 the server tries the bare path first, then `.cook`, then `.menu`.",
            )
            .params(vec![path_param(
                "path",
                "Recipe path relative to the recipe directory.",
            )])
            .response(
                r#"
---
servings: 2
tags: breakfast, quick
author: CookCLI Team
---

Crack the @eggs{3} into a blender, then add the @flour{125%g},
@milk{250%ml} and @sea salt{pinch}, and blitz until smooth.
"#,
            ),
            ep(
                "PUT",
                "/api/recipes/*path",
                "Create or overwrite a recipe",
                "The request body is the raw Cooklang source as `text/plain` — not JSON. \
                 Writes are atomic (temp file plus rename). If the file does not exist yet, \
                 it is created with a `.cook` extension — but the response's `path` echoes \
                 the request path verbatim and does not report that resolved filename. The \
                 parent directory must already exist: writing into a directory that is not \
                 there returns a 500 whose message talks about permissions even when the \
                 real cause is the missing directory.",
            )
            .params(vec![path_param(
                "path",
                "Recipe path relative to the recipe directory.",
            )])
            .request(
                r#"
---
title: New Recipe
---

Mix the @flour{200%g} and @water{120%ml}.
"#,
            )
            .response(
                r#"
{
  "path": "Breakfast/New Recipe",
  "status": "success"
}
"#,
            ),
            ep(
                "DELETE",
                "/api/recipes/*path",
                "Delete a recipe file",
                "Permanently removes the file from disk. There is no undo and no trash.",
            )
            .params(vec![path_param(
                "path",
                "Recipe path relative to the recipe directory. The `.cook` extension is \
                 optional — the same bare → `.cook` → `.menu` resolution as the raw endpoint \
                 applies here too.",
            )])
            .response(
                r#"
{
  "status": "success",
  "path": "Breakfast/Old Recipe.cook"
}
"#,
            ),
            ep(
                "GET",
                "/api/static/*path",
                "Fetch a recipe asset",
                "Serves files straight from the recipe directory — this is where recipe images \
                 live. The `image` field returned by `GET /api/recipes/*path` is already a URL \
                 into this route.",
            )
            .params(vec![path_param(
                "path",
                "Asset path relative to the recipe directory, e.g. `Breakfast/Easy Pancakes.jpg`.",
            )]),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
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
    /// hand via the `NOT_ROUTER_REGISTERED` allowlist below; the omission
    /// here is deliberate, not a gap in this scan.
    fn router_paths(source: &str) -> BTreeSet<String> {
        const MARKER: &str = ".route(";

        // Brace-matching bounds the scan to api()'s body so a helper added
        // below api() that happens to call `.route(` is never mistaken for
        // one of its routes and given a wrong `/api` prefix. Naive about
        // braces inside string literals/comments — acceptable because api()
        // contains neither; if it ever breaks, the body comes back
        // truncated or missing and the path set is short or empty — which
        // `extracts_wrapped_routes_from_the_real_router` and
        // `no_documented_path_is_stale` both catch.
        let Some(body) = api_body(source) else {
            return BTreeSet::new();
        };

        // Routes registered in a merged or nested sub-router are invisible
        // to a textual scan of api()'s body. Rather than under-report them
        // silently — which would let the completeness check (added with the
        // sync section) pass vacuously — refuse to run at all.
        for shape in [".merge(", ".nest(", ".nest_service("] {
            assert!(
                !body.contains(shape),
                "api() uses `{shape}` — routes registered in the merged/nested router are \
                 invisible to this scan, so the completeness check (added with the sync \
                 section) would pass vacuously for them. Teach `router_paths` to follow it, \
                 or document those paths via NOT_ROUTER_REGISTERED."
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

    /// Paths that are documented but are not registered via `.route(...)`
    /// inside `api()`. Currently just the asset service, which is mounted
    /// with `.nest_service("/api/static", ...)` on the outer router.
    const NOT_ROUTER_REGISTERED: &[&str] = &["/api/static/*path"];

    fn documented_paths() -> BTreeSet<String> {
        sections()
            .iter()
            .flat_map(|s| s.endpoints.iter())
            .map(|e| e.path.clone())
            .collect()
    }

    /// `EndpointDoc::method_classes` falls back to a gray badge for any method
    /// it doesn't recognise, so a typo like "Get" degrades silently rather than
    /// failing. Across 33 hand-written entries that is worth guarding.
    #[test]
    fn all_methods_are_known_verbs() {
        const KNOWN: &[&str] = &["GET", "POST", "PUT", "DELETE"];
        for section in sections() {
            for endpoint in section.endpoints {
                assert!(
                    KNOWN.contains(&endpoint.method.as_str()),
                    "unknown HTTP method {:?} on {}",
                    endpoint.method,
                    endpoint.path
                );
            }
        }
    }

    #[test]
    fn no_documented_path_is_stale() {
        let router = router_paths(include_str!("../server/mod.rs"));
        for path in documented_paths() {
            if NOT_ROUTER_REGISTERED.contains(&path.as_str()) {
                continue;
            }
            assert!(
                router.contains(&path),
                "{path} is documented in api_docs.rs but no longer exists in the router. \
                 Remove or correct the doc entry."
            );
        }
    }

    #[test]
    fn exemptions_are_all_documented() {
        let documented = documented_paths();
        for path in NOT_ROUTER_REGISTERED {
            assert!(
                documented.contains(*path),
                "{path} is exempted from the router check but is not documented — \
                 remove the stale exemption"
            );
        }
    }

    /// `param()` takes two consecutive free-text `&str`s (`kind`, `type_name`)
    /// out of small closed vocabularies. A swapped or misspelled `kind` like
    /// `param("scale", "string", "query", false, ...)` compiles cleanly and
    /// silently renders "In: string / Type: query" — the same
    /// silent-degradation class `all_methods_are_known_verbs` guards against.
    #[test]
    fn all_param_kinds_are_known() {
        const KNOWN: &[&str] = &["path", "query", "body"];
        for section in sections() {
            for endpoint in section.endpoints {
                for p in endpoint.params {
                    assert!(
                        KNOWN.contains(&p.kind.as_str()),
                        "unknown param kind {:?} on {} {}",
                        p.kind,
                        endpoint.method,
                        endpoint.path
                    );
                }
            }
        }
    }

    /// `id` doubles as the anchor target and the TOC href; a duplicate would
    /// silently break navigation to the second section rather than fail
    /// loudly.
    #[test]
    fn section_ids_are_unique() {
        let ids: Vec<String> = sections().iter().map(|s| s.id.clone()).collect();
        let unique: BTreeSet<&String> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "duplicate section id in {ids:?}");
    }
}
