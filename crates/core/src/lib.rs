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

pub mod diagnostic;
pub mod error;
pub mod outcome;
pub mod source;

pub use diagnostic::{Diagnostic, Location, Severity, Span};
pub use error::CoreError;
pub use outcome::Outcome;
pub use source::{ConfigSource, RecipeSource};

/// Convenience alias for core results.
pub type Result<T> = std::result::Result<T, CoreError>;
