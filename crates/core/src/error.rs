//! The public error type.

use crate::diagnostic::Diagnostic;
use camino::Utf8PathBuf;

/// Errors returned by `cookcli-core` commands.
///
/// Every `Display` rendering is a single lowercase line with no trailing
/// newline, so it composes into log lines and error chains. Variants that have
/// a longer, human-formatted report carry it in a field for the caller to print
/// separately.
///
/// `#[non_exhaustive]` so that adding variants stays non-breaking for
/// downstream consumers.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// No recipe could be resolved from the given path or name.
    #[error("recipe not found: {name}")]
    RecipeNotFound {
        /// The path or name that was looked up.
        name: String,
    },

    /// A recipe failed to parse.
    #[error("failed to parse recipe '{name}'")]
    Parse {
        /// The recipe that failed to parse.
        name: String,
        /// The individual parse problems, for programmatic consumers.
        diagnostics: Vec<Diagnostic>,
        /// The parser's own multi-line report with source line context, which
        /// the CLI prints verbatim. Not part of `Display`.
        rendered: String,
    },

    /// A configuration could not be understood.
    #[error("invalid configuration{}: {message}", .path.as_ref().map(|p| format!(" at {p}")).unwrap_or_default())]
    Config {
        /// The file the configuration came from, absent when it was supplied
        /// inline.
        path: Option<Utf8PathBuf>,
        /// What was wrong with it.
        message: String,
    },

    /// A report template failed to render.
    #[error("template rendering failed: {message}")]
    Render {
        /// What the template engine reported.
        message: String,
    },

    /// A recipe referenced itself, directly or through other recipes.
    #[error("circular recipe reference: {chain}")]
    CircularReference {
        /// The reference cycle, as recipe names joined by arrows.
        chain: String,
    },

    /// A file could not be read or written.
    ///
    /// There is deliberately no `From<std::io::Error>`: every call site must
    /// name the path it was working on. The message stays neutral between
    /// reading and writing, because the variant covers both; the specific
    /// failure is in `source`.
    #[error("i/o error on {path}")]
    Io {
        /// The file being accessed.
        path: Utf8PathBuf,
        /// The underlying operating system error.
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(error: &CoreError) -> String {
        error.to_string()
    }

    /// Stops compiling when a `CoreError` variant is added or removed.
    ///
    /// `every_display_is_a_single_line` below has to list its inputs by hand,
    /// so it cannot notice a new variant on its own. When this match breaks,
    /// add the variant here *and* to that test's `errors` array — do not just
    /// add a `_ => {}` arm.
    fn _all_variants_are_covered(e: &CoreError) {
        match e {
            CoreError::RecipeNotFound { .. }
            | CoreError::Parse { .. }
            | CoreError::Config { .. }
            | CoreError::Render { .. }
            | CoreError::CircularReference { .. }
            | CoreError::Io { .. } => {}
        }
    }

    #[test]
    fn every_display_is_a_single_line() {
        let errors = [
            CoreError::RecipeNotFound {
                name: "soup".to_string(),
            },
            CoreError::Parse {
                name: "soup".to_string(),
                diagnostics: vec![Diagnostic::error("bad quantity")],
                rendered: "line 1\nline 2\n".to_string(),
            },
            CoreError::Config {
                path: None,
                message: "unknown section".to_string(),
            },
            CoreError::Render {
                message: "undefined variable".to_string(),
            },
            CoreError::CircularReference {
                chain: "a -> b -> a".to_string(),
            },
            CoreError::Io {
                path: Utf8PathBuf::from("config/aisle.conf"),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
            },
        ];

        for error in &errors {
            let rendered = display(error);
            assert!(!rendered.contains('\n'), "multi-line Display: {rendered:?}");
            assert!(
                !rendered.ends_with(char::is_whitespace),
                "trailing whitespace: {rendered:?}"
            );

            // Lowercase, so it reads correctly mid-sentence in an error chain.
            // A rendering may open with a path or another interpolated value,
            // so judge the first cased letter rather than the first character.
            if let Some(c) = rendered.chars().find(|c| c.is_alphabetic()) {
                assert!(
                    !c.is_uppercase(),
                    "Display starts with an uppercase word: {rendered:?}"
                );
            }
        }
    }

    #[test]
    fn parse_keeps_the_rendered_report_out_of_display() {
        let error = CoreError::Parse {
            name: "soup".to_string(),
            diagnostics: Vec::new(),
            rendered: "a very long report".to_string(),
        };
        assert_eq!(display(&error), "failed to parse recipe 'soup'");
    }

    #[test]
    fn config_display_covers_both_path_and_inline() {
        let with_path = CoreError::Config {
            path: Some(Utf8PathBuf::from("config/aisle.conf")),
            message: "unknown section".to_string(),
        };
        assert_eq!(
            display(&with_path),
            "invalid configuration at config/aisle.conf: unknown section"
        );

        let inline = CoreError::Config {
            path: None,
            message: "unknown section".to_string(),
        };
        assert_eq!(display(&inline), "invalid configuration: unknown section");
    }

    #[test]
    fn io_display_names_the_path() {
        let error = CoreError::Io {
            path: Utf8PathBuf::from("/etc/pantry.conf"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        assert_eq!(display(&error), "i/o error on /etc/pantry.conf");
        assert!(std::error::Error::source(&error).is_some());
    }
}
