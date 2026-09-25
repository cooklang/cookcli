//! `/login` and `/logout`.

use super::session;
use crate::server::AppState;
use crate::web::language::FeatureFlags;
use crate::web::templates::{LoginTemplate, Tr};
use crate::web::viewer::Viewer;
use axum::{
    extract::{Extension, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

#[derive(Deserialize, Default)]
pub struct LoginQuery {
    next: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
    next: Option<String>,
}

/// Where to go after signing in, relative to the URL prefix.
///
/// Only a path on this server is accepted: `//evil.test` and `/\evil.test`
/// are read by browsers as another host, so they, and anything that is not
/// plain visible ASCII, fall back to the home page.
pub fn sanitize_next(next: Option<&str>) -> String {
    match next {
        Some(next)
            if next.starts_with('/')
                && !next.starts_with("//")
                && !next.starts_with("/\\")
                && next.bytes().all(|b| b.is_ascii_graphic()) =>
        {
            next.to_string()
        }
        _ => "/".to_string(),
    }
}

fn redirect_to(url_prefix: &str, next: &str) -> Response {
    Redirect::to(&format!("{url_prefix}{next}")).into_response()
}

pub async fn login_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
    Query(query): Query<LoginQuery>,
) -> Response {
    let next = sanitize_next(query.next.as_deref());
    if state.auth.is_none() || viewer.is_signed_in() {
        return redirect_to(&state.url_prefix, &next);
    }
    login_template(&state, lang, features, next, String::new(), false).into_response()
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Form(form): Form<LoginForm>,
) -> Response {
    let next = sanitize_next(form.next.as_deref());
    let Some(auth) = &state.auth else {
        return redirect_to(&state.url_prefix, &next);
    };

    let username = form.username.trim().to_string();
    if auth.check_password(&username, form.password).await {
        if let Some(value) = auth.issue_session(&username) {
            tracing::info!(user = ?username, "signed in");
            let secure = crate::server::ui::forwarded_https(&headers);
            let cookie = session::session_cookie(&value, &state.url_prefix, secure);
            return (
                [(header::SET_COOKIE, cookie)],
                redirect_to(&state.url_prefix, &next),
            )
                .into_response();
        }
    }

    // Debug-formatted, so a name full of control characters cannot forge log
    // lines.
    tracing::warn!(user = ?username, "failed sign-in");
    (
        StatusCode::UNAUTHORIZED,
        login_template(&state, lang, features, next, username, true),
    )
        .into_response()
}

pub async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let secure = crate::server::ui::forwarded_https(&headers);
    (
        [(
            header::SET_COOKIE,
            session::clear_cookie(&state.url_prefix, secure),
        )],
        redirect_to(&state.url_prefix, "/"),
    )
        .into_response()
}

fn login_template(
    state: &AppState,
    lang: LanguageIdentifier,
    features: FeatureFlags,
    next: String,
    username: String,
    failed: bool,
) -> LoginTemplate {
    LoginTemplate {
        active: String::new(),
        next,
        username,
        failed,
        tr: Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        repo_url: None,
        features,
        viewer: Viewer::guest(),
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_next;

    #[test]
    fn keeps_local_paths() {
        assert_eq!(sanitize_next(Some("/edit/Soup.cook")), "/edit/Soup.cook");
        assert_eq!(
            sanitize_next(Some("/recipe/A%20B.cook?scale=2")),
            "/recipe/A%20B.cook?scale=2"
        );
        assert_eq!(sanitize_next(Some("/")), "/");
    }

    #[test]
    fn rejects_everything_else() {
        for bad in [
            None,
            Some(""),
            Some("edit/Soup.cook"),
            Some("//evil.test"),
            Some("/\\evil.test"),
            Some("https://evil.test/"),
            Some("/has space"),
            Some("/new\nline"),
            Some("/é"),
        ] {
            assert_eq!(sanitize_next(bad), "/", "{bad:?}");
        }
    }
}
