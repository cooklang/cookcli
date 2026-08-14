//! Recipe output formatters.
//!
//! The recipe formatters live in [`cooklang_format`], which is publishable on
//! its own so other projects can render recipes without depending on this
//! crate. They are re-exported here so that callers keep reaching them as
//! `cookcli_core::format::..`.
//!
//! [`shopping_list`] stays here: it renders this crate's
//! [`AggregatedList`](crate::shopping_list::AggregatedList), so moving it would
//! point the dependency between the two crates the wrong way.

pub mod shopping_list;

// `cooklang_source as cooklang`: in this crate the formatter can keep the bare
// name, but in `cooklang-format` it would collide with the re-exported
// `cooklang` parser crate at that crate's root. The alias keeps
// `format::cooklang::print_cooklang` working for `src/recipe/read.rs`; the
// unaliased name is re-exported too, so a reader who found it under that name
// in `cooklang-format`'s documentation can reach it here by the same spelling.
pub use cooklang_format::{
    cooklang_source, cooklang_source as cooklang, human, human_to_string, latex, markdown,
    markdown_to_string, number, quantity, schema, typst, PaperSize, Style,
};
