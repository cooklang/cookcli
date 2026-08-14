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
