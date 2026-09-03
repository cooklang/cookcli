//! CORS policy for `cook server`, built from the `--cors-origin` and
//! `--cors-allow-credentials` flags.
//!
//! CORS is enforced by browsers only, so this governs what cross-origin web
//! pages may do with the API. The web UI itself, `curl`, and every non-browser
//! client are unaffected by anything here.

use anyhow::{bail, Result};
use axum::http::{header, HeaderValue, Method};
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
    pub fn methods(&self) -> Vec<Method> {
        match &self.origins {
            CorsOrigins::Any => vec![Method::GET],
            CorsOrigins::List(_) => {
                vec![Method::GET, Method::POST, Method::PUT, Method::DELETE]
            }
        }
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

    let (host, port) = match rest.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (rest, None),
    };
    if host.is_empty() {
        bail!("invalid --cors-origin {origin:?}: expected scheme://host[:port]");
    }
    if let Some(bad) = host
        .chars()
        .find(|c| c.is_whitespace() || c.is_ascii_uppercase() || matches!(c, '/' | '?' | '#' | '@'))
    {
        bail!(
            "invalid --cors-origin {origin:?}: a browser sends a bare lowercase origin, \
             so {bad:?} can never match; expected scheme://host[:port], \
             e.g. http://localhost:3000"
        );
    }
    if let Some(port) = port {
        if port.parse::<u16>().is_err() {
            bail!("invalid --cors-origin {origin:?}: {port:?} is not a valid port number");
        }
    }

    HeaderValue::from_str(origin)
        .map_err(|e| anyhow::anyhow!("invalid --cors-origin {origin:?}: {e}"))
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
        ] {
            CorsConfig::from_args(&origins(&[bad]), false)
                .expect_err(&format!("{bad:?} must be rejected"));
        }
    }

    #[test]
    fn non_http_schemes_are_accepted() {
        // Browser extensions and desktop shells send these as `Origin`.
        for good in [
            "chrome-extension://abcdefghijklmnop",
            "moz-extension://abcdefghijklmnop",
            "tauri://localhost",
            "https://a.test:8443",
        ] {
            CorsConfig::from_args(&origins(&[good]), false)
                .unwrap_or_else(|e| panic!("{good:?} must be accepted: {e}"));
        }
    }
}
