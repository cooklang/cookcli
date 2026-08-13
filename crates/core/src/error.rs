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

    /// A command needed a configuration the context does not carry.
    ///
    /// Distinct from [`CoreError::Config`], which means one was supplied and
    /// could not be understood. Not every command needs one:
    /// `shopping_list::generate` treats an absent pantry as "subtract
    /// nothing", where the pantry queries cannot, because the pantry is the
    /// thing they report on.
    #[error("no {kind} configuration")]
    MissingConfig {
        /// Which configuration is missing, as it is named to the user:
        /// `"pantry"` is the only one anything returns today.
        kind: String,
    },

    /// A command that changes a configuration was handed one it cannot write
    /// back.
    ///
    /// Reached only through [`ConfigSource::Inline`](crate::ConfigSource):
    /// an editor holding pantry text in a buffer has somewhere to put an edit,
    /// but this crate has no idea where. Applying the change is the caller's
    /// to do, or it can supply a
    /// [`ConfigSource::Path`](crate::ConfigSource::Path) instead.
    ///
    /// Distinct from [`CoreError::MissingConfig`], which is having nothing to
    /// change at all.
    #[error("cannot write the {kind} configuration: it was supplied inline rather than as a file")]
    ReadOnlyConfig {
        /// Which configuration, as it is named to the user: `"pantry"` is the
        /// only one anything returns today.
        kind: String,
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

    /// A pantry could not be changed as asked.
    ///
    /// The file was read and understood; it is the change that does not apply
    /// — adding an item that is already there, or naming an item or section
    /// that is not. Distinct from [`CoreError::Config`], which is a file that
    /// could not be read as a pantry at all, and from [`CoreError::Io`], which
    /// is a file that could not be read or written.
    ///
    /// Nothing has been written when this is returned: every check runs before
    /// the pantry is saved.
    #[error("cannot change the pantry: {message}")]
    PantryEdit {
        /// What stopped the change, naming the item and section it was asked
        /// about.
        message: String,
    },

    /// A report could not be produced from a template and a recipe.
    ///
    /// Covers a broken *template* and a recipe `cooklang-reports` could not
    /// parse alike, because that crate parses the recipe itself, with its own
    /// parser configuration, from inside the render call — the two failures
    /// arrive down one channel and are not worth guessing apart. A recipe that
    /// fails core's own parser still yields [`CoreError::Parse`]; it is only
    /// [`report::render`](crate::report::render) that reports both this way.
    #[error("template rendering failed: {message}")]
    Render {
        /// A one-line summary, for logs and error chains.
        message: String,
        /// The template engine's own multi-line report — source location,
        /// error chain, and its hints — which the CLI prints verbatim. Not part
        /// of `Display`, exactly as [`CoreError::Parse`]'s `rendered` is not.
        rendered: String,
    },

    // No variant for a circular recipe reference. Reference expansion is
    // bounded rather than recursive, so a cycle neither loops nor fails — it
    // silently double-counts the ingredients it revisits, which is
    // <https://github.com/cooklang/cookcli/issues/424>. Whoever fixes that
    // needs to reintroduce a variant here; it was removed rather than left
    // unreachable, because a public variant nothing can return invites
    // consumers to write dead match arms and implies a guarantee this crate
    // does not make.
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

    /// A directory could not be searched.
    ///
    /// Distinct from [`CoreError::Io`] because nothing was read: the search
    /// root itself could not be turned into something searchable, so the walk
    /// never started. A root whose name contains glob syntax — `notes[2024]` —
    /// is the way to reach this, since `cooklang-find` builds its file pattern
    /// by joining onto the root without escaping it. Reporting that as a failed
    /// read would send the user looking at permissions.
    #[error("cannot search '{base_dir}': {message}")]
    Search {
        /// The directory that could not be searched.
        base_dir: Utf8PathBuf,
        /// What went wrong with it.
        message: String,
    },

    /// A saved shopping list could not be read as one.
    ///
    /// The `.shopping-list` file was read; it is its contents that could not be
    /// parsed. Distinct from [`CoreError::Io`], which is the file itself being
    /// unreadable, and from [`CoreError::Parse`], which is a recipe.
    ///
    /// Reachable because the file is a plain text format users and other
    /// Cooklang apps edit directly, so this crate is not the only thing that
    /// writes it.
    #[error("invalid shopping list at {path}: {message}")]
    InvalidShoppingList {
        /// The shopping list that could not be parsed.
        path: Utf8PathBuf,
        /// What was wrong with it, as the format parser reported it.
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
            | CoreError::MissingConfig { .. }
            | CoreError::ReadOnlyConfig { .. }
            | CoreError::Config { .. }
            | CoreError::PantryEdit { .. }
            | CoreError::Render { .. }
            | CoreError::Reference { .. }
            | CoreError::Search { .. }
            | CoreError::InvalidShoppingList { .. }
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
            CoreError::MissingConfig {
                kind: "pantry".to_string(),
            },
            CoreError::ReadOnlyConfig {
                kind: "pantry".to_string(),
            },
            CoreError::Config {
                path: None,
                message: "unknown section".to_string(),
            },
            CoreError::PantryEdit {
                message: "item 'flour' already exists in section 'pantry'".to_string(),
            },
            CoreError::Render {
                message: "undefined variable".to_string(),
                rendered: "line 1\nline 2\n".to_string(),
            },
            CoreError::Reference {
                name: "./sauce".to_string(),
                message: "unit mismatch (expected ml, got g)".to_string(),
            },
            CoreError::Search {
                base_dir: Utf8PathBuf::from("/recipes/notes[2024]"),
                message: "Pattern syntax error near position 20".to_string(),
            },
            CoreError::InvalidShoppingList {
                path: Utf8PathBuf::from("/recipes/.shopping-list"),
                message: "Invalid multiplier: expected a number".to_string(),
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
    fn render_keeps_the_rendered_report_out_of_display() {
        let error = CoreError::Render {
            message: "syntax error: unexpected end of input".to_string(),
            rendered: "a very long report\nwith hints\n".to_string(),
        };
        assert_eq!(
            display(&error),
            "template rendering failed: syntax error: unexpected end of input"
        );
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
