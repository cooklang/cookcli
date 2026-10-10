//! What `/api/static` serves from the recipe directory: the recipes' pictures,
//! and nothing else.
//!
//! The route is a `ServeDir` over the whole directory, which is often a git
//! checkout, a synced folder or part of a home directory. Served as it was, it
//! handed anyone `.git/config`, `.env`, `config/pantry.conf` and the shopping
//! list, sign-in or not (#658).

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// The extensions of the files `/api/static` serves: the title and step
/// pictures the recipe pages link to.
const PICTURES: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "avif"];

/// Whether `/api/static` serves `path`, the request path below the route.
///
/// It is percent-decoded first, as `ServeDir` does, so `%2Egit` is `.git` and
/// `%2F` separates folders. No folder or file name may start with `.`, and the
/// file must be a picture.
pub fn serves(path: &str) -> bool {
    let Ok(path) = urlencoding::decode(path) else {
        return false;
    };
    // Empty after a trailing `/`: a folder.
    let file = path.rsplit('/').next().unwrap_or_default();
    path.split('/').all(|name| !name.starts_with('.'))
        && file.rsplit_once('.').is_some_and(|(_, extension)| {
            PICTURES
                .iter()
                .any(|picture| extension.eq_ignore_ascii_case(picture))
        })
}

/// Answers `404` for whatever [`serves`] refuses, before `ServeDir` looks.
pub async fn only_pictures(request: Request, next: Next) -> Response {
    if serves(request.uri().path()) {
        next.run(request).await
    } else {
        tracing::debug!(path = request.uri().path(), "refused a static file");
        StatusCode::NOT_FOUND.into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::serves;

    #[test]
    fn serves_pictures() {
        for path in [
            "/Soup.jpg",
            "/Mains/Soup.1.WEBP",
            "/Mains/Soup.2.1.png",
            "/Easy%20Pancakes.jpeg",
            "/50%25%20%231%3F/Tart%27s.png",
            "/a.gif",
            "/a.avif",
        ] {
            assert!(serves(path), "{path}");
        }
    }

    #[test]
    fn refuses_the_rest() {
        for path in [
            "/",
            "",
            "/.shopping-list",
            "/.shopping-checked",
            "/config/pantry.conf",
            "/config/aisle.conf",
            "/Week.menu",
            "/Soup.cook",
            "/notes.html",
            "/logo.svg",
            "/.git/config",
            "/.config/gh/hosts.yml",
            "/.env",
            // A picture, but hidden or in a hidden folder.
            "/.jpg",
            "/Mains/.png",
            "/.git/screenshot.png",
            "/Mains/.cache/Soup.jpg",
            // The same, percent-encoded.
            "/%2Egit/config",
            "/%2egit%2Fscreenshot.png",
            "/Mains%2F.cache%2FSoup.jpg",
            "/%2Ejpg",
            // A folder, which ServeDir would answer with its index.html.
            "/Mains/",
            "/Mains.jpg/",
            // Not UTF-8 once decoded.
            "/%FF.jpg",
        ] {
            assert!(!serves(path), "{path}");
        }
    }
}
