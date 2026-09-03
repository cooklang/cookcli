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

### Default: wildcard origin, reads only

Chosen over keeping `*` for all methods (preserves the security hole the issue
opens with) and over dropping the CORS layer entirely by default (a hard break
for custom frontends, with a browser error that gives no hint about the flag).

This is the shape suggested in the issue's comment: keep `*` so read-only
integrations keep working, but stop cross-origin mutation unless the operator
opts in.

**This is a behaviour change.** Cross-origin `POST`/`PUT`/`DELETE` from a
browser stops working until the user passes `--cors-origin`. Same-origin
traffic is unaffected: the web UI's own `Origin` matches its `Host`, and
`curl` and other non-browser clients send no `Origin` at all.

### Flags

| Flag | Behaviour |
|---|---|
| `--cors-origin <ORIGIN>` | Repeatable. `*` selects the wildcard. Omitted → `*`. |
| `--cors-allow-credentials` | Requires explicit origins; errors when origins are `*`. |
| `--no-csrf-check` | Renamed from `--no-cors`, hidden clap alias keeps the old spelling working. Now governs both the same-origin check on `POST /new` and the cross-origin write guard. |

Resolved policy:

| Origins | `allow_origin` | `allow_methods` | `allow_headers` | `allow_credentials` | write guard |
|---|---|---|---|---|---|
| `*` (default) | any | `GET` | `content-type` | never (rejected at parse) | rejects non-`GET`/`HEAD` carrying a cross-origin `Origin` |
| explicit list | that list | `GET, POST, PUT, DELETE` | `content-type` | opt-in | additionally allows the listed origins |

### Correction: `allow_methods` cannot enforce read-only

The first version of this design assumed `allow_methods([GET])` would stop
cross-origin writes. It does not. Per the Fetch standard's CORS-preflight step,
the `Access-Control-Allow-Methods` list is consulted **only when the request's
method is not CORS-safelisted**, and `GET`, `HEAD` and `POST` are all
safelisted. So the method list blocks `PUT` and `DELETE` but never `POST`.
Verified in a real browser: with `allow_methods([GET])` and
`allow_headers([content-type])`, a cross-origin page successfully wrote to
`seed/config/pantry.conf` through `POST /api/pantry/add`.

Worse, that combination is *more* permissive than the hardcoded layer it
replaces. Today the only thing blocking a cross-origin JSON `POST` is that
`allow_headers` is unset, so the preflight fails on `content-type`. Supplying
`content-type` — which the issue rightly asks for, and which legitimate
integrations need — removes that accidental protection.

Relying on the accident is not an option either: it holds only because axum's
`Json` extractor rejects the content types that would make a `POST` a simple,
un-preflighted request. That is a property of the extractors, not a policy, and
it would break silently the first time a route accepted form-encoded input.

So the read-only guarantee is enforced **server-side**, by a small middleware,
and the CORS headers are left to describe the policy rather than enforce it.

### The cross-origin write guard

A middleware, applied only to non-preflight requests (the CORS layer is
outermost, so it answers `OPTIONS` preflights before this runs):

1. `GET`, `HEAD` and `OPTIONS` pass.
2. A request with no `Origin` header passes. `curl`, scripts and every
   non-browser client send none, and the API has no authentication — blocking
   them would break every existing integration to no benefit, since a client
   that sets no `Origin` is not a browser acting on some user's behalf.
3. An `Origin` matching the request's `Host` passes. Browsers send `Origin` on
   same-origin `POST`s too, so the web UI's own writes land here.
4. An `Origin` in the explicit `--cors-origin` list passes.
5. Anything else gets `403` with the API's standard `{"error": ...}` body.

Rule 4 is what gives reverse-proxy deployments a clean path: when the proxy
forwards a `Host` that does not match the browser's `Origin`, naming the public
origin with `--cors-origin` restores writes without disabling anything.

`--no-csrf-check` skips the guard entirely, alongside the same-origin check it
already governs on `POST /new`. That makes the flag's name accurate — it is now
the single switch for same-origin enforcement — and leaves an escape hatch for
deployments where neither rule 3 nor rule 4 fits.

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
    /// True when a request carrying this `Origin` may use a mutating method.
    pub fn allows_write_from(&self, origin: &str, host: &str) -> bool;
}

/// Rules 1-5 of the write guard, as axum middleware.
pub async fn write_guard(
    State(config): State<Arc<CorsConfig>>,
    request: Request,
    next: Next,
) -> Response;

/// Shared by the guard and `ui::validate_same_origin`.
pub(super) fn origin_matches_host(origin: &str, host: &str) -> bool;
```

The module owns one question — who may do what cross-origin — so both the
headers that describe the policy and the guard that enforces it live here.
`origin_matches_host` replaces the origin/host comparison duplicated inside
`ui::validate_same_origin`; that function keeps its own "no `Origin` and no
`Referer` → reject" rule, which is right for an HTML form and wrong for the
API, where a missing `Origin` means a non-browser client.

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
becomes `.layer(cors.layer())`. The guard is added as
`axum::middleware::from_fn_with_state(Arc::new(cors), cors::write_guard)`,
inside the CORS layer, and only when `csrf_check` is set — when it is not, the
layer is simply never added.

### The rename

Four sites now: the `ServerArgs` field (`cors: bool` → `csrf_check: bool`, with
`#[arg(long = "no-csrf-check", alias = "no-cors", action = SetFalse)]`),
`AppState.cors` → `AppState.csrf_check`, its read at `src/server/ui.rs:304`,
and the new decision in `run()` about whether to add the guard. `alias` rather
than `visible_alias`, so the old spelling keeps working without cluttering
`--help`.

## Testing

Unit tests in `src/server/cors.rs` covering `from_args`:

- no origins → `Any`
- explicit list → `List` in order
- `*` mixed with an explicit origin → error
- credentials + wildcard → error
- credentials + explicit origins → ok
- `http://app.test/` → error
- origins that could never match a browser's `Origin` (wrong case, whitespace,
  path, query, fragment, userinfo, bad port) → error
- bracketed IPv6 and non-`http` schemes (`chrome-extension://…`) → accepted

Unit tests for `allows_write_from` and `origin_matches_host`: same-origin with
and without a port, a listed origin, an unlisted origin, and a host that
differs only by port.

Integration tests in `tests/cors_test.rs`, following the server-booting pattern
of `tests/menu_api_test.rs` (ephemeral port, `ServerGuard`, `reqwest`):

- Default policy: preflight with `Access-Control-Request-Method: GET` →
  `access-control-allow-origin: *`; the same preflight with `PUT` →
  `access-control-allow-methods: GET`, so the browser blocks it.
- **Default policy, real cross-origin `POST`** → `403`. This is the test that
  matters: a preflight assertion cannot show it, because `AllowOrigin::any()`
  always answers `*` and `POST` is CORS-safelisted.
- Default policy, same-origin `POST` (`Origin` equal to the server's own
  authority) → not `403`. Proves the web UI still works.
- Default policy, `POST` with no `Origin` header → not `403`. Proves `curl` and
  existing scripts still work.
- `--cors-origin http://app.test`: `POST` preflight from `http://app.test` →
  allowed, `access-control-allow-headers: content-type`; a real `POST` from
  `http://app.test` → not `403`; from `http://evil.test` → `403`.
- `--no-csrf-check`: cross-origin `POST` from `http://evil.test` → not `403`.
- `--cors-origin http://app.test --cors-allow-credentials` →
  `access-control-allow-credentials: true`.
- `--cors-allow-credentials` alone → non-zero exit, message names the conflict.

## Documentation

- `docs/server.md`: three new rows in the Options table, an example, and a
  Notes bullet stating the default. `--no-csrf-check` gets documented for the
  first time.
- `src/web/api_docs.rs:56`: the CORS note is rewritten to describe the
  reads-by-default policy, the `403` on a cross-origin write, and
  `--cors-origin`.
- `docs/api.md` is **generated** from that file and guarded by
  `tests/api_docs_md_test.rs`. Regenerate with
  `UPDATE_API_DOCS=1 cargo test --test api_docs_md_test`; never hand-edit it.
  Per the test's own message, cooklang.org's `scripts/sync-cli-docs.sh` is the
  follow-up to update the website page.
