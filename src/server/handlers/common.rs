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

/// Rewrites the `tags` entry of a serialised metadata map into an array.
///
/// YAML frontmatter takes `tags: a, b` as well as `tags: [a, b]`, and the
/// parser keeps whichever type was written, so serialising the value verbatim
/// hands clients two different shapes for the same key. Every endpoint that
/// reports metadata runs it through here: a string is split the way
/// `cooklang`'s own `Metadata::tags` splits it — on commas, trimmed, with empty
/// entries and duplicates dropped — and any other type is left alone.
pub fn normalize_tags(metadata: &mut serde_json::Value) {
    let Some(raw) = metadata.get("tags").and_then(|v| v.as_str()) else {
        return;
    };

    let mut tags: Vec<&str> = Vec::new();
    for tag in raw.split(',').map(str::trim) {
        if tag.is_empty() || tags.contains(&tag) {
            continue;
        }
        tags.push(tag);
    }

    metadata["tags"] = serde_json::json!(tags);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn comma_separated_tags_become_an_array() {
        let mut metadata = json!({ "tags": "tag1, tag2" });
        normalize_tags(&mut metadata);
        assert_eq!(metadata, json!({ "tags": ["tag1", "tag2"] }));
    }

    #[test]
    fn a_single_tag_becomes_a_one_element_array() {
        let mut metadata = json!({ "tags": "tag1" });
        normalize_tags(&mut metadata);
        assert_eq!(metadata, json!({ "tags": ["tag1"] }));
    }

    #[test]
    fn blank_and_duplicate_entries_are_dropped() {
        let mut metadata = json!({ "tags": " a ,, b , a" });
        normalize_tags(&mut metadata);
        assert_eq!(metadata, json!({ "tags": ["a", "b"] }));
    }

    #[test]
    fn other_types_are_left_alone() {
        let mut metadata = json!({ "tags": ["a", "b"], "title": "Stew" });
        normalize_tags(&mut metadata);
        assert_eq!(metadata, json!({ "tags": ["a", "b"], "title": "Stew" }));

        let mut without_tags = json!({ "title": "Stew" });
        normalize_tags(&mut without_tags);
        assert_eq!(without_tags, json!({ "title": "Stew" }));
    }
}
