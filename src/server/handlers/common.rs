use axum::{http::StatusCode, Json};
use camino::{Utf8Component, Utf8Path};

pub fn json_error(msg: impl std::fmt::Display) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": msg.to_string() }))
}

pub fn check_path(p: &str) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let path = Utf8Path::new(p);
    if !path
        .components()
        .all(|c| matches!(c, Utf8Component::Normal(_)))
    {
        tracing::error!("Invalid path: {p}");
        return Err((
            StatusCode::BAD_REQUEST,
            json_error(format!("Invalid path: {p}")),
        ));
    }
    Ok(())
}
