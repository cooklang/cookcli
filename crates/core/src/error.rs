//! The public error type.

use crate::diagnostic::Diagnostic;
use camino::Utf8PathBuf;

/// Errors returned by `cookcli-core` commands.
///
/// `#[non_exhaustive]` so that adding variants stays non-breaking for
/// downstream consumers.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("recipe not found: {name}")]
    RecipeNotFound { name: String },

    /// A recipe failed to parse. `rendered` is `cooklang`'s own report output
    /// with source line context, which the CLI prints verbatim.
    #[error("failed to parse recipe '{name}'\n{rendered}")]
    Parse {
        name: String,
        diagnostics: Vec<Diagnostic>,
        rendered: String,
    },

    #[error("invalid configuration at {path}: {message}")]
    Config { path: Utf8PathBuf, message: String },

    #[error("template rendering failed: {message}")]
    Render { message: String },

    #[error("circular recipe reference: {chain}")]
    CircularReference { chain: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
