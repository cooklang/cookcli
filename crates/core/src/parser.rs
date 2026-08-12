//! Recipe parsing and conversion of `cooklang` reports into [`Diagnostic`]s.
//!
//! `cooklang` parses leniently, so a recipe can parse successfully and still
//! have something to say about itself. The CLI used to log those warnings and
//! drop them; here they come back to the caller in the [`Outcome`].

use crate::{CoreError, Diagnostic, Location, Outcome, Severity, Span};
use camino::Utf8Path;
use cooklang::{
    error::{SourceDiag, SourceReport},
    Converter, CooklangParser, Extensions, Recipe,
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
///
/// # Scale
///
/// `scale` must be finite; NaN and infinity give [`CoreError::InvalidScale`].
/// Zero and negative factors are *accepted*, matching what the CLI does today.
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
    if !scale.is_finite() {
        return Err(CoreError::InvalidScale { scale });
    }

    let mut outcome = parse_unscaled(text, name, file)?;
    outcome.value.scale(scale, PARSER.converter());
    Ok(outcome)
}

/// Parse recipe text and collect diagnostics, without scaling it.
///
/// Deliberately separate from `parse_recipe_at(.., 1.0, ..)`:
/// [`cooklang::Recipe::scale`] re-fits units even at a factor of one, so
/// `1500 ml` comes back as `1.5 l`. "Scale by one" and "do not scale" are
/// therefore different operations, and shopping-list reference expansion needs
/// the latter — it applies its own `scale_to_target` afterwards, and a fit in
/// between would change the numbers the user sees.
pub(crate) fn parse_unscaled(
    text: &str,
    name: &str,
    file: Option<&Utf8Path>,
) -> Result<Outcome<Recipe>, CoreError> {
    let parsed = PARSER.parse(text);
    let display_path = file.map_or_else(|| name.to_string(), |p| p.to_string());
    let parse_error = |report: &SourceReport| CoreError::Parse {
        name: name.to_string(),
        diagnostics: collect_diagnostics(report, file),
        rendered: render_report(report, &display_path, text, false),
    };

    if parsed.report().has_errors() {
        return Err(parse_error(parsed.report()));
    }
    let diagnostics = collect_diagnostics(parsed.report(), file);

    match parsed.into_result() {
        Ok((recipe, _)) => Ok(Outcome::with_diagnostics(recipe, diagnostics)),
        // `into_result` fails when `is_valid()` is false, which is
        // `has_output() && !has_errors()`. We have just ruled out errors, but
        // cooklang does not promise output is present, so this arm is
        // reachable in principle. Returning beats panicking: this crate is
        // called from a NAPI addon, where a panic crosses into JavaScript.
        Err(report) => Err(parse_error(&report)),
    }
}

/// Render a parse report the way the CLI prints it, with source line context.
///
/// `ansi` controls colour. The CLI passes `true` for terminal output; the
/// report stored in [`CoreError::Parse`] is always rendered with `false`.
///
/// Takes the report rather than the parse result so that it also renders
/// reports from metadata-only parses.
pub fn render_report(
    report: &SourceReport,
    display_path: &str,
    content: &str,
    ansi: bool,
) -> String {
    let mut buf = Vec::new();
    report.write(display_path, content, ansi, &mut buf).ok();
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
        // Often a ready-to-apply replacement, which is exactly the payload
        // the CLI's `warn!` used to discard.
        hints: diag.hints.iter().map(|h| h.to_string()).collect(),
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
    use cooklang::quantity::Value;

    const GOOD: &str = "Boil @water{2%cups} for ~{5%minutes}.\n";

    /// The numeric value of an ingredient's quantity, ignoring its unit.
    ///
    /// Asserting on the value rather than the formatted quantity matters:
    /// cooklang re-fits units when scaling, so `2 cups` can render as `4 c`
    /// and a string comparison would be testing the formatter, not the maths.
    fn quantity_value(recipe: &Recipe, index: usize) -> f64 {
        match recipe.ingredients[index]
            .quantity
            .as_ref()
            .expect("ingredient has a quantity")
            .value()
        {
            Value::Number(n) => n.value(),
            other => panic!("expected a numeric quantity, got {other:?}"),
        }
    }

    #[test]
    fn parses_a_clean_recipe_without_diagnostics() {
        let outcome = parse_recipe(GOOD, "simple", 1.0).expect("parses");
        assert_eq!(outcome.value.ingredients.len(), 1);
        assert!(outcome.diagnostics.is_empty());
    }

    #[test]
    fn scaling_multiplies_quantities_by_exactly_the_factor() {
        // GOOD declares `@water{2%cups}`.
        assert_eq!(
            quantity_value(&parse_recipe(GOOD, "s", 1.0).unwrap().value, 0),
            2.0
        );
        assert_eq!(
            quantity_value(&parse_recipe(GOOD, "s", 2.0).unwrap().value, 0),
            4.0
        );
        assert_eq!(
            quantity_value(&parse_recipe(GOOD, "s", 0.5).unwrap().value, 0),
            1.0
        );
    }

    /// The reason `parse_unscaled` exists: scaling by one is not a no-op,
    /// because it re-fits units. Anything that must not disturb the authored
    /// quantities has to skip the scale call, not pass `1.0`.
    #[test]
    fn scaling_by_one_refits_units_but_not_scaling_leaves_them_alone() {
        let text = "Pour @milk{1500%ml}.\n";

        let scaled = parse_recipe(text, "milk", 1.0).expect("parses").value;
        let quantity = scaled.ingredients[0].quantity.as_ref().unwrap();
        assert_eq!(
            (quantity.value().to_string(), quantity.unit()),
            ("1.5".to_string(), Some("l")),
            "scale(1.0) refits 1500 ml to 1.5 l"
        );

        let untouched = parse_unscaled(text, "milk", None).expect("parses").value;
        let quantity = untouched.ingredients[0].quantity.as_ref().unwrap();
        assert_eq!(
            (quantity.value().to_string(), quantity.unit()),
            ("1500".to_string(), Some("ml")),
            "parse_unscaled must leave the authored quantity alone"
        );
    }

    #[test]
    fn non_finite_scale_is_rejected() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            match parse_recipe(GOOD, "simple", bad) {
                Err(CoreError::InvalidScale { scale }) => {
                    assert_eq!(scale.is_nan(), bad.is_nan());
                }
                other => panic!("expected InvalidScale for {bad}, got {other:?}"),
            }
        }
    }

    /// Zero and negative scale are accepted, because the CLI accepts them.
    /// Rejecting them would be a behaviour change smuggled into a refactor.
    #[test]
    fn zero_and_negative_scale_are_accepted() {
        assert_eq!(
            quantity_value(&parse_recipe(GOOD, "s", 0.0).unwrap().value, 0),
            0.0
        );
        assert_eq!(
            quantity_value(&parse_recipe(GOOD, "s", -1.0).unwrap().value, 0),
            -2.0
        );
    }

    #[test]
    fn warnings_are_returned_not_swallowed() {
        // Deprecated `>>` metadata parses successfully but warns.
        let text = ">> title: Old Style\n\nBoil @water{}.\n";
        let outcome = parse_recipe(text, "old", 1.0).expect("parses despite warning");

        // Every diagnostic must be a Warning — not merely "at least one is",
        // which would still hold if errors were mislabelled as warnings.
        assert!(!outcome.diagnostics.is_empty(), "expected a diagnostic");
        for d in &outcome.diagnostics {
            assert_eq!(
                d.severity,
                Severity::Warning,
                "deprecated syntax is a warning, got {d:?}"
            );
        }
        assert!(!outcome.has_errors());
    }

    /// Pins the severity mapping in both directions at once, so that inverting
    /// it cannot pass. Also pins that *every* diagnostic is converted, not
    /// just the first.
    #[test]
    fn every_error_is_converted_with_error_severity() {
        // Two empty ingredient names: two errors, at two distinct spans.
        let text = "Add @{1%tsp} and @{2%tsp} to the pot.\n";
        let Err(CoreError::Parse { diagnostics, .. }) = parse_recipe(text, "broken", 1.0) else {
            panic!("expected a parse error");
        };

        assert_eq!(
            diagnostics.len(),
            2,
            "both errors must survive: {diagnostics:?}"
        );
        for d in &diagnostics {
            assert_eq!(
                d.severity,
                Severity::Error,
                "cooklang error must map to Error"
            );
        }

        let spans: Vec<_> = diagnostics
            .iter()
            .map(|d| d.location.as_ref().unwrap().span.unwrap())
            .collect();
        assert_eq!(
            spans,
            vec![Span { start: 5, end: 5 }, Span { start: 18, end: 18 }],
            "each diagnostic keeps its own span, in source order"
        );
    }

    /// `labels` is ordered most- to least-important, so the *first* is the
    /// primary location. Taking the last would underline the wrong text.
    #[test]
    fn the_first_label_wins_when_a_diagnostic_has_several() {
        let text = ">> title: A\n>> title: B\n\nBoil @water{}.\n";
        let outcome = parse_recipe(text, "dup", 1.0).expect("parses");

        let duplicate = outcome
            .diagnostics
            .iter()
            .find(|d| d.message.contains("duplicate") || d.message.contains("Duplicate"))
            .unwrap_or(&outcome.diagnostics[0]);

        assert_eq!(
            duplicate.location.as_ref().unwrap().span,
            Some(Span { start: 2, end: 11 }),
            "expected the first label's span, not a later one: {duplicate:?}"
        );
    }

    /// Hints are quick fixes, and the CLI's `warn!` threw them away.
    #[test]
    fn hints_are_captured() {
        let text = ">> title: A\n>> title: B\n\nBoil @water{}.\n";
        let outcome = parse_recipe(text, "dup", 1.0).expect("parses");

        let hints: Vec<&String> = outcome.diagnostics.iter().flat_map(|d| &d.hints).collect();
        assert!(
            !hints.is_empty(),
            "expected at least one hint, got {:?}",
            outcome.diagnostics
        );
        assert!(
            hints.iter().any(|h| h.contains("---")),
            "expected a ready-to-apply frontmatter fix, got {hints:?}"
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
                assert_eq!(diagnostics.len(), 1);
                assert_eq!(diagnostics[0].severity, Severity::Error);
                assert!(!rendered.is_empty(), "rendered report should be populated");
                // The stored report is documented as ANSI-free.
                assert!(
                    !rendered.contains('\u{1b}'),
                    "CoreError::Parse.rendered must carry no escape codes: {rendered:?}"
                );
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
        let rendered = render_report(parsed.report(), "recipes/broken.cook", text, false);

        assert!(
            rendered.contains("recipes/broken.cook"),
            "report should name the file: {rendered}"
        );
        assert!(
            rendered.contains("Add @{1%tsp} to the pot."),
            "report should quote the source line: {rendered}"
        );
    }

    #[test]
    fn render_report_honours_the_ansi_flag() {
        let text = "Add @{1%tsp} to the pot.\n";
        let parsed = PARSER.parse(text);

        let plain = render_report(parsed.report(), "broken.cook", text, false);
        let coloured = render_report(parsed.report(), "broken.cook", text, true);

        assert!(
            !plain.contains('\u{1b}'),
            "ansi=false must produce no escape codes: {plain:?}"
        );
        assert!(
            coloured.contains('\u{1b}'),
            "ansi=true must produce escape codes: {coloured:?}"
        );
    }

    /// `render_report` takes a report, not a parse result, so it also serves
    /// metadata-only parses.
    #[test]
    fn render_report_works_for_a_metadata_parse() {
        let text = ">> title: Old Style\n\nBoil @water{}.\n";
        let parsed = PARSER.parse_metadata(text);
        let rendered = render_report(parsed.report(), "old.cook", text, false);
        assert!(
            rendered.contains("old.cook"),
            "metadata report should render: {rendered}"
        );
    }
}
