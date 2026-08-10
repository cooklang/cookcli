//! Recipe, shopping list, pantry and report operations for Cooklang.
//!
//! This crate holds the logic behind CookCLI's commands, with the CLI reduced
//! to argument parsing and output formatting on top of it.
//!
//! Commands take their input as a [`RecipeSource`] or [`ConfigSource`] rather
//! than a path, so an editor can pass an unsaved buffer. They return
//! [`Outcome<T>`], which pairs the result with any [`Diagnostic`]s raised on
//! the way, or a [`CoreError`] when no result could be produced.

#![warn(missing_docs)]

pub mod context;
pub mod diagnostic;
pub mod error;
pub mod format;
pub mod outcome;
pub mod parser;
pub mod recipe;
pub mod source;

pub use context::{global_config_path, Context};
pub use diagnostic::{Diagnostic, Location, Severity, Span};
pub use error::CoreError;
pub use format::{PaperSize, Style};
pub use outcome::Outcome;
pub use parser::{parse_recipe, parse_recipe_at, render_report, PARSER};

/// The `cooklang` crate this library was built against.
///
/// [`parse_recipe`] returns a [`cooklang::Recipe`] and [`PARSER`] is a
/// [`cooklang::CooklangParser`], so those types are part of this crate's public
/// surface. Re-exporting lets consumers name them without adding their own
/// `cooklang` dependency, which could otherwise resolve to a different
/// version and fail to unify.
pub use cooklang;
pub use source::{ConfigSource, RecipeSource};

/// Convenience alias for core results.
pub type Result<T> = std::result::Result<T, CoreError>;
