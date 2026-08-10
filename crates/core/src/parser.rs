//! Recipe parsing and conversion of `cooklang` reports into [`Diagnostic`]s.
//!
//! `cooklang` parses leniently, so a recipe can parse successfully and still
//! have something to say about itself. The CLI used to log those warnings and
//! drop them; here they come back to the caller in the [`Outcome`].

use crate::{CoreError, Diagnostic, Location, Outcome, Severity, Span};
use camino::Utf8Path;
use cooklang::{
    error::{SourceDiag, SourceReport},
    Converter, CooklangParser, Extensions, Recipe, RecipeResult,
};
use std::sync::LazyLock;

/// The shared parser. Matches CookCLI's configuration exactly: no extensions,
/// default converter for unit support.
pub static PARSER: LazyLock<CooklangParser> =
    LazyLock::new(|| CooklangParser::new(Extensions::empty(), Converter::default()));

/// Parse recipe text, scale it, and collect diagnostics.
///
/// `name` identifies the recipe in error messages. `scale` is the scaling
/// factor; pass `1.0` to leave quantities alone.
///
/// Returns [`CoreError::Parse`] when the recipe has errors, since no recipe can
/// be produced in that case. Warnings come back in the [`Outcome`].
pub fn parse_recipe(text: &str, name: &str, scale: f64) -> Result<Outcome<Recipe>, CoreError> {
    parse_recipe_at(text, name, scale, None)
}

/// As [`parse_recipe`], but attributing diagnostics to a specific file path.
///
/// Pass `file` when the text came from disk, so that diagnostics point at
/// something the caller can open. An editor parsing an unsaved buffer passes
/// `None` and still gets spans.
pub fn parse_recipe_at(
    text: &str,
    name: &str,
    scale: f64,
    file: Option<&Utf8Path>,
) -> Result<Outcome<Recipe>, CoreError> {
    let parsed = PARSER.parse(text);
    let diagnostics = collect_diagnostics(parsed.report(), file);

    if parsed.report().has_errors() {
        let display_path = file.map_or_else(|| name.to_string(), |p| p.to_string());
        return Err(CoreError::Parse {
            name: name.to_string(),
            diagnostics,
            rendered: render_report(&parsed, &display_path, text, false),
        });
    }

    let (mut recipe, _) = parsed
        .into_result()
        .expect("report has no errors, so a recipe is present");
    recipe.scale(scale, PARSER.converter());

    Ok(Outcome::with_diagnostics(recipe, diagnostics))
}

/// Render a parse report the way the CLI prints it, with source line context.
///
/// `ansi` controls colour. The CLI passes `true` for terminal output.
pub fn render_report(
    parsed: &RecipeResult,
    display_path: &str,
    content: &str,
    ansi: bool,
) -> String {
    let mut buf = Vec::new();
    parsed
        .report()
        .write(display_path, content, ansi, &mut buf)
        .ok();
    String::from_utf8_lossy(&buf).into_owned()
}

/// Convert every entry of a `cooklang` report into a [`Diagnostic`].
fn collect_diagnostics(report: &SourceReport, file: Option<&Utf8Path>) -> Vec<Diagnostic> {
    report
        .iter()
        .map(|diag| convert_diagnostic(diag, file))
        .collect()
}

fn convert_diagnostic(diag: &SourceDiag, file: Option<&Utf8Path>) -> Diagnostic {
    let severity = match diag.severity {
        cooklang::error::Severity::Error => Severity::Error,
        cooklang::error::Severity::Warning => Severity::Warning,
    };

    // `labels` is ordered most- to least-important, so the first one is the
    // main location. Diagnostics about the recipe as a whole have none.
    let span = diag.labels.first().map(|(span, _)| span.range().into());

    Diagnostic {
        severity,
        message: diag.message.to_string(),
        location: location_for(file, span),
    }
}

/// Build a [`Location`] from whichever of file and span are known.
///
/// A span alone is still worth reporting: an editor parsing an unsaved buffer
/// has no path but still wants to underline the offending text. Only when
/// neither is known is there no location at all.
fn location_for(file: Option<&Utf8Path>, span: Option<Span>) -> Option<Location> {
    if file.is_none() && span.is_none() {
        return None;
    }
    Some(Location {
        file: file.map(ToOwned::to_owned),
        span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Severity, Span};

    const GOOD: &str = "Boil @water{2%cups} for ~{5%minutes}.\n";

    #[test]
    fn parses_a_clean_recipe_without_diagnostics() {
        let outcome = parse_recipe(GOOD, "simple", 1.0).expect("parses");
        assert_eq!(outcome.value.ingredients.len(), 1);
        assert!(outcome.diagnostics.is_empty());
    }

    #[test]
    fn scaling_multiplies_quantities() {
        let single = parse_recipe(GOOD, "simple", 1.0).unwrap().into_value();
        let double = parse_recipe(GOOD, "simple", 2.0).unwrap().into_value();

        let one = format!("{:?}", single.ingredients[0].quantity);
        let two = format!("{:?}", double.ingredients[0].quantity);
        assert_ne!(one, two, "scaling should change the quantity");
    }

    #[test]
    fn warnings_are_returned_not_swallowed() {
        // Deprecated `>>` metadata parses successfully but warns.
        let text = ">> title: Old Style\n\nBoil @water{}.\n";
        let outcome = parse_recipe(text, "old", 1.0).expect("parses despite warning");
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Warning),
            "expected a warning diagnostic, got {:?}",
            outcome.diagnostics
        );
    }

    #[test]
    fn parse_errors_carry_diagnostics_and_rendered_output() {
        // An ingredient with a quantity but no name is a hard parse error.
        let text = "Add @{1%tsp} to the pot.\n";
        match parse_recipe(text, "broken", 1.0) {
            Err(CoreError::Parse {
                name,
                diagnostics,
                rendered,
            }) => {
                assert_eq!(name, "broken");
                assert!(!diagnostics.is_empty());
                assert!(!rendered.is_empty(), "rendered report should be populated");
            }
            other => panic!("expected CoreError::Parse, got {other:?}"),
        }
    }

    /// Offsets must be byte offsets, since that is what `Span` documents and
    /// what an editor slicing a `str` needs.
    #[test]
    fn spans_are_byte_offsets_into_the_source() {
        // `é` is two bytes but one char, so the empty ingredient name sits at
        // byte offset 8 and char offset 7. The two disagree, which is the
        // point of using this text.
        let text = "Sauté @{1%tsp} it.\n";
        assert_eq!(text.find('{'), Some(8));
        assert_eq!(text.chars().position(|c| c == '{'), Some(7));

        let Err(CoreError::Parse { diagnostics, .. }) = parse_recipe(text, "broken", 1.0) else {
            panic!("expected a parse error");
        };

        let span = diagnostics[0]
            .location
            .as_ref()
            .expect("location set")
            .span
            .expect("span set");
        assert!(
            text.get(span.start..span.end).is_some(),
            "span {span:?} does not fall on char boundaries of {text:?}"
        );
        // Byte offset, not the char offset 7.
        assert_eq!(span, Span { start: 8, end: 8 });
    }

    #[test]
    fn parse_recipe_at_attributes_diagnostics_to_the_file() {
        let text = ">> title: Old Style\n\nBoil @water{}.\n";
        let file = Utf8Path::new("recipes/old.cook");
        let outcome = parse_recipe_at(text, "old", 1.0, Some(file)).expect("parses");

        let location = outcome.diagnostics[0]
            .location
            .as_ref()
            .expect("location set");
        assert_eq!(location.file.as_deref(), Some(file));
        assert!(location.span.is_some(), "warning should carry a span");
    }

    #[test]
    fn location_is_built_from_whichever_parts_are_known() {
        let file = Utf8Path::new("soup.cook");
        let span = Span { start: 1, end: 4 };

        // Neither known: no location at all, rather than an empty one.
        assert_eq!(location_for(None, None), None);

        // A span with no file still locates the problem for an unsaved buffer.
        assert_eq!(
            location_for(None, Some(span)),
            Some(Location {
                file: None,
                span: Some(span)
            })
        );

        assert_eq!(
            location_for(Some(file), None),
            Some(Location {
                file: Some(file.to_owned()),
                span: None
            })
        );

        assert_eq!(
            location_for(Some(file), Some(span)),
            Some(Location {
                file: Some(file.to_owned()),
                span: Some(span)
            })
        );
    }

    #[test]
    fn render_report_includes_the_display_path_and_source_context() {
        let text = "Add @{1%tsp} to the pot.\n";
        let parsed = PARSER.parse(text);
        let rendered = render_report(&parsed, "recipes/broken.cook", text, false);

        assert!(
            rendered.contains("recipes/broken.cook"),
            "report should name the file: {rendered}"
        );
        assert!(
            rendered.contains("Add @{1%tsp} to the pot."),
            "report should quote the source line: {rendered}"
        );
    }
}
