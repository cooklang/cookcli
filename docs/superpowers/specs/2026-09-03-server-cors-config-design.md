# `cook server`: configurable CORS

**Date:** 2026-09-03
**Status:** Approved, ready for planning
**Issue:** [#465](https://github.com/cooklang/cookcli/issues/465) — "server: make CORS configuration configurable"

## Problem

`cook server` applies one hardcoded CORS policy (`src/server/mod.rs:206-210`):

```rust
CorsLayer::new()
    .allow_origin("*".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]);
```

It is wrong in both directions at once.

**Too permissive.** Any page open in the user's browser can call the API of a
running server, including the mutating `POST`/`PUT`/`DELETE` routes — shopping
list, recipe editing, pantry. That matters more since `--host` started exposing
the server on the LAN. There is no way to lock it down.

**Too restrictive where it counts.** `allow_headers` is never set, so
tower-http defaults to allowing none. A cross-origin `POST` carrying
`Content-Type: application/json` fails preflight, so the wide-open method list
buys legitimate integrations nothing. `allow_credentials` cannot be used at
all, because it is invalid alongside a wildcard origin. Anyone building a
custom frontend has to patch the binary.

### The `--no-cors` name is already taken

`ServerArgs` has a `--no-cors` flag today. It does not touch `CorsLayer`. It
disables a same-origin CSRF check on the HTML form `POST /new`
(`src/server/ui.rs:304`, reading `AppState.cors`). It is undocumented in
`docs/server.md`. Adding `--cors-origin` beside it would make the collision
actively misleading.

## Decisions

### Default: wildcard origin, `GET` only

Chosen over keeping `*` for all methods (preserves the security hole the issue
opens with) and over dropping the CORS layer entirely by default (a hard break
for custom frontends, with a browser error that gives no hint about the flag).

This is the shape suggested in the issue's comment: keep `*` so read-only
integrations keep working, but stop cross-origin mutation unless the operator
opts in.

**This is a behaviour change.** Cross-origin `POST`/`PUT`/`DELETE` stops
working until the user passes `--cors-origin`. Same-origin traffic is
unaffected — the web UI itself, `curl`, and every non-browser client never
consult CORS, which is browser-enforced only.

### Flags

| Flag | Behaviour |
|---|---|
| `--cors-origin <ORIGIN>` | Repeatable. `*` selects the wildcard. Omitted → `*`. |
| `--cors-allow-credentials` | Requires explicit origins; errors when origins are `*`. |
| `--no-csrf-check` | Renamed from `--no-cors`, hidden clap alias keeps the old spelling working. Behaviour unchanged. |

Resolved policy:

| Origins | `allow_origin` | `allow_methods` | `allow_headers` | `allow_credentials` |
|---|---|---|---|---|
| `*` (default) | any | `GET` | `content-type` | never (rejected at parse) |
| explicit list | that list | `GET, POST, PUT, DELETE` | `content-type` | opt-in |

**Methods are derived from origins, not their own flag.** A wildcard origin is
exactly the case where mutation should be unreachable; naming an origin is
exactly the statement "I trust this app". A `--cors-allow-method` flag would
let a user reconstruct today's insecure default by accident and adds a knob to
document and test that nobody needs.

**`content-type` is allowed unconditionally**, so no `--cors-allow-header`
flag. Without it, cross-origin JSON `POST` fails preflight whatever the origin
setting is — there is no configuration here worth exposing. The three
CORS-safelisted request headers (`Accept`, `Accept-Language`,
`Content-Language`) are permitted by the browser regardless and need no entry.

### No config file

The issue asks whether this should be expressible in a config file. No. The
comment's point was that CORS settings should be set the same way as `--host`
and `--port`, and those are flags only. A config file is a separate, broader
change to the whole server surface.

## Design

### `src/server/cors.rs` (new)

The policy becomes a validated value rather than an inline builder chain, so
the rules are unit-testable without booting a server:

```rust
pub enum CorsOrigins {
    Any,
    List(Vec<HeaderValue>),
}

pub struct CorsConfig {
    origins: CorsOrigins,
    allow_credentials: bool,
}

impl CorsConfig {
    /// Validates the flag combination.
    pub fn from_args(origins: &[String], allow_credentials: bool) -> Result<Self>;
    pub fn layer(&self) -> CorsLayer;
}
```

`from_args` rules:

- Empty `origins` → `CorsOrigins::Any`.
- An entry of `*` must be the only entry. Mixed with an explicit origin →
  error naming the conflict.
- Each explicit origin must parse as a `HeaderValue` and be a bare origin —
  scheme, host, optional port; no path, no trailing slash. `http://app.test/`
  is an error, because a browser sends `Origin: http://app.test` and a trailing
  slash would silently never match.
- `allow_credentials` with `CorsOrigins::Any` → error telling the user to name
  explicit origins.

`layer()` maps the value onto tower-http: `AllowOrigin::any()` or
`AllowOrigin::list(..)`, the method list from the table above,
`[header::CONTENT_TYPE]`, and `allow_credentials` when set.

### `src/server/mod.rs`

`run()` calls `CorsConfig::from_args(..)?` **before** `build_state` and before
binding the socket, so a bad combination fails immediately rather than after
the "Listening on…" banner. The hardcoded block at `src/server/mod.rs:206-210`
becomes `.layer(cors.layer())`.

### The rename

Three sites: the `ServerArgs` field (`cors: bool` → `csrf_check: bool`, with
`#[arg(long = "no-csrf-check", alias = "no-cors", action = SetFalse)]`),
`AppState.cors` → `AppState.csrf_check`, and its single read at
`src/server/ui.rs:304`. `alias` rather than `visible_alias`, so the old
spelling keeps working without cluttering `--help`.

## Testing

Unit tests in `src/server/cors.rs` covering `from_args`:

- no origins → `Any`
- explicit list → `List` in order
- `*` mixed with an explicit origin → error
- credentials + wildcard → error
- credentials + explicit origins → ok
- `http://app.test/` → error

Integration tests in `tests/cors_test.rs`, following the server-booting pattern
of `tests/menu_api_test.rs` (ephemeral port, `ServerGuard`, `reqwest`):

- Default policy: preflight with `Access-Control-Request-Method: GET` →
  `access-control-allow-origin: *`; the same preflight with `POST` → not
  allowed.
- `--cors-origin http://app.test`: `POST` preflight from `http://app.test` →
  allowed, `access-control-allow-headers: content-type`; from
  `http://evil.test` → not allowed.
- `--cors-origin http://app.test --cors-allow-credentials` →
  `access-control-allow-credentials: true`.
- `--cors-allow-credentials` alone → non-zero exit, message names the conflict.

## Documentation

- `docs/server.md`: three new rows in the Options table, an example, and a
  Notes bullet stating the default. `--no-csrf-check` gets documented for the
  first time.
- `src/web/api_docs.rs:56`: the CORS note is rewritten to describe the
  `GET`-by-default policy and point at `--cors-origin`.
- `docs/api.md` is **generated** from that file and guarded by
  `tests/api_docs_md_test.rs`. Regenerate with
  `UPDATE_API_DOCS=1 cargo test --test api_docs_md_test`; never hand-edit it.
  Per the test's own message, cooklang.org's `scripts/sync-cli-docs.sh` is the
  follow-up to update the website page.
