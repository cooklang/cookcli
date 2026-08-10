//! The result wrapper every command returns.

use crate::diagnostic::Diagnostic;

/// A command result paired with any non-fatal diagnostics produced along the way.
///
/// `cooklang` parses leniently: a recipe can parse successfully and still carry
/// warnings. Consumers that do not care about diagnostics ignore the field.
#[derive(Debug, Clone)]
pub struct Outcome<T> {
    pub value: T,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> Outcome<T> {
    /// Wrap a value with no diagnostics.
    pub fn new(value: T) -> Self {
        Self {
            value,
            diagnostics: Vec::new(),
        }
    }

    /// Wrap a value together with diagnostics.
    pub fn with_diagnostics(value: T, diagnostics: Vec<Diagnostic>) -> Self {
        Self { value, diagnostics }
    }

    /// Discard diagnostics and take the value.
    pub fn into_value(self) -> T {
        self.value
    }

    /// True when any diagnostic has `Severity::Error`.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == crate::diagnostic::Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Diagnostic, Severity};

    #[test]
    fn new_has_no_diagnostics() {
        let outcome = Outcome::new(42);
        assert_eq!(outcome.value, 42);
        assert!(outcome.diagnostics.is_empty());
        assert!(!outcome.has_errors());
    }

    #[test]
    fn has_errors_detects_error_severity() {
        let outcome = Outcome::with_diagnostics(
            (),
            vec![Diagnostic {
                severity: Severity::Warning,
                message: "just a warning".to_string(),
                location: None,
            }],
        );
        assert!(!outcome.has_errors());

        let outcome = Outcome::with_diagnostics(
            (),
            vec![Diagnostic {
                severity: Severity::Error,
                message: "a real error".to_string(),
                location: None,
            }],
        );
        assert!(outcome.has_errors());
    }

    #[test]
    fn into_value_discards_diagnostics() {
        let outcome = Outcome::with_diagnostics(
            "hello",
            vec![Diagnostic {
                severity: Severity::Hint,
                message: "hint".to_string(),
                location: None,
            }],
        );
        assert_eq!(outcome.into_value(), "hello");
    }
}
