//! Structured diagnostics shared by every command.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

/// How much a [`Diagnostic`] matters.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The input is broken. Whether this is fatal depends on the command: see
    /// [`Outcome`](crate::Outcome) for when errors are reported as data rather
    /// than returned as `Err`.
    Error,
    /// The input is usable but suspect, and the result may not be what the
    /// author intended.
    Warning,
    /// A suggestion. Nothing is wrong.
    Hint,
}

/// A byte range into the source file: `start..end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// Byte offset of the first byte of the range.
    pub start: usize,
    /// Byte offset one past the last byte of the range.
    pub end: usize,
}

/// Where in a source file a diagnostic applies.
///
/// Both fields are optional: a diagnostic about a configuration file as a whole
/// has a file but no span, and one raised before any source is known has
/// neither.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// The file the diagnostic refers to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<Utf8PathBuf>,
    /// The range within that file the diagnostic refers to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

/// A single problem found while running a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// How much this diagnostic matters.
    pub severity: Severity,
    /// Human-readable description of the problem, one line, no trailing period.
    pub message: String,
    /// Where the problem is, when that is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

impl Diagnostic {
    /// A diagnostic with [`Severity::Warning`] and no location.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            location: None,
        }
    }

    /// A diagnostic with [`Severity::Error`] and no location.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            location: None,
        }
    }

    /// A diagnostic with [`Severity::Hint`] and no location.
    pub fn hint(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Hint,
            message: message.into(),
            location: None,
        }
    }

    /// Attach a source file to this diagnostic, keeping any span already set.
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
    fn hint_constructor_sets_severity() {
        let d = Diagnostic::hint("consider adding a servings metadata key");
        assert_eq!(d.severity, Severity::Hint);
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
    fn at_file_preserves_an_existing_span() {
        let d = Diagnostic {
            severity: Severity::Error,
            message: "boom".to_string(),
            location: Some(Location {
                file: None,
                span: Some(Span { start: 12, end: 20 }),
            }),
        }
        .at_file("soup.cook");

        let location = d.location.expect("location set");
        assert_eq!(
            location.file.as_deref().map(|p| p.as_str()),
            Some("soup.cook")
        );
        assert_eq!(location.span, Some(Span { start: 12, end: 20 }));
    }

    #[test]
    fn serializes_without_null_location() {
        let d = Diagnostic::warning("no location");
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, r#"{"severity":"warning","message":"no location"}"#);
    }

    #[test]
    fn serializes_a_full_location_with_a_named_span() {
        let d = Diagnostic {
            severity: Severity::Error,
            message: "bad quantity".to_string(),
            location: Some(Location {
                file: Some(Utf8PathBuf::from("soup.cook")),
                span: Some(Span { start: 12, end: 20 }),
            }),
        };
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(
            json,
            r#"{"severity":"error","message":"bad quantity","location":{"file":"soup.cook","span":{"start":12,"end":20}}}"#
        );
    }

    #[test]
    fn omitted_fields_deserialize_back_to_none() {
        let d: Diagnostic =
            serde_json::from_str(r#"{"severity":"warning","message":"no location"}"#).unwrap();
        assert_eq!(d, Diagnostic::warning("no location"));

        let d: Diagnostic = serde_json::from_str(
            r#"{"severity":"error","message":"m","location":{"file":"a.cook"}}"#,
        )
        .unwrap();
        let location = d.location.expect("location set");
        assert_eq!(location.file.as_deref().map(|p| p.as_str()), Some("a.cook"));
        assert_eq!(location.span, None);

        let d: Diagnostic =
            serde_json::from_str(r#"{"severity":"hint","message":"m","location":{}}"#).unwrap();
        assert_eq!(
            d.location,
            Some(Location {
                file: None,
                span: None
            })
        );
    }
}
