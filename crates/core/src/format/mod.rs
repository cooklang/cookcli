//! Recipe output formatters.
//!
//! Each submodule renders a parsed [`cooklang::Recipe`] into one target
//! format. The `print_*` functions write into a [`std::io::Write`], so a
//! caller already holding a file or socket does not pay for a second copy of
//! the whole document — though they do buffer a step or a table at a time
//! internally. The `*_to_string` wrappers here are for callers that want a
//! `String`, such as the NAPI addon.
//!
//! # Errors
//!
//! These functions return a bare [`std::io::Error`] rather than
//! [`CoreError`](crate::CoreError). The only thing that can fail is the
//! caller's own writer: the recipe is already parsed, and every field is
//! optional, so there is nothing left to reject. `CoreError::Io` also wants a
//! path, and a formatter has none.

// `cooklang_format` cannot export the writer module under the bare name
// `cooklang` — that name is already taken there by the re-exported
// `cooklang` crate. Aliasing it back to `cooklang` here preserves this
// crate's public path `format::cooklang::print_cooklang`, which
// `src/recipe/read.rs:228` depends on.
pub use cooklang_format::{
    cooklang_source as cooklang, human, human_to_string, latex, markdown, markdown_to_string,
    number, quantity, schema, typst, PaperSize, Style,
};
pub mod shopping_list;
