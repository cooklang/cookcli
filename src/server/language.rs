use accept_language::parse;
use axum::{
    extract::Request,
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

const SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[EN_US, DE_DE, NL_NL, FR_FR, ES_ES];

/// Get the preferred language from headers
/// 1. Check for 'lang' cookie
/// 2. Parse Accept-Language header
/// 3. Fall back to EN_US
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
                let base_lang = user_lang.split('-').next().unwrap_or(&user_lang);

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
pub async fn language_middleware(mut req: Request, next: Next) -> Response {
    let lang = get_preferred_language(req.headers());
    req.extensions_mut().insert(lang);
    next.run(req).await
}
