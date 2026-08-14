//! Render Cooklang recipes into text formats.
//!
//! Each module turns a parsed [`cooklang::Recipe`] into one target format.
//! The `print_*` functions write into a [`std::io::Write`]; the `*_to_string`
//! wrappers are for callers that want a `String`.

#![warn(missing_docs)]

/// Human-friendly rendering of quantity numbers.
pub mod number;
/// Deterministic ordering for grouped quantities.
pub mod quantity;
/// schema.org/Recipe JSON-LD rendering of a recipe.
pub mod schema;

/// The `cooklang` crate this library was built against.
///
/// Every public function takes `cooklang` types, so they are part of this
/// crate's public surface. Re-exporting lets consumers name them without
/// adding their own `cooklang` dependency, which could otherwise resolve to a
/// different version and fail to unify.
pub use cooklang;

/// The separator a recipe reference's path is built and reported with.
///
/// **Always `/`, never [`std::path::MAIN_SEPARATOR`].** A reference is written
/// `@./sauce{}` in Cooklang, so `/` is the separator the user typed and the one
/// they should be shown back. Joining with the platform separator instead made
/// Windows disagree with itself: `doctor` reports a broken reference as
/// `./absent` because it joins with `/`, while the shopping list reported the
/// same one as `.\absent`, and the `./`-stripping in [`get_recipe`] only
/// matches the forward-slash form, so the prefix survived into the reported
/// name (<https://github.com/cooklang/cookcli/issues/442>).
///
/// Resolution is unaffected either way — `Utf8Path::join` takes `.\sauce` and
/// `./sauce` alike on Windows — but two of these paths are not diagnostics at
/// all: the Cooklang writer re-emits the reference as source, where a backslash
/// is not valid syntax, and the Markdown writer puts it in a link target.
///
/// On Unix this is what [`std::path::MAIN_SEPARATOR`] already was, so nothing
/// about the output changes there.
pub const REFERENCE_SEPARATOR: &str = "/";

/// A test-only stand-in for `cookcli-core`'s parser.
///
/// The formatters take an already-parsed recipe, so the parser is not part of
/// this crate's public surface — but its own tests still need to build a
/// `Recipe` from source. This reproduces `cookcli_core::parser`'s
/// configuration exactly (no extensions, default converter) and its call
/// shape, so the tests read the same on both sides of the split.
#[cfg(test)]
pub(crate) mod test_support {
    use cooklang::{Converter, CooklangParser, Extensions, Recipe};
    use std::sync::LazyLock;

    pub(crate) static PARSER: LazyLock<CooklangParser> =
        LazyLock::new(|| CooklangParser::new(Extensions::empty(), Converter::default()));

    /// Stands in for `cookcli_core::Outcome`, of which the formatter tests
    /// only ever use `.value`.
    pub(crate) struct Parsed {
        pub(crate) value: Recipe,
    }

    /// Parse and scale, mirroring `cookcli_core::parse_recipe`.
    ///
    /// `scale` is applied unconditionally, including at `1.0`, because that is
    /// what core does — see the note on `parse_unscaled` there.
    pub(crate) fn parse_recipe(text: &str, name: &str, scale: f64) -> Result<Parsed, String> {
        let parsed = PARSER.parse(text);
        if parsed.report().has_errors() {
            return Err(format!("{name} failed to parse"));
        }
        match parsed.into_result() {
            Ok((mut recipe, _)) => {
                recipe.scale(scale, PARSER.converter());
                Ok(Parsed { value: recipe })
            }
            Err(_) => Err(format!("{name} produced no output")),
        }
    }
}
