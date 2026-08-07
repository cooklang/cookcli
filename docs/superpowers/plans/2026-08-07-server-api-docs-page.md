# Server API Documentation Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `/api-docs` page to the CookCLI web server documenting all ~33 HTTP API endpoints, linked from the preferences page.

**Architecture:** Endpoint documentation is authored as Rust data in `src/web/api_docs.rs` and rendered by an Askama template that loops over it. A unit test cross-checks the documented paths against the `.route("…")` literals in `src/server/mod.rs` so the docs cannot silently drift from the router.

**Tech Stack:** Rust, axum 0.7, Askama templates, Tailwind CSS, Playwright for e2e.

**Design spec:** `docs/superpowers/specs/2026-08-07-server-api-docs-page-design.md`

---

## Background for the implementer

**All example payloads in this plan were captured from a live server** (`cook server ./seed --port 9099`) on 2026-08-07. Do not rewrite them from imagination. Where a payload was trimmed for length, the plan says so explicitly and the trim is marked in the JSON with a `…` comment line.

**Repo conventions you must follow:**
- Commit messages use Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`). Release automation depends on this.
- Before the final commit: `cargo fmt`, `cargo clippy`, `cargo test` must all pass cleanly.
- `cargo build` needs compiled front-end assets to exist (`static/css/output.css`, `static/js/editor.bundle.js`). They are checked in. If a build fails complaining about missing assets, run `npm install && make css && make js`.
- The `server` feature is in `default`, so a plain `cargo build` includes it. Note that a previously-built binary in `target/` may have been built *without* it — if `cook server --help` says "unrecognized subcommand", rebuild.

**Two structural facts that matter:**
1. `src/web/` is compiled even when the `server` feature is off (it is shared with `cook build`'s static site export). Everything added for this page is server-only and must be gated with `#[cfg(feature = "server")]`.
2. The `/api-docs` route goes in `src/server/ui.rs`, which only the server uses. That is why no `static_mode` flag is needed on the page itself — but the *preferences* template does render in static mode, so the link to it must be gated in the template.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/web/templates.rs` (modify) | Add `ApiSection`, `EndpointDoc`, `ParamDoc` data types and the `ApiDocsTemplate` Askama struct. Types only — no content. |
| `src/web/api_docs.rs` (create) | All endpoint content, the builder helpers that keep it readable, and the router-drift test. This is the only file that changes when an endpoint changes. |
| `src/web/mod.rs` (modify) | Register the new module. |
| `templates/api_docs.html` (create) | Presentation: intro card, table of contents, endpoint cards. Loops over the data; contains no endpoint knowledge. |
| `src/server/ui.rs` (modify) | `/api-docs` route + `api_docs_page` handler. |
| `templates/preferences.html` (modify) | Link into the existing "Documentation & Resources" card. |
| `tests/e2e/api-docs.spec.ts` (create) | Playwright coverage: page renders, anchors resolve, preferences link works. |

---

## Task 1: Documentation data types

**Files:**
- Modify: `src/web/templates.rs` (append at end of file)
- Modify: `src/web/mod.rs:32-36`

- [ ] **Step 1: Add the data types to `src/web/templates.rs`**

Append to the end of the file:

```rust
// -- API documentation page --

/// One parameter of a documented endpoint.
#[cfg(feature = "server")]
pub struct ParamDoc {
    pub name: String,
    /// Where the parameter goes: "path", "query", or "body".
    pub kind: String,
    pub required: bool,
    /// Human-facing type, e.g. "string", "number", "string[]".
    pub type_name: String,
    pub description: String,
}

/// One documented API endpoint.
#[cfg(feature = "server")]
pub struct EndpointDoc {
    pub method: String,
    /// Path in axum route syntax, e.g. "/api/pantry/:section/:name".
    pub path: String,
    pub summary: String,
    /// Longer prose. Empty string means "no extra detail".
    pub description: String,
    pub params: Vec<ParamDoc>,
    /// Pretty-printed JSON request body, if the endpoint takes one.
    pub request_example: Option<String>,
    /// Pretty-printed JSON response body, if the endpoint returns one.
    pub response_example: Option<String>,
    /// Cargo feature required for this endpoint to exist, e.g. "sync".
    pub feature: Option<String>,
}

#[cfg(feature = "server")]
impl EndpointDoc {
    /// Tailwind classes for the method badge. Kept here rather than in the
    /// template so the template needs no conditional chain per endpoint.
    pub fn method_classes(&self) -> &'static str {
        match self.method.as_str() {
            "GET" => "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
            "POST" => "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
            "PUT" => "bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200",
            "DELETE" => "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200",
            _ => "bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200",
        }
    }
}

/// A group of related endpoints, rendered as one section with an anchor.
#[cfg(feature = "server")]
pub struct ApiSection {
    /// Anchor target, e.g. "shopping-list".
    pub id: String,
    pub title: String,
    pub description: String,
    pub endpoints: Vec<EndpointDoc>,
}

```

**`ApiDocsTemplate` is deliberately NOT added here.** It carries `#[derive(Template)] #[template(path = "api_docs.html")]`, and Askama resolves that path at compile time — adding it before `templates/api_docs.html` exists breaks the build. It moves to Task 10, alongside the template file it needs.

- [ ] **Step 1b: Let Tailwind see Rust source**

`method_classes()` above is the first place in this repo that names Tailwind classes from Rust. `tailwind.config.js` only scans `./templates/` and `./static/`, so those class names get purged from the compiled CSS and every method badge renders unstyled. Add the glob:

```js
  content: [
    "./templates/**/*.{html,js}",
    "./static/**/*.{html,js}",
    "./src/**/*.rs",
  ],
```

Then rebuild and prove it:

```bash
make css
for c in bg-blue-100 bg-green-100 bg-amber-100 text-amber-800 text-red-800 \
         'dark\\:bg-blue-900' 'dark\\:text-amber-200' 'dark\\:text-red-200'; do
  printf "%-24s %s\n" "$c" "$(grep -c "\.$c" static/css/output.css)"
done
```

Every count must be non-zero.

**Grep the escaped form for `dark:` variants.** Tailwind escapes the colon in the emitted selector, so it writes `.dark\:bg-blue-900`. Searching for the literal `dark:bg-blue-900` returns 0 whether or not the class is present — a false negative that will make a working build look broken.

`static/css/output.css` is gitignored, so only the config change is committed.

A known, accepted cost of the `./src/**/*.rs` glob: Tailwind scans `.rs` as undifferentiated text, so bare identifiers that happen to match utility names get emitted as dead rules (`to_lowercase()` yields `.lowercase`; the word "grow" yields `.grow`). That is ~40 bytes of inert CSS in a 46 KB file, and no element ever carries those classes. Narrowing to `./src/web/**/*.rs` would shrink the surface but would silently reintroduce the purge bug the day someone emits a class name from `src/server/`. The broad glob fails safe; keep it.

- [ ] **Step 2: Register the module in `src/web/mod.rs`**

Change the module list (currently lines 32-36) to:

```rust
pub mod builders;
mod i18n;
pub mod language;
pub mod menus;
pub mod templates;

/// API reference content for the `/api-docs` page. Server-only.
#[cfg(feature = "server")]
pub mod api_docs;
```

- [ ] **Step 3: Create a placeholder `src/web/api_docs.rs` so the crate compiles**

```rust
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
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: finishes with no errors. `dead_code` warnings about the new types are expected at this point and clear in Task 10.

- [ ] **Step 5: Commit**

```bash
git add src/web/templates.rs src/web/mod.rs src/web/api_docs.rs
git commit -m "feat(server): add data types for the API documentation page"
```

---

## Task 2: Router path extractor

This is the machinery behind the drift test. It reads `src/server/mod.rs` as text and pulls out every route path. Written test-first because the string handling has real edge cases.

**Files:**
- Modify: `src/web/api_docs.rs`

- [ ] **Step 1: Write the failing test**

Append to `src/web/api_docs.rs`:

```rust
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
        // 4 distinct paths in the fixture: shopping_list, pantry/:section/:name
        // (registered twice, for DELETE and PUT), recipes/*path, sync/status.
        assert_eq!(paths.len(), 4, "expected 4 distinct paths, got {paths:?}");
    }

    #[test]
    fn includes_feature_gated_routes() {
        // include_str! sees the source text regardless of active features, so
        // sync endpoints are always documented and badged rather than hidden.
        let paths = router_paths(FIXTURE);
        assert!(paths.contains("/api/sync/status"));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib api_docs`
Expected: FAIL — `cannot find function 'router_paths' in this scope`

- [ ] **Step 3: Implement `router_paths`**

Insert into `src/web/api_docs.rs`, above the `#[cfg(test)]` module:

```rust
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
    const MARKER: &str = ".route(\"";

    // Everything before `fn api(` is a different router (e.g. the
    // `/static/*file` route on the outer app) and must not be collected.
    let Some(start) = source.find("fn api(") else {
        return BTreeSet::new();
    };

    let mut paths = BTreeSet::new();
    let mut rest = &source[start..];

    while let Some(i) = rest.find(MARKER) {
        rest = &rest[i + MARKER.len()..];
        let Some(end) = rest.find('"') else { break };
        paths.insert(format!("/api{}", &rest[..end]));
        rest = &rest[end..];
    }

    paths
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib api_docs`
Expected: PASS — 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/web/api_docs.rs
git commit -m "feat(server): extract router paths from source for API doc drift checks"
```

---

## Task 3: Content builders and the Recipes section

**Files:**
- Modify: `src/web/api_docs.rs`

- [ ] **Step 1: Add the builder helpers**

In `src/web/api_docs.rs`, **replace** the existing `use crate::web::templates::ApiSection;` line with the block below (one `use` statement, not two — `cargo fmt` will not merge them for you), placing it above `pub fn sections()`:

```rust
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

    fn requires(mut self, feature: &str) -> Self {
        self.feature = Some(feature.to_string());
        self
    }
}
```

Note: the `impl EndpointDoc` block here is separate from the one in `templates.rs`. Rust allows multiple inherent impl blocks for a type within the same crate, and keeping these private builders next to the content is intentional — they are an authoring convenience, not part of the type's public surface.

- [ ] **Step 2: Add the Recipes section**

Replace the body of `pub fn sections()` with:

```rust
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
                "Returns the recipe tree rooted at the server's base path. Directories \
                 become `children` entries; a node with a `recipe` field is a file. \
                 Menus (`.menu`) appear in the same tree as recipes.",
            )
            .response(
                r#"
{
  "children": {
    "2 Day Plan": {
      "children": {},
      "name": "2 Day Plan",
      "path": "/absolute/path/to/seed/2 Day Plan.menu",
      "recipe": {
        "metadata": { "servings": 2 },
        "source": {
          "path": "/absolute/path/to/seed/2 Day Plan.menu",
          "source_type": "Path"
        }
      }
    }
  }
}
"#,
            ),
            ep(
                "GET",
                "/api/recipes/*path",
                "Read one parsed recipe",
                "Parses the recipe and returns its ingredients, cookware, timers and steps. \
                 `grouped_ingredients` aggregates repeated ingredients and indexes back into \
                 `ingredients`. The `image` field is a URL under `/api/static/` when the recipe \
                 has a title image, otherwise null.",
            )
            .params(vec![
                param("path", "path", "string", true, "Recipe path relative to the recipe directory, e.g. `Breakfast/Easy Pancakes.cook`."),
                param("scale", "query", "number", false, "Scaling factor applied during parsing. Defaults to 1."),
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
        "reference": null
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
            .params(vec![param(
                "path",
                "path",
                "string",
                true,
                "Recipe path relative to the recipe directory.",
            )])
            .response(
                r#"
---
servings: 2
tags: breakfast, quick
author: CookCLI Team
---

Crack the @eggs{6} into a blender.
"#,
            ),
            ep(
                "PUT",
                "/api/recipes/*path",
                "Create or overwrite a recipe",
                "The request body is the raw Cooklang source as `text/plain` — not JSON. \
                 Writes are atomic (temp file plus rename). If the file does not exist yet, \
                 it is created with a `.cook` extension.",
            )
            .params(vec![param(
                "path",
                "path",
                "string",
                true,
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
  "status": "success",
  "path": "Breakfast/New Recipe.cook"
}
"#,
            ),
            ep(
                "DELETE",
                "/api/recipes/*path",
                "Delete a recipe file",
                "Permanently removes the file from disk. There is no undo and no trash.",
            )
            .params(vec![param(
                "path",
                "path",
                "string",
                true,
                "Recipe path relative to the recipe directory.",
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
            .params(vec![param(
                "path",
                "path",
                "string",
                true,
                "Asset path relative to the recipe directory, e.g. `Breakfast/Easy Pancakes.jpg`.",
            )]),
        ],
    )
}
```

- [ ] **Step 3: Write the "no stale docs" test**

Add to the `#[cfg(test)] mod tests` block in `src/web/api_docs.rs`:

```rust
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
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib api_docs`
Expected: PASS — 6 tests. If `no_documented_path_is_stale` fails, a path string in the Recipes section does not match the router literal exactly (check `*path` vs `*file` and the `raw` segment).

- [ ] **Step 5: Commit**

```bash
git add src/web/api_docs.rs
git commit -m "feat(server): document recipe API endpoints"
```

---

## Task 4: Menus section

**Files:**
- Modify: `src/web/api_docs.rs`

- [ ] **Step 1: Add the section function**

Append to `src/web/api_docs.rs` (below `fn recipes()`):

```rust
fn menus() -> ApiSection {
    section(
        "menus",
        "Menus",
        "`.menu` files group recipes into meal plans. These endpoints return a menu's \
         structure — days, meals, and the recipes and loose ingredients in each.",
        vec![
            ep(
                "GET",
                "/api/menus",
                "List every menu",
                "Walks the recipe tree and returns only `.menu` files.",
            )
            .response(
                r#"
[
  { "name": "Weekly Plan", "path": "Weekly Plan.menu" },
  { "name": "2 Day Plan", "path": "2 Day Plan.menu" }
]
"#,
            ),
            ep(
                "GET",
                "/api/menus/*path",
                "Read one menu",
                "Sections correspond to days; a `date` is extracted when the section name \
                 contains one in parentheses, e.g. `Day 1 (2026-03-04)`. Meal items are \
                 tagged by `kind`: `recipe_reference` points at another file, `ingredient` \
                 is a loose item written directly in the menu. Returns 400 if the path is \
                 not a menu file.",
            )
            .params(vec![
                param("path", "path", "string", true, "Menu path relative to the recipe directory, e.g. `2 Day Plan.menu`."),
                param("scale", "query", "number", false, "Scaling factor applied to the whole menu. Defaults to 1."),
            ])
            .response(
                r#"
{
  "name": "2 Day Plan",
  "path": "2 Day Plan.menu",
  "metadata": { "servings": "2" },
  "sections": [
    {
      "name": "Day 1",
      "date": null,
      "meals": [
        {
          "type": "Breakfast",
          "time": null,
          "items": [
            {
              "kind": "recipe_reference",
              "name": "./Breakfast/Easy Pancakes",
              "path": "./Breakfast/Easy Pancakes.cook",
              "scale": 10.0
            },
            {
              "kind": "ingredient",
              "name": "maple syrup",
              "quantity": "2",
              "unit": "tbsp"
            }
          ]
        }
      ]
    }
  ]
}
"#,
            ),
        ],
    )
}
```

- [ ] **Step 2: Register the section**

Change `pub fn sections()` to:

```rust
pub fn sections() -> Vec<ApiSection> {
    vec![recipes(), menus()]
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib api_docs`
Expected: PASS — 6 tests, `no_documented_path_is_stale` now covering the two menu paths.

- [ ] **Step 4: Commit**

```bash
git add src/web/api_docs.rs
git commit -m "feat(server): document menu API endpoints"
```

---

## Task 5: Shopping List section

**Files:**
- Modify: `src/web/api_docs.rs`

- [ ] **Step 1: Add the section function**

Append to `src/web/api_docs.rs`:

```rust
fn shopping_list() -> ApiSection {
    section(
        "shopping-list",
        "Shopping List",
        "Two distinct things live here. `POST /api/shopping_list` is stateless: send recipes, \
         get an aggregated ingredient list back. Everything else operates on the server's \
         persistent list, stored as `.shopping-list` and `.shopping-checked` in the recipe \
         directory.",
        vec![
            ep(
                "POST",
                "/api/shopping_list",
                "Aggregate ingredients across recipes",
                "Stateless — nothing is stored. Ingredients with the same name are combined and \
                 unit-converted, grouped into aisle categories from `aisle.conf`, and reduced by \
                 anything in `pantry.conf`. Ingredients with no category land in `other`, sorted \
                 alphabetically. `pantry_items` lists names that were found in the pantry and \
                 subtracted; `checked` reflects the persistent checked state.",
            )
            .params(vec![
                param("recipe", "body", "string", true, "Recipe path. The array may hold several."),
                param("scale", "body", "number", false, "Scaling factor for this recipe. Defaults to 1."),
                param("included_references", "body", "string[]", false, "Which sub-recipe references to expand. Omit to include all of them."),
            ])
            .request(
                r#"
[
  { "recipe": "Neapolitan Pizza", "scale": 2 },
  { "recipe": "Salads/Caprese", "included_references": ["Shared/Vinaigrette"] }
]
"#,
            )
            .response(
                r#"
{
  "categories": [
    {
      "category": "milk and dairy",
      "items": [
        {
          "name": "mozzarella cheese",
          "quantities": [
            {
              "scalable": false,
              "unit": "g",
              "value": { "type": "number", "value": { "type": "regular", "value": 200.0 } }
            }
          ]
        }
      ]
    }
  ],
  "pantry_items": ["flour", "water", "salt", "olive oil"],
  "checked": []
}
"#,
            ),
            ep(
                "GET",
                "/api/shopping_list/items",
                "Read the stored recipe list",
                "Returns the recipes currently on the shopping list, not their ingredients. \
                 An entry with a `recipes` array is a menu added via `add_menu`; its nested \
                 entries carry their own resolved scale.",
            )
            .response(
                r#"
[
  {
    "path": "Salads/Caprese.cook",
    "name": "Caprese",
    "scale": 1.0,
    "included_references": ["Shared/Vinaigrette"]
  },
  {
    "path": "Breakfast/Easy Pancakes.cook",
    "name": "Easy Pancakes",
    "scale": 1.0
  }
]
"#,
            ),
            ep(
                "POST",
                "/api/shopping_list/add",
                "Add one recipe to the stored list",
                "Responds `200 OK` with an empty body. The display name is derived from the \
                 path server-side; a client-supplied name would be discarded, so it is not \
                 accepted.",
            )
            .params(vec![
                param("path", "body", "string", true, "Recipe path relative to the recipe directory."),
                param("scale", "body", "number", true, "Scaling factor to store with the entry."),
                param("included_references", "body", "string[]", false, "Which sub-recipe references to expand. Omit to include all."),
            ])
            .request(
                r#"
{
  "path": "Salads/Caprese.cook",
  "scale": 2.0,
  "included_references": ["Shared/Vinaigrette"]
}
"#,
            ),
            ep(
                "POST",
                "/api/shopping_list/add_menu",
                "Add every recipe in a menu",
                "Stored as a single entry with the menu's recipes nested inside. Each nested \
                 recipe's scale is resolved from the menu reference: a bare `{2}` is a raw \
                 multiplier, `{3%servings}` targets 3 servings against the recipe's own \
                 `servings` metadata, and any other unit targets its `yield` metadata. \
                 Responds `200 OK` with an empty body; returns 404 if the menu is not found.",
            )
            .params(vec![
                param("path", "body", "string", true, "Menu path relative to the recipe directory."),
                param("scale", "body", "number", true, "Scaling factor applied to the whole menu."),
            ])
            .request(
                r#"
{
  "path": "2 Day Plan.menu",
  "scale": 1.0
}
"#,
            ),
            ep(
                "POST",
                "/api/shopping_list/remove",
                "Remove one recipe from the stored list",
                "Also compacts the checked log, dropping checks for ingredients no longer \
                 referenced by any remaining recipe. Responds `200 OK` with an empty body.",
            )
            .params(vec![param(
                "path",
                "body",
                "string",
                true,
                "Recipe path exactly as stored.",
            )])
            .request(
                r#"
{ "path": "Salads/Caprese.cook" }
"#,
            ),
            ep(
                "POST",
                "/api/shopping_list/clear",
                "Empty the stored list",
                "Removes every recipe and all checked state. Responds `200 OK` with an empty body.",
            ),
            ep(
                "POST",
                "/api/shopping_list/check",
                "Mark an ingredient as bought",
                "The name must match an aggregated ingredient name as returned by \
                 `POST /api/shopping_list`. Responds `200 OK` with an empty body.",
            )
            .params(vec![param(
                "name",
                "body",
                "string",
                true,
                "Aggregated ingredient name.",
            )])
            .request(
                r#"
{ "name": "mozzarella cheese" }
"#,
            ),
            ep(
                "POST",
                "/api/shopping_list/uncheck",
                "Clear an ingredient's bought mark",
                "Responds `200 OK` with an empty body.",
            )
            .params(vec![param(
                "name",
                "body",
                "string",
                true,
                "Aggregated ingredient name.",
            )])
            .request(
                r#"
{ "name": "mozzarella cheese" }
"#,
            ),
            ep(
                "GET",
                "/api/shopping_list/checked",
                "List checked ingredient names",
                "",
            )
            .response(
                r#"
["mozzarella cheese", "tipo zero flour"]
"#,
            ),
            ep(
                "POST",
                "/api/shopping_list/compact",
                "Drop stale checked entries",
                "Re-aggregates the current list and removes checks for ingredients that are no \
                 longer in it. Refuses to compact (500) if any recipe fails to parse, rather \
                 than wiping checks based on a partial ingredient set. Responds `200 OK` with \
                 an empty body.",
            ),
        ],
    )
}
```

- [ ] **Step 2: Register the section**

```rust
pub fn sections() -> Vec<ApiSection> {
    vec![recipes(), menus(), shopping_list()]
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib api_docs`
Expected: PASS — 6 tests.

- [ ] **Step 4: Commit**

```bash
git add src/web/api_docs.rs
git commit -m "feat(server): document shopping list API endpoints"
```

---

## Task 6: Pantry section

**Files:**
- Modify: `src/web/api_docs.rs`

- [ ] **Step 1: Add the section function**

Append to `src/web/api_docs.rs`:

```rust
fn pantry() -> ApiSection {
    section(
        "pantry",
        "Pantry",
        "Reads and writes `pantry.conf`, a TOML file of what you already have at home. \
         Quantities use Cooklang's `VALUE%UNIT` form, e.g. `250%g`. Every endpoint here \
         returns 404 when no pantry file is configured.",
        vec![
            ep(
                "GET",
                "/api/pantry",
                "Read the whole pantry",
                "Top-level keys are section names — you choose them; `fridge` and `garden` \
                 are just conventions. Item fields other than `name` are all optional.",
            )
            .response(
                r#"
{
  "fridge": [
    { "name": "butter", "quantity": "250%g", "expire": "2026-04-15" },
    { "name": "eggs", "quantity": "12", "bought": "2026-03-07" },
    { "name": "milk", "quantity": "2%l" }
  ],
  "garden": [
    { "name": "fresh basil", "quantity": "unlim" }
  ]
}
"#,
            ),
            ep(
                "POST",
                "/api/pantry/add",
                "Add an item",
                "Creates the section if it does not exist. Duplicate names are not merged — \
                 adding an existing name appends a second entry.",
            )
            .params(vec![
                param("section", "body", "string", true, "Section to add the item to."),
                param("name", "body", "string", true, "Item name."),
                param("quantity", "body", "string", false, "Amount as `VALUE%UNIT`, or `unlim` for unlimited."),
                param("bought", "body", "string", false, "Purchase date. Accepts `YYYY-MM-DD`, `DD.MM.YYYY`, `DD/MM/YYYY`, `MM/DD/YYYY`, `YYYY.MM.DD` or `DD-MM-YYYY`."),
                param("expire", "body", "string", false, "Expiry date, same accepted formats as `bought`."),
                param("low", "body", "string", false, "Threshold below which the item counts as running low."),
            ])
            .request(
                r#"
{
  "section": "fridge",
  "name": "butter",
  "quantity": "250%g",
  "expire": "2026-04-15"
}
"#,
            )
            .response(
                r#"
{
  "success": true,
  "message": "Added butter to fridge"
}
"#,
            ),
            ep(
                "PUT",
                "/api/pantry/:section/:name",
                "Update an item",
                "Only the fields present in the body are changed; omitted fields keep their \
                 current values. Returns 404 if the section does not exist.",
            )
            .params(vec![
                param("section", "path", "string", true, "Section containing the item."),
                param("name", "path", "string", true, "Item name."),
                param("quantity", "body", "string", false, "New amount."),
                param("bought", "body", "string", false, "New purchase date."),
                param("expire", "body", "string", false, "New expiry date."),
                param("low", "body", "string", false, "New low threshold."),
            ])
            .request(
                r#"
{ "quantity": "500%g" }
"#,
            )
            .response(
                r#"
{
  "success": true,
  "message": "Updated butter in fridge"
}
"#,
            ),
            ep(
                "DELETE",
                "/api/pantry/:section/:name",
                "Remove an item",
                "The section is deleted too if it becomes empty. Returns 404 if the section \
                 does not exist.",
            )
            .params(vec![
                param("section", "path", "string", true, "Section containing the item."),
                param("name", "path", "string", true, "Item name."),
            ])
            .response(
                r#"
{
  "success": true,
  "message": "Removed butter from fridge"
}
"#,
            ),
            ep(
                "GET",
                "/api/pantry/expiring",
                "List items expiring soon",
                "Sorted most urgent first. `days_remaining` goes negative for items that have \
                 already expired, and those are always included regardless of the window.",
            )
            .params(vec![param(
                "days",
                "query",
                "number",
                false,
                "Look-ahead window in days. Defaults to 7. Negative values return 400.",
            )])
            .response(
                r#"
[
  {
    "section": "fridge",
    "name": "butter",
    "expire": "2026-04-15",
    "days_remaining": -114
  }
]
"#,
            ),
            ep(
                "GET",
                "/api/pantry/depleted",
                "List items running low",
                "An item counts as low when its quantity has fallen to or below its `low` \
                 threshold.",
            )
            .response(
                r#"
[
  { "section": "fridge", "name": "milk", "low": "1%l" }
]
"#,
            ),
        ],
    )
}
```

- [ ] **Step 2: Register the section**

```rust
pub fn sections() -> Vec<ApiSection> {
    vec![recipes(), menus(), shopping_list(), pantry()]
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib api_docs`
Expected: PASS — 6 tests. Both `PUT` and `DELETE` on `/api/pantry/:section/:name` map to the one router path, which the `BTreeSet` handles.

- [ ] **Step 4: Commit**

```bash
git add src/web/api_docs.rs
git commit -m "feat(server): document pantry API endpoints"
```

---

## Task 7: Search & Stats section

**Files:**
- Modify: `src/web/api_docs.rs`

- [ ] **Step 1: Add the section function**

Append to `src/web/api_docs.rs`:

```rust
fn search_and_stats() -> ApiSection {
    section(
        "search-stats",
        "Search & Stats",
        "Collection-wide queries.",
        vec![
            ep(
                "GET",
                "/api/search",
                "Full-text recipe search",
                "Matches against recipe names and content. Menus are searched alongside \
                 recipes. Returns an empty array rather than 404 when nothing matches.",
            )
            .params(vec![param(
                "q",
                "query",
                "string",
                true,
                "Search term. Omitting it returns 400.",
            )])
            .response(
                r#"
[
  { "name": "Neapolitan Pizza", "path": "Neapolitan Pizza.cook" },
  { "name": "Pizza Dough", "path": "Shared/Pizza Dough.cook" },
  { "name": "2 Day Plan", "path": "2 Day Plan.menu" }
]
"#,
            ),
            ep(
                "GET",
                "/api/stats",
                "Collection counts",
                "Pantry counts are all zero when no pantry file is configured. \
                 `pantry_expiring_count` uses a fixed 7-day window.",
            )
            .response(
                r#"
{
  "recipe_count": 12,
  "menu_count": 2,
  "pantry_item_count": 29,
  "pantry_expiring_count": 1,
  "pantry_depleted_count": 0
}
"#,
            ),
            ep(
                "GET",
                "/api/reload",
                "Reload recipes (no-op)",
                "Kept for client compatibility. The server reads from disk on every request, \
                 so there is no cache to clear and this endpoint does nothing.",
            )
            .response(
                r#"
{
  "status": "success",
  "message": "Recipes will be refreshed from disk on next request"
}
"#,
            ),
            ep(
                "POST",
                "/api/reload",
                "Reload recipes (no-op)",
                "Identical to the GET form; both verbs are accepted.",
            )
            .response(
                r#"
{
  "status": "success",
  "message": "Recipes will be refreshed from disk on next request"
}
"#,
            ),
        ],
    )
}
```

- [ ] **Step 2: Register the section**

```rust
pub fn sections() -> Vec<ApiSection> {
    vec![
        recipes(),
        menus(),
        shopping_list(),
        pantry(),
        search_and_stats(),
    ]
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib api_docs`
Expected: PASS — 6 tests.

- [ ] **Step 4: Commit**

```bash
git add src/web/api_docs.rs
git commit -m "feat(server): document search and stats API endpoints"
```

---

## Task 8: Realtime section

**Files:**
- Modify: `src/web/api_docs.rs`

- [ ] **Step 1: Add the section function**

Append to `src/web/api_docs.rs`:

```rust
fn realtime() -> ApiSection {
    section(
        "realtime",
        "Realtime",
        "Long-lived connections. Neither of these returns a normal JSON response.",
        vec![
            ep(
                "GET",
                "/api/shopping_list/events",
                "Server-sent events for shopping list changes",
                "Emits a `change` event whenever `.shopping-list` or `.shopping-checked` is \
                 modified on disk — including by another client or by the `cook` CLI. Events \
                 carry only which file changed, so the intended pattern is to re-fetch the \
                 list on each event rather than to apply a diff. A `ping` keep-alive comment \
                 is sent every 30 seconds. If the filesystem watcher failed to start, the \
                 stream connects normally but stays silent.",
            )
            .response(
                r#"
event: change
data: {"file":"ShoppingList"}
"#,
            ),
            ep(
                "GET",
                "/api/ws/lsp",
                "Language server bridge (websocket)",
                "Upgrades to a websocket that bridges to the Cooklang language server, \
                 providing diagnostics and completions to the built-in editor. Messages are \
                 Language Server Protocol messages — see the LSP specification for the format. \
                 Not a REST endpoint and not usable with a plain HTTP client.",
            ),
        ],
    )
}
```

- [ ] **Step 2: Register the section**

```rust
pub fn sections() -> Vec<ApiSection> {
    vec![
        recipes(),
        menus(),
        shopping_list(),
        pantry(),
        search_and_stats(),
        realtime(),
    ]
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib api_docs`
Expected: PASS — 6 tests.

- [ ] **Step 4: Commit**

```bash
git add src/web/api_docs.rs
git commit -m "feat(server): document realtime API endpoints"
```

---

## Task 9: Sync section and the completeness test

This task closes the loop: with every endpoint documented, the second direction of the drift test can be turned on.

**Files:**
- Modify: `src/web/api_docs.rs`

- [ ] **Step 1: Add the section function**

Append to `src/web/api_docs.rs`:

```rust
fn sync() -> ApiSection {
    section(
        "sync",
        "Sync",
        "Sign in to CookCloud and sync recipes across devices. These endpoints exist only \
         when CookCLI is built with the `sync` feature, which is on by default. \
         Authentication uses the OAuth device flow: start a login, show the user the code, \
         then poll status until it completes.",
        vec![
            ep(
                "GET",
                "/api/sync/status",
                "Current sync and login state",
                "`pending_login` is non-null while a device-code login is in progress; poll \
                 this endpoint to detect completion. `expires_in_secs` counts down.",
            )
            .requires("sync")
            .response(
                r#"
{
  "logged_in": false,
  "email": null,
  "syncing": false,
  "pending_login": null
}
"#,
            ),
            ep(
                "POST",
                "/api/sync/login",
                "Start a device-code login",
                "Show `user_code` to the user and send them to `verification_uri`, or open \
                 `verification_uri_complete` which pre-fills the code. Returns 400 if already \
                 logged in or if a login is already in progress, and 502 if cook.md is \
                 unreachable. Takes no request body.",
            )
            .requires("sync")
            .response(
                r#"
{
  "user_code": "ABCD-EFGH",
  "verification_uri": "https://cook.md/device",
  "verification_uri_complete": "https://cook.md/device?code=ABCD-EFGH",
  "expires_in_secs": 900
}
"#,
            ),
            ep(
                "POST",
                "/api/sync/cancel_login",
                "Abandon a pending login",
                "`cancelled` is false when there was no login in progress. Takes no request body.",
            )
            .requires("sync")
            .response(
                r#"
{ "cancelled": true }
"#,
            ),
            ep(
                "POST",
                "/api/sync/logout",
                "Sign out and stop syncing",
                "Clears the stored session and halts the background sync task. Takes no \
                 request body.",
            )
            .requires("sync")
            .response(
                r#"
{ "ok": true }
"#,
            ),
        ],
    )
}
```

- [ ] **Step 2: Register the section**

```rust
pub fn sections() -> Vec<ApiSection> {
    vec![
        recipes(),
        menus(),
        shopping_list(),
        pantry(),
        search_and_stats(),
        realtime(),
        sync(),
    ]
}
```

- [ ] **Step 3: Write the failing completeness test**

Add to the `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn every_router_path_is_documented() {
        let documented = documented_paths();
        for path in router_paths(include_str!("../server/mod.rs")) {
            assert!(
                documented.contains(&path),
                "{path} is registered in the router but missing from api_docs.rs. \
                 Add a doc entry for it."
            );
        }
    }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib api_docs`
Expected: PASS — 6 tests.

If `every_router_path_is_documented` fails, the named path exists in `src/server/mod.rs` but has no matching entry. Either a route was added since this plan was written, or a path string has a typo. Fix the doc entry rather than the assertion.

- [ ] **Step 5: Verify the test actually catches drift**

Temporarily add a fake route to `src/server/mod.rs` inside `api()`, immediately after the `.route("/stats", ...)` line:

```rust
        .route("/definitely_not_documented", get(handlers::stats))
```

Run: `cargo test --lib api_docs`
Expected: FAIL with `/api/definitely_not_documented is registered in the router but missing from api_docs.rs`

Now revert that line:

```bash
git checkout src/server/mod.rs
```

Run: `cargo test --lib api_docs`
Expected: PASS — 6 tests. This confirms the guard is real and not vacuously passing.

- [ ] **Step 6: Commit**

```bash
git add src/web/api_docs.rs
git commit -m "feat(server): document sync API endpoints and enforce doc coverage"
```

---

## Task 10: The page template, route and handler

**Files:**
- Create: `templates/api_docs.html`
- Modify: `src/server/ui.rs:34-44` (router) and end of file (handler)

- [ ] **Step 1: Create `templates/api_docs.html`**

```html
{% extends "base.html" %}

{% block title %}Server API - Cook{% endblock %}

{% block content %}
<div class="max-w-5xl">
    <h1 class="text-2xl font-bold mb-2">Server API</h1>
    <p class="text-gray-600 dark:text-gray-400 mb-6">
        HTTP endpoints for building integrations against this CookCLI server.
    </p>

    <!-- Ground rules -->
    <div class="bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-gray-800 dark:to-gray-800 p-6 rounded-2xl border-2 border-blue-200 dark:border-gray-700 mb-8">
        <h2 class="text-lg font-semibold mb-3 text-blue-900 dark:text-blue-200">Before you start</h2>
        <div class="text-sm space-y-2 text-gray-700 dark:text-gray-300">
            <div>
                <span class="font-medium">Base URL:</span>
                <code class="ml-2 px-2 py-0.5 rounded bg-white dark:bg-gray-900 font-mono">{{ base_url }}</code>
            </div>
            <div>
                <span class="font-medium">Authentication:</span>
                <span class="ml-2">None. Anyone who can reach the server can read and modify your recipes — think twice before using <code class="font-mono">--host</code> on an untrusted network.</span>
            </div>
            <div>
                <span class="font-medium">CORS:</span>
                <span class="ml-2">All origins are allowed, for the methods GET, POST, PUT and DELETE.</span>
            </div>
            <div>
                <span class="font-medium">Request size limit:</span>
                <span class="ml-2">1 MB.</span>
            </div>
            <div>
                <span class="font-medium">Content type:</span>
                <span class="ml-2">JSON in and out, except where noted — raw recipe text is <code class="font-mono">text/plain</code>.</span>
            </div>
        </div>
    </div>

    <!-- Errors -->
    <div class="bg-gray-50 dark:bg-gray-800 p-6 rounded-2xl border border-gray-200 dark:border-gray-700 mb-8">
        <h2 class="text-lg font-semibold mb-3">Errors</h2>
        <p class="text-sm text-gray-700 dark:text-gray-300 mb-3">
            Every failure returns the same shape, with the status code carrying the meaning:
        </p>
        <pre class="bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-3 text-xs overflow-x-auto font-mono">{ "error": "Recipe not found: Nope.cook" }</pre>
        <ul class="mt-3 text-sm space-y-1 text-gray-700 dark:text-gray-300">
            <li><span class="font-mono font-medium">400</span> — malformed input: an invalid path, a bad query parameter, or a recipe that failed to parse.</li>
            <li><span class="font-mono font-medium">404</span> — the recipe, menu, or pantry section does not exist, or no pantry file is configured.</li>
            <li><span class="font-mono font-medium">500</span> — the server could not read or write a file.</li>
        </ul>
    </div>

    <!-- Contents -->
    <div class="bg-gray-50 dark:bg-gray-800 p-6 rounded-2xl border border-gray-200 dark:border-gray-700 mb-8">
        <h2 class="text-lg font-semibold mb-3">Contents</h2>
        <div class="flex flex-wrap gap-2">
            {% for section in sections %}
            <a href="#{{ section.id }}"
               class="px-3 py-1.5 rounded-lg text-sm font-medium bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-200 hover:border-orange-400 hover:bg-orange-50 dark:hover:bg-gray-700 transition-colors">
                {{ section.title }}
            </a>
            {% endfor %}
        </div>
    </div>

    <!-- Endpoints -->
    {% for section in sections %}
    <section id="{{ section.id }}" class="mb-10 scroll-mt-24">
        <h2 class="text-xl font-bold mb-2">{{ section.title }}</h2>
        <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">{{ section.description }}</p>

        <div class="space-y-4">
            {% for endpoint in section.endpoints %}
            <article class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl p-5">
                <div class="flex flex-wrap items-center gap-2 mb-2">
                    <span class="px-2 py-0.5 rounded text-xs font-bold font-mono {{ endpoint.method_classes() }}">{{ endpoint.method }}</span>
                    <code class="font-mono text-sm break-all">{{ endpoint.path }}</code>
                    {% if let Some(feature) = endpoint.feature %}
                    <span class="px-2 py-0.5 rounded text-xs font-medium bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200">
                        requires <span class="font-mono">{{ feature }}</span> build
                    </span>
                    {% endif %}
                </div>

                <p class="text-sm font-medium mb-1">{{ endpoint.summary }}</p>
                {% if !endpoint.description.is_empty() %}
                <p class="text-sm text-gray-600 dark:text-gray-400">{{ endpoint.description }}</p>
                {% endif %}

                {% if !endpoint.params.is_empty() %}
                <div class="mt-4 overflow-x-auto">
                    <table class="w-full text-sm">
                        <thead>
                            <tr class="text-left text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400 border-b border-gray-200 dark:border-gray-700">
                                <th class="py-1.5 pr-4 font-medium">Name</th>
                                <th class="py-1.5 pr-4 font-medium">In</th>
                                <th class="py-1.5 pr-4 font-medium">Type</th>
                                <th class="py-1.5 font-medium">Description</th>
                            </tr>
                        </thead>
                        <tbody>
                            {% for p in endpoint.params %}
                            <tr class="border-b border-gray-100 dark:border-gray-700 align-top">
                                <td class="py-1.5 pr-4 font-mono whitespace-nowrap">
                                    {{ p.name }}{% if p.required %}<span class="text-red-600 dark:text-red-400" title="required">*</span>{% endif %}
                                </td>
                                <td class="py-1.5 pr-4 text-gray-500 dark:text-gray-400">{{ p.kind }}</td>
                                <td class="py-1.5 pr-4 font-mono text-gray-500 dark:text-gray-400 whitespace-nowrap">{{ p.type_name }}</td>
                                <td class="py-1.5 text-gray-600 dark:text-gray-400">{{ p.description }}</td>
                            </tr>
                            {% endfor %}
                        </tbody>
                    </table>
                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                        <span class="text-red-600 dark:text-red-400">*</span> required
                    </p>
                </div>
                {% endif %}

                {% if let Some(request) = endpoint.request_example %}
                <div class="mt-4">
                    <h4 class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400 mb-1">Request body</h4>
                    <pre class="bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-3 text-xs overflow-x-auto font-mono">{{ request }}</pre>
                </div>
                {% endif %}

                {% if let Some(response) = endpoint.response_example %}
                <div class="mt-4">
                    <h4 class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400 mb-1">Response</h4>
                    <pre class="bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-3 text-xs overflow-x-auto font-mono">{{ response }}</pre>
                </div>
                {% endif %}
            </article>
            {% endfor %}
        </div>
    </section>
    {% endfor %}
</div>
{% endblock %}
```

The `{% if let Some(x) = ... %}` form works on `Option<String>` in the Askama version this repo pins (0.12.1) — `templates/pantry.html:64` already relies on it.

- [ ] **Step 2: Add the route in `src/server/ui.rs`**

Change the `ui()` function (currently lines 34-44) to add one line:

```rust
pub fn ui() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(recipes_page))
        .route("/directory/*path", get(recipes_directory))
        .route("/recipe/*path", get(recipe_page))
        .route("/edit/*path", get(edit_page))
        .route("/new", get(new_page).post(create_recipe))
        .route("/shopping-list", get(shopping_list_page))
        .route("/pantry", get(pantry_page))
        .route("/preferences", get(preferences_page))
        .route("/api-docs", get(api_docs_page))
}
```

- [ ] **Step 3: Add the handler at the end of `src/server/ui.rs`**

```rust
async fn api_docs_page(
    State(state): State<Arc<AppState>>,
    Host(host): Host,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> impl askama_axum::IntoResponse {
    ApiDocsTemplate {
        active: "preferences".to_string(),
        // Rendered so integrators can copy a working URL rather than a
        // relative path. `Host` reflects however the client reached us.
        base_url: format!("http://{host}{}/api", state.url_prefix),
        sections: crate::web::api_docs::sections(),
        tr: Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        repo_url: None,
        features,
    }
}
```

`Host`, `State` and `Extension` are already imported at the top of this file — no import changes are needed. `ApiDocsTemplate` arrives via the existing `use crate::web::templates::*;`.

- [ ] **Step 4: Build and check the page renders**

```bash
cargo build
./target/debug/cook server ./seed --port 9099
```

In another terminal:

```bash
curl -s http://127.0.0.1:9099/api-docs | grep -c "endpoint\|<article"
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:9099/api-docs
```

Expected: HTTP `200`, and the grep count is non-zero.

Then open `http://127.0.0.1:9099/api-docs` in a browser and confirm:
- the base URL card shows `http://127.0.0.1:9099/api`
- the Contents links jump to each section
- method badges are colored per verb
- JSON examples scroll horizontally rather than pushing the page wide
- toggling the browser to dark mode keeps everything readable

Stop the server with Ctrl-C.

- [ ] **Step 5: Commit**

```bash
git add templates/api_docs.html src/server/ui.rs
git commit -m "feat(server): serve the API documentation page at /api-docs"
```

---

## Task 11: Link from preferences

**Files:**
- Modify: `templates/preferences.html:148-180`

- [ ] **Step 1: Add the link**

In the "Documentation & Resources" card, insert this block as the **first** entry inside `<div class="space-y-2 text-sm">`, immediately before the existing CLI Documentation link:

```html
                {% if !static_mode %}
                <div>
                    <a href="{{ prefix }}/api-docs"
                       class="text-orange-700 hover:text-orange-800 hover:underline font-medium">
                        🔌 Server API
                    </a>
                    <span class="text-gray-600 ml-2">- HTTP endpoints for building integrations</span>
                </div>
                {% endif %}
```

The `{% if !static_mode %}` guard matters: `preferences.html` is also rendered by the static site export, where no server exists to answer these calls.

Unlike its siblings this link has no `target="_blank"` or `rel` attributes — it is an internal page, not an external site.

- [ ] **Step 2: Verify the link works**

```bash
cargo build && ./target/debug/cook server ./seed --port 9099
```

Open `http://127.0.0.1:9099/preferences`, confirm "🔌 Server API" appears at the top of Documentation & Resources, and click it — it should land on the API docs page in the same tab. Stop the server.

- [ ] **Step 3: Verify it is absent from a static build**

```bash
./target/debug/cook build web /tmp/cook-static-check --base-path ./seed
grep -rc "api-docs" /tmp/cook-static-check/ | grep -v ":0$"
```

Expected: the `grep -v` filters out every file with zero matches, so the command should print nothing and exit non-zero. Any line printed means the link leaked into the static site — check that the `{% if !static_mode %}` guard wraps the new block.

Clean up: `rm -rf /tmp/cook-static-check`

- [ ] **Step 4: Commit**

```bash
git add templates/preferences.html
git commit -m "feat(server): link the API docs from the preferences page"
```

---

## Task 12: Playwright coverage

**Files:**
- Create: `tests/e2e/api-docs.spec.ts`

- [ ] **Step 1: Write the spec**

```typescript
import { test, expect } from '@playwright/test';

const SECTIONS = [
  'recipes',
  'menus',
  'shopping-list',
  'pantry',
  'search-stats',
  'realtime',
  'sync',
];

test.describe('API documentation page', () => {
  test('renders the page with its ground rules', async ({ page }) => {
    await page.goto('/api-docs');
    await expect(page.getByRole('heading', { name: 'Server API', level: 1 })).toBeVisible();
    await expect(page.getByText('Base URL:')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Errors' })).toBeVisible();
  });

  test('every section anchor resolves to a section', async ({ page }) => {
    await page.goto('/api-docs');
    for (const id of SECTIONS) {
      await expect(page.locator(`#${id}`)).toHaveCount(1);
      await expect(page.locator(`a[href="#${id}"]`)).toHaveCount(1);
    }
  });

  test('documents endpoints with method badges and paths', async ({ page }) => {
    await page.goto('/api-docs');
    await expect(page.getByText('/api/shopping_list/items')).toBeVisible();
    await expect(page.locator('#pantry').getByText('/api/pantry/:section/:name').first()).toBeVisible();

    // Every verb the API uses should appear as a badge somewhere on the page.
    for (const method of ['GET', 'POST', 'PUT', 'DELETE']) {
      await expect(page.getByText(method, { exact: true }).first()).toBeVisible();
    }
  });

  test('is reachable from the preferences page', async ({ page }) => {
    await page.goto('/preferences');
    const link = page.getByRole('link', { name: /Server API/ });
    await expect(link).toBeVisible();
    await link.click();
    await expect(page).toHaveURL(/\/api-docs$/);
    await expect(page.getByRole('heading', { name: 'Server API', level: 1 })).toBeVisible();
  });
});
```

- [ ] **Step 2: Run the spec**

Run: `npx playwright test tests/e2e/api-docs.spec.ts --project=chromium`
Expected: 4 tests pass.

If Playwright browsers are not installed, run `npx playwright install chromium` first. The config starts the dev server automatically on port 9080.

- [ ] **Step 3: Commit**

```bash
git add tests/e2e/api-docs.spec.ts
git commit -m "test(server): add e2e coverage for the API documentation page"
```

---

## Task 13: Final verification

**Files:** none modified unless a check fails.

- [ ] **Step 1: Format**

Run: `cargo fmt`
Then: `git diff --stat`

If `cargo fmt` changed anything, commit it:

```bash
git add -A && git commit -m "style: cargo fmt"
```

- [ ] **Step 2: Lint**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: no warnings. A likely one is `clippy::too_many_lines` on `shopping_list()` in `api_docs.rs` — if it fires, add `#[allow(clippy::too_many_lines)]` to the offending function with a one-line comment saying the length is content, not logic.

- [ ] **Step 3: Full test suite**

Run: `cargo test`
Expected: all tests pass, including the 7 in `api_docs`.

- [ ] **Step 4: Confirm the drift guard covers the real router**

Run: `cargo test --lib api_docs -- --nocapture`
Expected: PASS. This is the check that the whole approach exists for — do not skip it or mark it "probably fine".

- [ ] **Step 5: Final visual check**

```bash
./target/debug/cook server ./seed --port 9099
```

Walk the page one last time in both light and dark mode, on a narrow viewport as well as wide. Confirm no horizontal scrolling of the page body — only inside the `<pre>` and table containers. Stop the server.

- [ ] **Step 6: Commit anything outstanding**

```bash
git status --short
```

Expected: clean. If not, commit the remaining changes with an appropriate Conventional Commit message.

---

## Verification summary

When this plan is complete, all of the following are true:

- `GET /api-docs` returns a page documenting 33 endpoints across 7 sections
- The preferences page links to it, and static exports do not
- `cargo test` fails if someone adds a route to `src/server/mod.rs` without documenting it
- `cargo test` fails if someone documents a path that no longer exists
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` all pass
- `npx playwright test tests/e2e/api-docs.spec.ts` passes

**Known limit, by design:** the drift test compares paths only. Parameters and example payloads can still go stale while the path stays put. Guarding that would require the OpenAPI approach the design spec rejected. Anyone changing a handler's request or response shape must update `src/web/api_docs.rs` by hand.
