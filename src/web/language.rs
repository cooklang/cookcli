#[cfg(feature = "server")]
use accept_language::parse;
#[cfg(feature = "server")]
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap},
    middleware::Next,
    response::Response,
};
use unic_langid::{langid, LanguageIdentifier};

/// Supported languages
pub const EN_US: LanguageIdentifier = langid!("en-US");
pub const DE_DE: LanguageIdentifier = langid!("de-DE");
pub const NL_NL: LanguageIdentifier = langid!("nl-NL");
pub const FR_FR: LanguageIdentifier = langid!("fr-FR");
pub const ES_ES: LanguageIdentifier = langid!("es-ES");
pub const EU_ES: LanguageIdentifier = langid!("eu-ES");
pub const SV_SE: LanguageIdentifier = langid!("sv-SE");

pub const SUPPORTED_LANGUAGES: &[LanguageIdentifier] =
    &[EN_US, DE_DE, NL_NL, FR_FR, ES_ES, EU_ES, SV_SE];

/// Per-request feature visibility flags, read from cookies.
#[derive(Clone, Copy, Debug)]
pub struct FeatureFlags {
    pub show_shopping_list: bool,
    pub show_pantry: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            show_shopping_list: true,
            show_pantry: true,
        }
    }
}

/// Parse feature flag cookies from request headers.
/// Absent cookie → feature enabled (default true).
/// Cookie value "0" → disabled. Any other value → enabled.
#[cfg(feature = "server")]
pub fn parse_feature_flags(headers: &HeaderMap) -> FeatureFlags {
    let mut flags = FeatureFlags::default();

    if let Some(cookie_header) = headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 {
                    match parts[0] {
                        "show_shopping_list" => flags.show_shopping_list = parts[1] != "0",
                        "show_pantry" => flags.show_pantry = parts[1] != "0",
                        _ => {}
                    }
                }
            }
        }
    }

    flags
}

/// Parse a user-supplied language tag (e.g. "de", "de-DE") into one of the
/// supported [`LanguageIdentifier`]s. Returns `None` for unsupported tags.
pub fn parse_supported_language(s: &str) -> Option<LanguageIdentifier> {
    let parsed: LanguageIdentifier = s.parse().ok()?;
    if SUPPORTED_LANGUAGES.contains(&parsed) {
        return Some(parsed);
    }
    // Allow bare language codes ("de") to match a supported region ("de-DE").
    let base = s.split('-').next().unwrap_or(s);
    SUPPORTED_LANGUAGES
        .iter()
        .find(|l| l.language.as_str().eq_ignore_ascii_case(base))
        .cloned()
}

/// Get the preferred language from headers
/// 1. Check for 'lang' cookie
/// 2. Parse Accept-Language header
/// 3. Fall back to EN_US
#[cfg(feature = "server")]
pub fn get_preferred_language(headers: &HeaderMap) -> LanguageIdentifier {
    // First, check for language cookie
    if let Some(cookie_header) = headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == "lang" {
                    if let Ok(lang) = parts[1].parse::<LanguageIdentifier>() {
                        // Check if it's a supported language
                        if SUPPORTED_LANGUAGES.contains(&lang) {
                            return lang;
                        }
                    }
                }
            }
        }
    }

    // Next, check Accept-Language header
    if let Some(accept_lang) = headers.get(header::ACCEPT_LANGUAGE) {
        if let Ok(accept_lang_str) = accept_lang.to_str() {
            let user_langs = parse(accept_lang_str);

            // Try to match each user language preference
            for user_lang in &user_langs {
                // Try exact match first
                if let Ok(lang) = user_lang.parse::<LanguageIdentifier>() {
                    if SUPPORTED_LANGUAGES.contains(&lang) {
                        return lang;
                    }
                }

                // Try to match just the language part (e.g., "de" matches "de-DE")
                let base_lang = user_lang.split('-').next().unwrap_or(user_lang);

                for supported_lang in SUPPORTED_LANGUAGES {
                    let supported_str = supported_lang.to_string();
                    let supported_base = supported_str.split('-').next().unwrap_or(&supported_str);

                    if base_lang.eq_ignore_ascii_case(supported_base) {
                        return supported_lang.clone();
                    }
                }
            }
        }
    }

    // Default to English
    EN_US
}

/// Middleware to inject language into request extensions
#[cfg(feature = "server")]
pub async fn language_middleware(mut req: Request, next: Next) -> Response {
    let lang = get_preferred_language(req.headers());
    req.extensions_mut().insert(lang);
    next.run(req).await
}

/// Middleware that reads feature flag cookies, injects them as a request
/// extension, and refreshes the cookie expiry on every response.
/// Takes the URL prefix as state so Set-Cookie headers use the correct path.
#[cfg(feature = "server")]
pub async fn features_middleware(
    State(url_prefix): State<String>,
    mut req: Request,
    next: Next,
) -> Response {
    let features = parse_feature_flags(req.headers());
    req.extensions_mut().insert(features);
    let mut response = next.run(req).await;

    let max_age = 365 * 24 * 60 * 60_u32;
    let cookie_path = if url_prefix.is_empty() {
        "/".to_string()
    } else {
        format!("{url_prefix}/")
    };

    for (name, val) in [
        (
            "show_shopping_list",
            if features.show_shopping_list {
                "1"
            } else {
                "0"
            },
        ),
        ("show_pantry", if features.show_pantry { "1" } else { "0" }),
    ] {
        let cookie = format!("{name}={val}; path={cookie_path}; max-age={max_age}; SameSite=Lax");
        if let Ok(header_val) = cookie.parse() {
            response
                .headers_mut()
                .append(header::SET_COOKIE, header_val);
        }
    }

    response
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    fn make_cookie_headers(cookies: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, cookies.parse().unwrap());
        h
    }

    #[test]
    fn test_feature_flags_default_true_when_no_cookies() {
        let flags = parse_feature_flags(&HeaderMap::new());
        assert!(flags.show_shopping_list);
        assert!(flags.show_pantry);
    }

    #[test]
    fn test_feature_flags_disabled_by_zero() {
        let flags =
            parse_feature_flags(&make_cookie_headers("show_shopping_list=0; show_pantry=0"));
        assert!(!flags.show_shopping_list);
        assert!(!flags.show_pantry);
    }

    #[test]
    fn test_feature_flags_enabled_by_one() {
        let flags =
            parse_feature_flags(&make_cookie_headers("show_shopping_list=1; show_pantry=1"));
        assert!(flags.show_shopping_list);
        assert!(flags.show_pantry);
    }

    #[test]
    fn test_feature_flags_partial_override() {
        let flags = parse_feature_flags(&make_cookie_headers("show_shopping_list=0"));
        assert!(!flags.show_shopping_list);
        assert!(flags.show_pantry); // absent → default true
    }

    #[test]
    fn test_feature_flags_unknown_value_treated_as_enabled() {
        // Anything that isn't "0" is truthy
        let flags = parse_feature_flags(&make_cookie_headers("show_shopping_list=yes"));
        assert!(flags.show_shopping_list);
    }
}
