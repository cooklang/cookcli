//! Render Cooklang recipes into text formats.
//!
//! Each module turns a parsed [`cooklang::Recipe`] into one target format.
//! The `print_*` functions write into a [`std::io::Write`]; the `*_to_string`
//! wrappers are for callers that want a `String`.
//!
//! # Errors
//!
//! These functions return a bare [`std::io::Error`]. The only thing that can
//! fail is the caller's own writer: the recipe is already parsed, and every
//! field is optional, so there is nothing left to reject.

#![warn(missing_docs)]

// Declared bare, without `///` docs: a doc attribute on a `mod` declaration is
// merged with the module's own `//!` header and the whole thing then resolves
// its intra-doc links in *this* scope rather than the module's, which breaks
// every link a module writes to its own items. Each module documents itself.
pub mod cooklang_source;
pub mod human;
pub mod latex;
pub mod markdown;
pub mod number;
pub mod quantity;
pub mod schema;
pub mod typst;

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
/// same one as `.\absent`, and the `./`-stripping in `cookcli-core`'s recipe
/// lookup only matches the forward-slash form, so the prefix survived into the reported
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

use ::cooklang::{convert::Converter, Recipe};

/// Whether formatters emit ANSI escape codes.
///
/// A library must not emit escape sequences by default, and `yansi`'s global
/// enable/disable is unacceptable shared mutable state in a published crate,
/// so colour is passed explicitly. The CLI passes `Ansi`; consumers get `Plain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Style {
    /// No escape codes: safe for files, pipes and editor buffers.
    #[default]
    Plain,
    /// Escape codes for a terminal that understands them.
    Ansi,
}

impl Style {
    /// True when ANSI escape codes should be emitted.
    pub fn is_ansi(self) -> bool {
        matches!(self, Style::Ansi)
    }
}

/// Page size for the [`latex`] and [`typst`] formatters.
///
/// Those formatters take the paper name as a string because each typesetter
/// spells it differently; this enum maps one choice onto both spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PaperSize {
    /// ISO A4, 210 x 297 mm.
    #[default]
    A4,
    /// US Letter, 8.5 x 11 in.
    Letter,
    /// ISO A5, 148 x 210 mm.
    A5,
    /// US Legal, 8.5 x 14 in.
    Legal,
}

impl PaperSize {
    /// The name LaTeX's `geometry`/`article` class expects.
    pub fn latex_name(self) -> &'static str {
        match self {
            PaperSize::A4 => "a4paper",
            PaperSize::Letter => "letterpaper",
            PaperSize::A5 => "a5paper",
            PaperSize::Legal => "legalpaper",
        }
    }

    /// The name Typst's `page(paper: ..)` expects.
    pub fn typst_name(self) -> &'static str {
        match self {
            PaperSize::A4 => "a4",
            PaperSize::Letter => "us-letter",
            PaperSize::A5 => "a5",
            PaperSize::Legal => "us-legal",
        }
    }
}

/// Render a recipe the way `cook recipe` prints it, into a `String`.
///
/// Convenience over [`human::print_human`], which stays the primitive.
pub fn human_to_string(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
    style: Style,
) -> std::io::Result<String> {
    let mut buf = Vec::new();
    human::print_human(recipe, name, scale, converter, style, &mut buf)?;
    into_string(buf)
}

/// Render a recipe as Markdown, into a `String`.
///
/// Convenience over [`markdown::print_md`], which stays the primitive.
pub fn markdown_to_string(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
) -> std::io::Result<String> {
    let mut buf = Vec::new();
    markdown::print_md(recipe, name, scale, converter, &mut buf)?;
    into_string(buf)
}

/// The formatters only ever write UTF-8, so this cannot fail in practice.
/// Returning rather than panicking keeps a NAPI consumer from taking down its
/// JavaScript host if that ever stops being true.
fn into_string(buf: Vec<u8>) -> std::io::Result<String> {
    String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Compiles `README.md`'s example as a doctest, so the crate's front page
/// cannot rot into something that no longer builds.
///
/// Exists only under `cfg(doctest)`, so it is not part of the public API and
/// does not appear in the rendered documentation.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

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

    /// Stands in for `cookcli_core::Outcome`, of which the formatter tests use
    /// `.value` and `.diagnostics`.
    pub(crate) struct Parsed {
        pub(crate) value: Recipe,
        /// The parse warnings, rendered.
        ///
        /// `cookcli_core::Outcome` carries structured `Diagnostic`s, built from
        /// the same `report.iter()` this reads. The tests here only ask whether
        /// the collection is empty and print it when it is not, so strings carry
        /// exactly the meaning they need without this crate depending on core —
        /// which it cannot do anyway, since core depends on this crate.
        pub(crate) diagnostics: Vec<String>,
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
        // Collected before `into_result` consumes the parse result. Errors are
        // already ruled out above, so what remains is warnings.
        let diagnostics = parsed
            .report()
            .iter()
            .map(|diag| diag.message.to_string())
            .collect();
        match parsed.into_result() {
            Ok((mut recipe, _)) => {
                recipe.scale(scale, PARSER.converter());
                Ok(Parsed {
                    value: recipe,
                    diagnostics,
                })
            }
            Err(_) => Err(format!("{name} produced no output")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{parse_recipe, PARSER};

    /// Exercises tags, a timer, an ingredient and a step: every place the
    /// human formatter reaches for a colour.
    const RECIPE: &str = "---\ntitle: Tea\ntags: [hot, quick]\n---\n\nBoil @water{2%cups} in a #pot for ~{5%minutes}.\n";

    fn human(style: Style) -> String {
        let recipe = parse_recipe(RECIPE, "Tea", 1.0).expect("parses").value;
        human_to_string(&recipe, "Tea", 1.0, PARSER.converter(), style).expect("formats")
    }

    /// The whole point of [`Style`]: `Plain` must not leak escape codes, and
    /// it must not lose anything else either. Both halves hold on every
    /// platform — they say nothing about whether yansi chose to paint.
    #[test]
    fn plain_is_ansi_with_the_escapes_removed() {
        let plain = human(Style::Plain);
        let coloured = human(Style::Ansi);

        assert!(
            !plain.contains('\u{1b}'),
            "Style::Plain must emit no escape codes: {plain:?}"
        );
        assert_eq!(
            plain,
            anstream::adapter::strip_str(&coloured).to_string(),
            "Plain must be exactly Ansi with the escapes removed, not different text"
        );
    }

    /// The other direction: `Ansi` must not strip. yansi decides whether to
    /// paint at all by probing the console, and off Windows that probe always
    /// says yes; on Windows it can say no, which would make this assertion
    /// about the host rather than about the code.
    #[cfg(not(windows))]
    #[test]
    fn ansi_keeps_the_escape_codes() {
        let coloured = human(Style::Ansi);
        assert!(
            coloured.contains('\u{1b}'),
            "Style::Ansi must emit escape codes: {coloured:?}"
        );
    }

    /// Pins the spellings the two typesetters expect. Guessing these wrong
    /// produces a document that fails to compile, which no unit test of the
    /// formatters themselves would catch.
    #[test]
    fn paper_sizes_map_to_both_typesetter_spellings() {
        let all = [
            (PaperSize::A4, "a4paper", "a4"),
            (PaperSize::Letter, "letterpaper", "us-letter"),
            (PaperSize::A5, "a5paper", "a5"),
            (PaperSize::Legal, "legalpaper", "us-legal"),
        ];
        for (size, latex, typst) in all {
            assert_eq!(size.latex_name(), latex, "latex name for {size:?}");
            assert_eq!(size.typst_name(), typst, "typst name for {size:?}");
        }
        assert_eq!(PaperSize::default(), PaperSize::A4);
    }

    #[test]
    fn style_default_is_plain_and_only_ansi_is_ansi() {
        assert_eq!(Style::default(), Style::Plain);
        assert!(Style::Ansi.is_ansi());
        assert!(!Style::Plain.is_ansi());
    }

    /// The `*_to_string` wrappers must return exactly what the `Write`-based
    /// primitives produce, not a re-rendering.
    #[test]
    fn to_string_wrappers_match_the_writer_primitives() {
        let recipe = parse_recipe(RECIPE, "Tea", 1.0).expect("parses").value;

        let mut buf = Vec::new();
        human::print_human(
            &recipe,
            "Tea",
            1.0,
            PARSER.converter(),
            Style::Plain,
            &mut buf,
        )
        .expect("formats");
        assert_eq!(
            human_to_string(&recipe, "Tea", 1.0, PARSER.converter(), Style::Plain).unwrap(),
            String::from_utf8(buf).unwrap()
        );

        let mut buf = Vec::new();
        markdown::print_md(&recipe, "Tea", 1.0, PARSER.converter(), &mut buf).expect("formats");
        assert_eq!(
            markdown_to_string(&recipe, "Tea", 1.0, PARSER.converter()).unwrap(),
            String::from_utf8(buf).unwrap()
        );
    }
}
