//! Structured diagnostics shared by every command.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

/// Where in a source file a diagnostic applies.
///
/// `span` is a byte range into the file content, matching `cooklang`'s
/// source-span convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub file: Option<Utf8PathBuf>,
    pub span: Option<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

impl Diagnostic {
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            location: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            location: None,
        }
    }

    /// Attach a source file to this diagnostic.
    pub fn at_file(mut self, file: impl Into<Utf8PathBuf>) -> Self {
        let location = self.location.get_or_insert(Location {
            file: None,
            span: None,
        });
        location.file = Some(file.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_constructor_sets_severity() {
        let d = Diagnostic::warning("bad aisle line");
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.message, "bad aisle line");
        assert!(d.location.is_none());
    }

    #[test]
    fn at_file_attaches_location() {
        let d = Diagnostic::error("boom").at_file("config/aisle.conf");
        let location = d.location.expect("location set");
        assert_eq!(
            location.file.as_deref().map(|p| p.as_str()),
            Some("config/aisle.conf")
        );
    }

    #[test]
    fn serializes_without_null_location() {
        let d = Diagnostic::warning("no location");
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, r#"{"severity":"warning","message":"no location"}"#);
    }
}
