//! The result wrapper every command returns.

use crate::diagnostic::Diagnostic;

/// A command result paired with any non-fatal diagnostics produced along the
/// way.
///
/// `cooklang` parses leniently: a recipe can parse successfully and still carry
/// warnings. Consumers that do not care about diagnostics ignore the field.
///
/// # When a command returns `Err` and when it returns error diagnostics
///
/// An `Outcome` on the success path may still carry [`Severity::Error`]
/// diagnostics. The rule commands follow is:
///
/// - Return `Err(CoreError)` when the command could not produce its value.
///   `recipe::read` cannot return a recipe that failed to parse, so it returns
///   `Err`.
/// - Return `Ok(Outcome)` with error-severity diagnostics when producing the
///   value *is* the job and the errors are the payload. `doctor::validate`
///   reports broken recipes as data and must still succeed.
///
/// Callers that treat any error diagnostic as failure — a CI exit code, say —
/// check [`has_errors`](Outcome::has_errors) in addition to the `Result`.
///
/// [`Severity::Error`]: crate::Severity::Error
#[derive(Debug, Clone)]
pub struct Outcome<T> {
    /// What the command produced.
    pub value: T,
    /// Problems found while producing it. May be empty.
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

    /// True when any diagnostic has [`Severity::Error`](crate::Severity::Error).
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == crate::diagnostic::Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Diagnostic;

    #[test]
    fn new_has_no_diagnostics() {
        let outcome = Outcome::new(42);
        assert_eq!(outcome.value, 42);
        assert!(outcome.diagnostics.is_empty());
        assert!(!outcome.has_errors());
    }

    #[test]
    fn has_errors_detects_error_severity() {
        let outcome = Outcome::with_diagnostics((), vec![Diagnostic::warning("just a warning")]);
        assert!(!outcome.has_errors());

        let outcome = Outcome::with_diagnostics((), vec![Diagnostic::error("a real error")]);
        assert!(outcome.has_errors());

        // A hint is not an error either.
        let outcome = Outcome::with_diagnostics((), vec![Diagnostic::hint("a hint")]);
        assert!(!outcome.has_errors());
    }

    #[test]
    fn into_value_discards_diagnostics() {
        let outcome = Outcome::with_diagnostics("hello", vec![Diagnostic::hint("hint")]);
        assert_eq!(outcome.into_value(), "hello");
    }
}
