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

pub mod cooklang;
pub mod human;
pub mod latex;
pub mod markdown;
pub use cooklang_format::{number, quantity, schema};
pub mod shopping_list;
pub mod typst;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_recipe, PARSER};

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
