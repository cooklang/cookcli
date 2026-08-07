# Server API Documentation Page — Design

Date: 2026-08-07

## Problem

CookCLI's server exposes roughly 33 HTTP endpoints under `/api`, but none of them
are documented anywhere. `docs/server.md` covers only the CLI flags for launching
the server. Anyone building an integration — a Home Assistant component, a script,
a mobile client — has to read `src/server/mod.rs` and the handler modules to learn
what exists and what it returns.

## Goal

A reference page served by the web UI at `/api-docs` that documents every API
endpoint with its method, path, parameters, and example request/response payloads,
linked from the preferences page.

Audience: someone writing a client against a running CookCLI server. The page is
read-only reference material, not an interactive console.

## Decisions

**Content authored as Rust data, rendered by a template.** The alternative of
hand-writing the endpoint list directly into an Askama template is simpler but
goes stale silently. Holding the content as data makes an automated drift check
possible, and for a reference document being *true* is the property that matters.

Generating from an OpenAPI spec (utoipa + a spec viewer) was rejected: it would
touch all ~30 handlers, add a heavy dependency, and its viewer assets do not bundle
cleanly into the existing `rust-embed` `StaticFiles` setup — a large change for a
page with a small readership.

**English-only, server-only.** Endpoint names, parameters, and JSON payloads are
English regardless, so routing prose through `tr.t()` and into all seven locale
files buys little and creates a large translation surface. The page is not part of
the static site export, where no API server exists to answer the documented calls.

**No new locale keys at all.** The preferences link label is hardcoded English to
match its four sibling links in the same card, which are already hardcoded.

## Architecture

### Data model

Structs live in `src/web/templates.rs`, following the existing `PantryItem` /
`PantrySection` precedent. Content lives in a new `src/web/api_docs.rs` exposing a
single `pub fn api_docs() -> Vec<ApiSection>`.

```rust
pub struct ApiSection {
    pub id: String,            // anchor target, e.g. "shopping-list"
    pub title: String,
    pub description: String,
    pub endpoints: Vec<EndpointDoc>,
}

pub struct EndpointDoc {
    pub method: String,        // "GET", "POST", "PUT", "DELETE"
    pub path: String,          // "/api/shopping_list/items"
    pub summary: String,
    pub description: String,
    pub params: Vec<ParamDoc>,
    pub request_example: Option<String>,   // pretty-printed JSON
    pub response_example: Option<String>,  // pretty-printed JSON
    pub feature: Option<String>,           // e.g. "sync" → build-requirement badge
}

pub struct ParamDoc {
    pub name: String,
    pub kind: String,          // "path" | "query" | "body"
    pub required: bool,
    pub type_name: String,     // "string", "number", "string[]"
    pub description: String,
}
```

### Sections

Seven sections, ~33 entries: Recipes, Menus, Shopping List, Pantry, Search & Stats,
Realtime (SSE + LSP websocket), Sync.

Sync endpoints are documented unconditionally and marked with a "requires sync
build" badge rather than being `#[cfg]`-gated out of the page. A reference that
silently omits endpoints depending on build flags is worse than one that labels
them.

`/api/static/*` (the `nest_service` serving recipe assets from the base path,
`src/server/mod.rs:176`) is documented alongside the Recipes section.

### Route and handler

`/api-docs` is added to `ui()` in `src/server/ui.rs`. That router is used only by
the server; the static export path goes through `web::builders` and never reaches
it, so exclusion from static exports requires no flag.

`api_docs_page` builds `ApiDocsTemplate { active, sections, base_url, tr, prefix,
static_mode: false, repo_url: None, features }`.

`base_url` is derived from axum's `Host` extractor — the same pattern
`create_recipe` uses in `src/server/ui.rs` — so the page shows a real
`http://localhost:9080/api` rather than a relative path or a JS-patched
placeholder.

### Page structure

Extends `base.html` with `active: "preferences"`, keeping the nav highlighted where
the reader came from.

- Intro card: base URL plus the four facts an integrator otherwise learns the hard
  way — **no authentication**, CORS is `allow_origin: *` (`src/server/mod.rs:201`),
  1 MB request body limit (`src/server/mod.rs:170`), JSON in and out.
- Table of contents linking to `#section-id` anchors.
- One card per endpoint: colored method badge, monospace path, summary, a params
  table when the endpoint has params, and `<pre>` blocks for request and response
  examples.

Existing Tailwind component classes are reused. `base.html` carries a dark-mode
block, so new card and `<pre>` styles need dark variants like their neighbours.

### Preferences link

Added to the existing "Documentation & Resources" card in
`templates/preferences.html`, wrapped in `{% if !static_mode %}` because that page
*is* rendered in static exports:

> 🔌 **Server API** — HTTP endpoints for building integrations

Internal link to `{{ prefix }}/api-docs`, without `target="_blank"` unlike its
external siblings.

## Error handling

The page itself has no failure path — its content is compile-time constant — so no
`error_page()` fallback is needed, unlike `recipe_page`.

The API's error convention is documented once at the top of the page rather than
repeated on all 33 entries. Handlers are consistent: `json_error` in
`src/server/handlers/common.rs` returns `{"error": "message"}` for every failure.
The subsection lists the status codes in use (400 for invalid paths, 404, 500).

## Content sourcing

Examples are read out of the handlers that produce them — roughly 2,200 lines
across `src/server/handlers/`. Parameters come from the `Deserialize` structs;
response examples come from the `json!` bodies the handlers actually return.
Anything not confidently determinable by reading is verified by running
`cook server ./seed` and calling the endpoint. No unverified payload ships.

## Testing

- **`router_and_docs_agree()`** in `src/web/api_docs.rs`. Uses
  `include_str!("../server/mod.rs")`, slices the source from `fn api(` to end of
  file, and collects every `.route("…")` path literal. Asserts both directions:
  every router path has at least one doc entry (catches a newly added undocumented
  endpoint) and every documented path still exists in the router (catches stale
  docs).

  Slicing from `fn api(` excludes the `/static/*file` route defined above it.
  `include_str!` sees the `#[cfg(feature = "sync")]` routes regardless of which
  features are active, which is what the badge decision requires.

  Known limit: the test cannot catch parameters or example payloads drifting while
  the path stays put. Guarding that would require the rejected OpenAPI approach.

- **`tests/e2e/api-docs.spec.ts`** (Playwright): the page renders, all seven
  section anchors resolve, and the preferences link navigates to it.

- **Manual**: `cargo fmt`, `cargo clippy`, `cargo test` per CLAUDE.md's pre-PR
  checklist, plus a visual check in both light and dark mode.

## Out of scope

- Try-it / send-request buttons
- Copy-to-clipboard on examples
- Search or filtering within the page
- OpenAPI spec export
- Documenting the `/api/ws/lsp` websocket message format beyond "bridges to the
  Cooklang LSP over websocket" — the protocol is the LSP specification's, not ours
