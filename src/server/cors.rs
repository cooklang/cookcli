//! CORS policy for `cook server`, built from the `--cors-origin` and
//! `--cors-allow-credentials` flags.
//!
//! CORS is enforced by browsers only, so this governs what cross-origin web
//! pages may do with the API. The web UI itself, `curl`, and every non-browser
//! client are unaffected by anything here.

use anyhow::{bail, Result};
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Which origins may make cross-origin requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorsOrigins {
    /// `--cors-origin '*'`, or no `--cors-origin` at all. Read-only: see
    /// [`CorsConfig::methods`].
    Any,
    /// One or more explicit origins, in the order given on the command line.
    List(Vec<HeaderValue>),
}

/// A validated CORS policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorsConfig {
    origins: CorsOrigins,
    allow_credentials: bool,
}

impl CorsConfig {
    /// Validates a `--cors-origin` / `--cors-allow-credentials` combination.
    ///
    /// `origins` is the raw repeated flag; empty means the flag was not given.
    pub fn from_args(origins: &[String], allow_credentials: bool) -> Result<Self> {
        let wildcard = origins.iter().any(|o| o == "*");
        if wildcard && origins.iter().any(|o| o != "*") {
            bail!(
                "--cors-origin '*' cannot be combined with explicit origins; \
                 pass either '*' or a list of origins, not both"
            );
        }

        let origins = if origins.is_empty() || wildcard {
            CorsOrigins::Any
        } else {
            let parsed = origins
                .iter()
                .map(|origin| parse_origin(origin))
                .collect::<Result<Vec<_>>>()?;
            CorsOrigins::List(parsed)
        };

        if allow_credentials && matches!(origins, CorsOrigins::Any) {
            bail!(
                "--cors-allow-credentials requires explicit --cors-origin values; \
                 browsers reject credentialed requests against a wildcard origin"
            );
        }

        Ok(Self {
            origins,
            allow_credentials,
        })
    }

    /// The methods this policy allows cross-origin.
    ///
    /// A wildcard origin means *any* page in the user's browser can reach the
    /// server, so it gets read-only access. Naming an origin is an explicit
    /// statement of trust, and unlocks the mutating routes.
    fn methods(&self) -> Vec<Method> {
        match &self.origins {
            CorsOrigins::Any => vec![Method::GET],
            CorsOrigins::List(_) => {
                vec![Method::GET, Method::POST, Method::PUT, Method::DELETE]
            }
        }
    }

    /// Builds the tower-http layer for this policy.
    ///
    /// `content-type` is always allowed: without it a cross-origin JSON `POST`
    /// fails preflight no matter what the origin setting is, so there is
    /// nothing here worth making configurable. The CORS-safelisted request
    /// headers (`Accept`, `Accept-Language`, `Content-Language`) need no entry
    /// — browsers permit them regardless.
    pub fn layer(&self) -> CorsLayer {
        let allow_origin = match &self.origins {
            CorsOrigins::Any => AllowOrigin::any(),
            CorsOrigins::List(list) => AllowOrigin::list(list.iter().cloned()),
        };

        CorsLayer::new()
            .allow_origin(allow_origin)
            .allow_methods(self.methods())
            .allow_headers([header::CONTENT_TYPE])
            .allow_credentials(self.allow_credentials)
    }

    /// Whether a request with this method, these headers and this `Host` may
    /// modify recipes.
    ///
    /// `allow_methods` cannot express this. `POST` is a CORS-safelisted method,
    /// so a browser never consults `Access-Control-Allow-Methods` for it —
    /// only a server-side check makes the wildcard default actually read-only.
    fn allows_write(&self, method: &Method, headers: &HeaderMap, host: &str) -> bool {
        if *method == Method::GET || *method == Method::HEAD || *method == Method::OPTIONS {
            return true;
        }
        // No `Origin` means no browser. `curl` and other clients send none, and
        // the API has no authentication for them to bypass.
        let Some(origin) = headers.get(header::ORIGIN) else {
            return true;
        };
        let Ok(origin) = origin.to_str() else {
            return false;
        };
        origin_matches_host(origin, host) || self.lists_origin(origin)
    }

    fn lists_origin(&self, origin: &str) -> bool {
        match &self.origins {
            CorsOrigins::Any => false,
            CorsOrigins::List(list) => list
                .iter()
                .any(|listed| listed.as_bytes() == origin.as_bytes()),
        }
    }
}

/// The `Host` header, if the request carries exactly one.
///
/// Deliberately ignores `Forwarded` / `X-Forwarded-Host`: those are set by
/// whoever is on the other end of a direct connection, so a security decision
/// must not depend on them. A deployment behind a proxy that rewrites `Host`
/// names its public origin with `--cors-origin` instead.
///
/// More than one `Host` header is a malformed request (RFC 9112 §3.2 requires
/// exactly one), and picking the first would let a raw client choose which one
/// the guard sees — so that is treated as no host at all, which fails closed.
pub(super) fn host_header(headers: &HeaderMap) -> Option<&str> {
    let mut hosts = headers.get_all(header::HOST).iter();
    let host = hosts.next()?;
    if hosts.next().is_some() {
        return None;
    }
    host.to_str().ok()
}

/// The authority this request was addressed to.
///
/// Deliberately reads the `Host` header (or, for HTTP/2, the URI authority)
/// rather than `Forwarded` / `X-Forwarded-Host`. Those are set by whoever is
/// on the other end of a direct connection, so a security decision must not
/// depend on them — `Origin: http://evil.test` plus
/// `X-Forwarded-Host: evil.test` would otherwise look same-origin. A
/// deployment behind a proxy that rewrites `Host` names its public origin with
/// `--cors-origin` instead.
fn request_host(request: &Request) -> Option<&str> {
    host_header(request.headers()).or_else(|| {
        request
            .uri()
            .authority()
            .map(|authority| authority.as_str())
    })
}

/// Whether an `Origin` value denotes the same host the request was sent to.
///
/// `Origin` is `scheme://host[:port]` while `Host` is `host[:port]`, and a
/// browser omits the port when it is the scheme's default — so the comparison
/// has to allow `https://a.test` to match either `a.test` or `a.test:443`.
///
/// The scheme is deliberately not compared. `cook server` speaks plaintext
/// HTTP, but is routinely fronted by a TLS-terminating proxy that passes
/// `Host` through unchanged, so `Origin: https://cook.example.com` against
/// `Host: cook.example.com` is the *normal* proxied case — rejecting it would
/// break those deployments. The converse reading, an `https://` page reaching
/// a plaintext server on port 80, is blocked by browsers' mixed-content
/// policy, so nothing reachable is given up.
pub(super) fn origin_matches_host(origin: &str, host: &str) -> bool {
    let Ok(url) = url::Url::parse(origin) else {
        return false;
    };
    let Some(origin_host) = url.host_str() else {
        return false;
    };
    match url.port() {
        Some(port) => host.eq_ignore_ascii_case(&format!("{origin_host}:{port}")),
        None => match url.port_or_known_default() {
            Some(default) => {
                host.eq_ignore_ascii_case(origin_host)
                    || host.eq_ignore_ascii_case(&format!("{origin_host}:{default}"))
            }
            None => host.eq_ignore_ascii_case(origin_host),
        },
    }
}

/// Parses one `--cors-origin` value.
///
/// A browser sends a bare origin — lowercase scheme, lowercase host, optional
/// numeric port — in the `Origin` header, and tower-http compares it byte for
/// byte. Anything richer or differently cased would never match and would fail
/// silently at request time, with no diagnostic anywhere, so it is rejected
/// here instead.
fn parse_origin(origin: &str) -> Result<HeaderValue> {
    let Some((scheme, rest)) = origin.split_once("://") else {
        bail!("invalid --cors-origin {origin:?}: expected a scheme, e.g. http://localhost:3000");
    };
    if scheme.is_empty() || rest.is_empty() {
        bail!("invalid --cors-origin {origin:?}: expected scheme://host[:port]");
    }
    // Schemes are ASCII, and browsers lowercase them. `chrome-extension` and
    // other non-http schemes are legitimate `Origin` values, so the rule is a
    // character set rather than an allowlist.
    if !scheme
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '-' | '.'))
    {
        bail!(
            "invalid --cors-origin {origin:?}: {scheme:?} is not a valid lowercase scheme, \
             e.g. http://localhost:3000"
        );
    }

    // Check the whole remainder before splitting off a port, so userinfo and
    // other extras are reported as what they are rather than as a bad port.
    if let Some(bad) = rest
        .chars()
        .find(|c| c.is_whitespace() || c.is_ascii_uppercase() || matches!(c, '/' | '?' | '#' | '@'))
    {
        bail!(
            "invalid --cors-origin {origin:?}: a browser sends a bare lowercase origin, \
             so {bad:?} can never match; expected scheme://host[:port], \
             e.g. http://localhost:3000"
        );
    }

    // An IPv6 host is bracketed and full of colons, so the port can only be
    // split off after the closing bracket.
    let (host, port) = if let Some(rest) = rest.strip_prefix('[') {
        let Some((host, after)) = rest.split_once(']') else {
            bail!(
                "invalid --cors-origin {origin:?}: unterminated IPv6 host, e.g. http://[::1]:3000"
            );
        };
        let port = match after {
            "" => None,
            _ => Some(after.strip_prefix(':').ok_or_else(|| {
                anyhow::anyhow!("invalid --cors-origin {origin:?}: expected scheme://host[:port]")
            })?),
        };
        (host, port)
    } else {
        match rest.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (rest, None),
        }
    };

    if host.is_empty() {
        bail!("invalid --cors-origin {origin:?}: missing host, expected scheme://host[:port]");
    }
    if let Some(port) = port {
        if port.parse::<u16>().is_err() {
            bail!("invalid --cors-origin {origin:?}: {port:?} is not a valid port number");
        }
    }

    HeaderValue::from_str(origin)
        .map_err(|e| anyhow::anyhow!("invalid --cors-origin {origin:?}: {e}"))
}

/// Rejects cross-origin requests that would modify recipes.
///
/// Applied inside the CORS layer, so tower-http has already answered any
/// `OPTIONS` preflight before this runs.
pub async fn write_guard(
    State(config): State<Arc<CorsConfig>>,
    request: Request,
    next: Next,
) -> Response {
    let host = request_host(&request).unwrap_or_default();
    if config.allows_write(request.method(), request.headers(), host) {
        return next.run(request).await;
    }

    tracing::warn!(
        method = %request.method(),
        path = %request.uri().path(),
        "refused a cross-origin request that would modify recipes"
    );
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({
            "error": "Cross-origin requests may not modify recipes. Start the server with \
                      --cors-origin <ORIGIN> to allow this origin, or --no-csrf-check to \
                      disable this check."
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origins(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_flag_defaults_to_any() {
        let config = CorsConfig::from_args(&[], false).expect("valid");
        assert_eq!(config.origins, CorsOrigins::Any);
        assert!(!config.allow_credentials);
    }

    #[test]
    fn explicit_wildcard_is_any() {
        let config = CorsConfig::from_args(&origins(&["*"]), false).expect("valid");
        assert_eq!(config.origins, CorsOrigins::Any);
        assert_eq!(config.methods(), vec![Method::GET]);
    }

    #[test]
    fn any_allows_only_get() {
        let config = CorsConfig::from_args(&[], false).expect("valid");
        assert_eq!(config.methods(), vec![Method::GET]);
    }

    #[test]
    fn explicit_origins_keep_order_and_allow_mutation() {
        let config =
            CorsConfig::from_args(&origins(&["http://a.test", "https://b.test:8443"]), false)
                .expect("valid");
        assert_eq!(
            config.origins,
            CorsOrigins::List(vec![
                HeaderValue::from_static("http://a.test"),
                HeaderValue::from_static("https://b.test:8443"),
            ])
        );
        assert_eq!(
            config.methods(),
            vec![Method::GET, Method::POST, Method::PUT, Method::DELETE]
        );
    }

    #[test]
    fn wildcard_mixed_with_explicit_origin_is_rejected() {
        let err = CorsConfig::from_args(&origins(&["*", "http://a.test"]), false)
            .expect_err("must reject");
        assert!(
            err.to_string().contains("cannot be combined"),
            "unhelpful error: {err}"
        );
    }

    #[test]
    fn credentials_with_wildcard_is_rejected() {
        let err = CorsConfig::from_args(&[], true).expect_err("must reject");
        assert!(
            err.to_string().contains("--cors-origin"),
            "error must point at the fix: {err}"
        );

        // The spelling a user actually types, and the one that would panic
        // tower-http at request time if this guard ever stopped firing.
        let err = CorsConfig::from_args(&origins(&["*"]), true).expect_err("must reject");
        assert!(
            err.to_string().contains("--cors-origin"),
            "error must point at the fix: {err}"
        );
    }

    #[test]
    fn credentials_with_explicit_origins_is_allowed() {
        let config = CorsConfig::from_args(&origins(&["http://a.test"]), true).expect("valid");
        assert!(config.allow_credentials);
    }

    #[test]
    fn origin_with_trailing_slash_is_rejected() {
        let err =
            CorsConfig::from_args(&origins(&["http://a.test/"]), false).expect_err("must reject");
        assert!(
            err.to_string().contains("http://a.test/"),
            "error must name the bad origin: {err}"
        );
    }

    #[test]
    fn origin_without_scheme_is_rejected() {
        let err = CorsConfig::from_args(&origins(&["a.test"]), false).expect_err("must reject");
        assert!(
            err.to_string().contains("expected a scheme"),
            "unhelpful error: {err}"
        );
    }

    #[test]
    fn empty_origin_is_rejected() {
        let err = CorsConfig::from_args(&origins(&[""]), false).expect_err("must reject");
        assert!(
            err.to_string().contains("expected a scheme"),
            "unhelpful error: {err}"
        );
    }

    #[test]
    fn origins_that_could_never_match_a_browser_are_rejected() {
        // tower-http compares the `Origin` header byte for byte, and browsers
        // send a bare lowercase origin. Each of these would start the server
        // happily and then never match anything.
        for bad in [
            "HTTP://A.test",
            "http://a.test ",
            " http://a.test",
            "http://a.test\t",
            "http://a.test/foo",
            "http://a.test?x=1",
            "http://a.test#frag",
            "http://user:pass@a.test",
            "http://a.test:notaport",
            "http://a.test:99999",
            "*://a.test",
            "://a.test",
            "http://[::1",
            "http://[::1]x",
            "http://[]",
            "http://:3000",
        ] {
            CorsConfig::from_args(&origins(&[bad]), false)
                .expect_err(&format!("{bad:?} must be rejected"));
        }
    }

    #[test]
    fn unusual_but_valid_origins_are_accepted() {
        // Browser extensions, desktop shells, and IPv6 hosts are legitimate
        // `Origin` values.
        for good in [
            "chrome-extension://abcdefghijklmnop",
            "moz-extension://abcdefghijklmnop",
            "tauri://localhost",
            "https://a.test:8443",
            "http://[::1]:3000",
            "http://[::1]",
        ] {
            CorsConfig::from_args(&origins(&[good]), false)
                .unwrap_or_else(|e| panic!("{good:?} must be accepted: {e}"));
        }
    }

    #[test]
    fn userinfo_is_reported_as_userinfo_not_as_a_bad_port() {
        let err = CorsConfig::from_args(&origins(&["http://user:pass@a.test"]), false)
            .expect_err("must reject");
        assert!(
            err.to_string().contains("bare lowercase origin"),
            "userinfo must not be misreported as a bad port: {err}"
        );
    }

    /// tower-http's `ensure_usable_cors_rules` runs in `Layer::layer`, not in
    /// the `CorsLayer` builder, so the layer has to be applied to a real
    /// service for its assertions to fire.
    fn assert_layer_applies(config: &CorsConfig) {
        let _ = axum::Router::<()>::new()
            .route("/", axum::routing::get(|| async {}))
            .layer(config.layer());
    }

    #[test]
    fn layer_applies_for_wildcard() {
        let config = CorsConfig::from_args(&[], false).expect("valid");
        assert_layer_applies(&config);
    }

    #[test]
    fn layer_applies_for_explicit_origins_with_credentials() {
        let config = CorsConfig::from_args(&origins(&["http://a.test"]), true).expect("valid");
        assert_layer_applies(&config);
    }

    fn headers_with_origin(origin: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_str(origin).expect("valid"),
        );
        headers
    }

    #[test]
    fn origin_matches_its_own_host() {
        assert!(origin_matches_host("http://a.test:9080", "a.test:9080"));
        assert!(origin_matches_host("http://a.test", "a.test"));
        // `Host` may or may not spell out a scheme's default port.
        assert!(origin_matches_host("https://a.test", "a.test:443"));
        assert!(origin_matches_host("http://a.test", "a.test:80"));
        // `Url::parse` lowercases the origin's host; the Host header is not
        // normalized for us.
        assert!(origin_matches_host("http://A.Test:9080", "a.test:9080"));
        assert!(origin_matches_host("http://a.test:9080", "A.TEST:9080"));
    }

    #[test]
    fn origin_on_a_different_port_is_not_the_same_host() {
        assert!(!origin_matches_host("http://a.test:9080", "a.test"));
        assert!(!origin_matches_host("http://a.test:9080", "a.test:3000"));
        assert!(!origin_matches_host("http://a.test", "b.test"));
        assert!(!origin_matches_host("not a url", "a.test"));
    }

    #[test]
    fn the_scheme_is_not_compared() {
        // Deliberate: see the note on `origin_matches_host`. A TLS-terminating
        // proxy passing Host through is the normal case for an https origin.
        assert!(origin_matches_host(
            "https://cook.example.test",
            "cook.example.test"
        ));
        // An explicit, non-default port still has to match exactly.
        assert!(!origin_matches_host(
            "https://cook.example.test",
            "cook.example.test:80"
        ));
    }

    fn request_with(headers: &[(&str, &str)]) -> Request {
        let mut builder = axum::http::Request::builder().uri("/api/pantry/add");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder
            .body(axum::body::Body::empty())
            .expect("valid request")
    }

    #[test]
    fn host_comes_from_the_host_header_not_a_forwarded_one() {
        // `Origin: http://evil.test` plus `X-Forwarded-Host: evil.test` must
        // not read as same-origin: both are attacker-supplied.
        let request = request_with(&[
            ("host", "127.0.0.1:9080"),
            ("x-forwarded-host", "evil.test"),
            ("forwarded", "host=evil.test"),
        ]);
        assert_eq!(request_host(&request), Some("127.0.0.1:9080"));
    }

    #[test]
    fn a_request_with_no_host_at_all_has_no_authority() {
        // Nothing can then match same-origin, so writes need --cors-origin.
        let request = request_with(&[]);
        assert_eq!(request_host(&request), None);
    }

    #[test]
    fn two_host_headers_are_treated_as_none() {
        // Picking the first would let a raw client choose which host the guard
        // compares against. Fail closed instead.
        let request = request_with(&[("host", "evil.test"), ("host", "127.0.0.1:9080")]);
        assert_eq!(request_host(&request), None);
    }

    #[test]
    fn http2_falls_back_to_the_uri_authority() {
        // h2 synthesises a Host header in practice, so this branch is belt and
        // braces — but it must still be right.
        let request = axum::http::Request::builder()
            .uri("http://127.0.0.1:9080/api/pantry/add")
            .body(axum::body::Body::empty())
            .expect("valid request");
        assert_eq!(request_host(&request), Some("127.0.0.1:9080"));
    }

    #[test]
    fn reads_are_always_allowed() {
        let config = CorsConfig::from_args(&[], false).expect("valid");
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert!(
                config.allows_write(&method, &headers_with_origin("http://evil.test"), "a.test"),
                "{method} must pass the guard"
            );
        }
    }

    #[test]
    fn a_request_without_an_origin_is_not_a_browser() {
        // curl and every other non-browser client. The API has no auth, so
        // rejecting these would break integrations and protect nothing.
        let config = CorsConfig::from_args(&[], false).expect("valid");
        assert!(config.allows_write(&Method::POST, &HeaderMap::new(), "a.test"));
    }

    #[test]
    fn same_origin_writes_are_allowed() {
        // This is the web UI's own POST.
        let config = CorsConfig::from_args(&[], false).expect("valid");
        assert!(config.allows_write(
            &Method::POST,
            &headers_with_origin("http://a.test:9080"),
            "a.test:9080"
        ));
    }

    #[test]
    fn cross_origin_writes_are_refused_under_the_wildcard_default() {
        // The hole this guard exists to close: POST is CORS-safelisted, so
        // Access-Control-Allow-Methods: GET does not stop it.
        let config = CorsConfig::from_args(&[], false).expect("valid");
        for method in [Method::POST, Method::PUT, Method::DELETE] {
            assert!(
                !config.allows_write(
                    &method,
                    &headers_with_origin("http://evil.test"),
                    "a.test:9080"
                ),
                "cross-origin {method} must be refused"
            );
        }
    }

    #[test]
    fn a_listed_origin_may_write() {
        let config =
            CorsConfig::from_args(&origins(&["http://app.test:3000"]), false).expect("valid");
        assert!(config.allows_write(
            &Method::POST,
            &headers_with_origin("http://app.test:3000"),
            "a.test:9080"
        ));
        assert!(!config.allows_write(
            &Method::POST,
            &headers_with_origin("http://evil.test"),
            "a.test:9080"
        ));
    }
}
