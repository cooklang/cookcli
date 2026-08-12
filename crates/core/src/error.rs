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
        /// How the recipe is identified in messages. Commands reading from
        /// disk put the file's path here, so that it agrees with the location
        /// `rendered` and the diagnostics point at; text parsed from memory
        /// carries whatever name the caller supplied.
        name: String,
        /// The individual parse problems, for programmatic consumers.
        diagnostics: Vec<Diagnostic>,
        /// The parser's own multi-line report with source line context, which
        /// the CLI prints verbatim. Not part of `Display`.
        ///
        /// Always free of ANSI escape codes, so it is safe to write to a file
        /// or send over a wire. Callers wanting colour for a terminal should
        /// re-render with [`render_report`](crate::parser::render_report) and
        /// `ansi: true`.
        rendered: String,
    },

    /// A scaling factor was not a finite number.
    ///
    /// Only NaN and infinity are rejected. Zero and negative factors are
    /// accepted, because the CLI accepts them and this crate must not change
    /// that behaviour. NaN is worth catching because a missing JavaScript
    /// argument arrives as `undefined`, which becomes NaN across NAPI and
    /// would otherwise silently produce NaN quantities.
    #[error("scale factor must be finite, but was {scale}")]
    InvalidScale {
        /// The rejected factor.
        scale: f64,
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

    /// A recipe reference could not be expanded into its ingredients.
    ///
    /// The referenced recipe was found and parsed; what failed was working out
    /// how much of it the referring recipe wants — an unusable quantity on the
    /// reference, or a target the referenced recipe cannot be scaled to.
    /// Absence and parse failures are [`CoreError::RecipeNotFound`] and
    /// [`CoreError::Parse`] as usual.
    #[error("cannot expand recipe reference '{name}': {message}")]
    Reference {
        /// The referenced recipe, as it is spelled in the referring recipe.
        name: String,
        /// What went wrong.
        message: String,
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
            | CoreError::InvalidScale { .. }
            | CoreError::Config { .. }
            | CoreError::Render { .. }
            | CoreError::CircularReference { .. }
            | CoreError::Reference { .. }
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
            CoreError::InvalidScale { scale: f64::NAN },
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
            CoreError::Reference {
                name: "./sauce".to_string(),
                message: "unit mismatch (expected ml, got g)".to_string(),
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
