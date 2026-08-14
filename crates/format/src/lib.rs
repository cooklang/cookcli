//! Render Cooklang recipes into text formats.
//!
//! Each module turns a parsed [`cooklang::Recipe`] into one target format.
//! The `print_*` functions write into a [`std::io::Write`]; the `*_to_string`
//! wrappers are for callers that want a `String`.

#![warn(missing_docs)]

/// The `cooklang` crate this library was built against.
///
/// Every public function takes `cooklang` types, so they are part of this
/// crate's public surface. Re-exporting lets consumers name them without
/// adding their own `cooklang` dependency, which could otherwise resolve to a
/// different version and fail to unify.
pub use cooklang;
