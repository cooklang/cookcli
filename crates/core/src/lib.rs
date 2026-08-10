//! Recipe, shopping list, pantry and report operations for Cooklang.
//!
//! This crate holds the logic behind CookCLI's commands, with the CLI reduced
//! to argument parsing and output formatting on top of it.

pub mod diagnostic;
pub mod error;
pub mod outcome;
pub mod source;

pub use diagnostic::{Diagnostic, Location, Severity};
pub use error::CoreError;
pub use outcome::Outcome;
pub use source::{ConfigSource, RecipeSource};

/// Convenience alias for core results.
pub type Result<T> = std::result::Result<T, CoreError>;
