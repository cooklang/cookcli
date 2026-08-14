// This file includes a substantial portion of code from
// https://github.com/Zheoni/cooklang-chef
//
// The original code is licensed under the MIT License, a copy of which
// is provided below in addition to our project's license.
//
//

// MIT License

// Copyright (c) 2023 Francisco J. Sanchez

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Format a recipe as cooklang

use std::{fmt::Write, io};

use cooklang::{
    metadata::Metadata,
    model::{Item, Section, Step},
    parser::Modifiers,
    quantity::Quantity,
    Recipe,
};
use regex::Regex;

/// Write `recipe` back out as Cooklang source.
///
/// Metadata is emitted as YAML front-matter; steps are re-wrapped to the
/// terminal width without breaking a component across lines.
pub fn print_cooklang(recipe: &Recipe, mut writer: impl io::Write) -> io::Result<()> {
    let w = &mut writer;

    metadata(w, &recipe.metadata)?;
    writeln!(w)?;
    sections(w, recipe)?;

    Ok(())
}

fn metadata(w: &mut impl io::Write, metadata: &Metadata) -> io::Result<()> {
    // TODO if the recipe has been scaled and multiple servings are defined
    // it can lead to the recipe not parsing.
    if metadata.map.is_empty() {
        return Ok(());
    }

    let map = metadata.map.clone();

    const FRONTMATTER_FENCE: &str = "---";
    writeln!(w, "{FRONTMATTER_FENCE}")?;
    serde_yaml::to_writer(&mut *w, &map)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(w, "{FRONTMATTER_FENCE}\n")?;
    Ok(())
}

fn sections(w: &mut impl io::Write, recipe: &Recipe) -> io::Result<()> {
    for (index, section) in recipe.sections.iter().enumerate() {
        w_section(w, section, recipe, index)?;
    }
    Ok(())
}

fn w_section(
    w: &mut impl io::Write,
    section: &Section,
    recipe: &Recipe,
    index: usize,
) -> io::Result<()> {
    if let Some(name) = &section.name {
        writeln!(w, "== {name} ==")?;
    } else if index > 0 {
        writeln!(w, "====")?;
    }
    for content in &section.content {
        match content {
            cooklang::Content::Step(step) => w_step(w, step, recipe)?,
            cooklang::Content::Text(text) => w_text_block(w, text)?,
        }
        writeln!(w)?;
    }
    Ok(())
}

fn w_step(w: &mut impl io::Write, step: &Step, recipe: &Recipe) -> io::Result<()> {
    let mut step_str = String::new();
    for item in &step.items {
        match item {
            Item::Text { value } => step_str.push_str(value),
            &Item::Ingredient { index } => {
                let igr = &recipe.ingredients[index];

                let name = if let Some(reference) = &igr.reference {
                    // `path` already carries the leading `.` — a reference's
                    // components always begin with one, because `@./sauce{}`
                    // parses as components `["."]`. Prepending another `./`
                    // here, as this used to, made the formatter emit
                    // `@././sub/sauce{}`; that reparses as components
                    // `[".", ".", "sub"]`, so the next pass prepended one more
                    // and the prefix grew without bound on every rewrite.
                    reference.path(crate::REFERENCE_SEPARATOR)
                } else {
                    igr.name.clone()
                };

                ComponentFormatter {
                    kind: ComponentKind::Ingredient,
                    modifiers: igr.modifiers(),
                    name: Some(&name),
                    alias: igr.alias.as_deref(),
                    quantity: igr.quantity.as_ref(),
                    note: igr.note.as_deref(),
                }
                .format(&mut step_str)
            }
            &Item::Cookware { index } => {
                let cw = &recipe.cookware[index];
                ComponentFormatter {
                    kind: ComponentKind::Cookware,
                    modifiers: cw.modifiers(),
                    name: Some(&cw.name),
                    alias: cw.alias.as_deref(),
                    quantity: cw.quantity.as_ref(),
                    note: None,
                }
                .format(&mut step_str)
            }
            &Item::Timer { index } => {
                let t = &recipe.timers[index];
                ComponentFormatter {
                    kind: ComponentKind::Timer,
                    modifiers: Modifiers::empty(),
                    name: t.name.as_deref(),
                    alias: None,
                    quantity: t.quantity.as_ref(),
                    note: None,
                }
                .format(&mut step_str)
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                write!(&mut step_str, "{}", q.value()).expect("writing to a String is infallible");
                if let Some(u) = q.unit() {
                    step_str.push_str(u);
                }
            }
        }
    }
    let width = textwrap::termwidth().min(80);
    let options = textwrap::Options::new(width)
        .word_separator(textwrap::WordSeparator::Custom(component_word_separator));
    let lines = textwrap::wrap(step_str.trim(), options);
    for line in lines {
        writeln!(w, "{line}")?;
    }
    Ok(())
}

fn w_text_block(w: &mut impl io::Write, text: &str) -> io::Result<()> {
    let width = textwrap::termwidth().min(80);
    let indent = "> ";
    let options = textwrap::Options::new(width)
        .initial_indent(indent)
        .subsequent_indent(indent);
    let lines = textwrap::wrap(text.trim(), options);
    for line in lines {
        writeln!(w, "{line}")?;
    }
    Ok(())
}

// This prevents spliting a multi word component in two lines, because that's
// invalid.
fn component_word_separator<'a>(
    line: &'a str,
) -> Box<dyn Iterator<Item = textwrap::core::Word<'a>> + 'a> {
    use textwrap::core::Word;

    let re = {
        static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new(r"[@#~][^@#~]*\{[^\}]*\}").unwrap())
    };

    let mut words = vec![];
    let mut last_added = 0;
    let default_separator = textwrap::WordSeparator::new();

    for component in re.find_iter(line) {
        if last_added < component.start() {
            words.extend(default_separator.find_words(&line[last_added..component.start()]));
        }

        // Take the whitespace that follows the component along with it.
        //
        // A `textwrap::Word` carries the whitespace that *trails* it, and that
        // trailing whitespace is what gets dropped when the line breaks there.
        // Emitting the component on its own left it with none, so the space
        // after it fell to the following word instead — as an empty word whose
        // whitespace is that space — and when the break landed at exactly that
        // point the space moved to the start of the wrapped line. Reparsed, a
        // leading space is ordinary step text, so the next format pass added
        // another, and another: the formatter was not idempotent, and
        // rewriting a collection through it corrupted every step that happened
        // to wrap just after a component
        // (<https://github.com/cooklang/cookcli/issues/414>).
        //
        // `Word::from` splits trailing whitespace off into the `whitespace`
        // field for us, so extending the slice is the whole fix.
        let trailing = line[component.end()..]
            .find(|c: char| !c.is_whitespace())
            .map_or(line.len(), |offset| component.end() + offset);
        words.push(Word::from(&line[component.start()..trailing]));
        last_added = trailing;
    }
    if last_added < line.len() {
        words.extend(default_separator.find_words(&line[last_added..]));
    }
    Box::new(words.into_iter())
}

struct ComponentFormatter<'a> {
    kind: ComponentKind,
    modifiers: Modifiers,
    name: Option<&'a str>,
    alias: Option<&'a str>,
    quantity: Option<&'a Quantity>,
    note: Option<&'a str>,
}

enum ComponentKind {
    Ingredient,
    Cookware,
    Timer,
}

impl ComponentFormatter<'_> {
    fn format(self, w: &mut String) {
        w.push(match self.kind {
            ComponentKind::Ingredient => '@',
            ComponentKind::Cookware => '#',
            ComponentKind::Timer => '~',
        });
        for m in self.modifiers {
            w.push(match m {
                Modifiers::RECIPE => '@',
                Modifiers::HIDDEN => '-',
                Modifiers::OPT => '?',
                Modifiers::REF => '&',
                Modifiers::NEW => '+',
                _ => panic!("Unknown modifier: {m:?}"),
            });
        }
        let mut multi_word = false;
        if let Some(name) = self.name {
            if name.chars().any(|c| !c.is_alphanumeric()) {
                multi_word = true;
            }
            w.push_str(name);
            if let Some(alias) = self.alias {
                multi_word = true;
                w.push('|');
                w.push_str(alias);
            }
        }
        if let Some(q) = self.quantity {
            w.push('{');
            w.push_str(&q.value().to_string());
            if let Some(unit) = q.unit() {
                write!(w, "%{unit}").unwrap();
            }
            w.push('}');
        } else if multi_word {
            w.push_str("{}");
        }
        if let Some(note) = self.note {
            write!(w, "({note})").unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` already brings `std::fmt::Write` into scope for `write!`.
    use crate::test_support::parse_recipe;

    /// Exercises the things this formatter can get wrong: multi-word names
    /// (which need `{}` to stay one component), a note, an aliased name, an
    /// ingredient with no quantity, cookware with and without an amount,
    /// named and unnamed timers, several steps, a text block, and both an
    /// unnamed and a named section.
    ///
    /// The long step is deliberate: it must wrap, and it places a multi-word
    /// component near the wrap point, which is exactly what
    /// [`component_word_separator`] exists to protect. Splitting
    /// `@caster sugar{2%tbsp}` across two lines produces text that no longer
    /// parses as one ingredient.
    const FIXTURE: &str = "\
---
title: Round Trip
servings: 4
---

Mix @plain flour{200%g} with @whole milk{250%ml} in a #large mixing bowl{} \
and beat until the batter is completely smooth, then fold in @caster \
sugar{2%tbsp} and a pinch of @fine sea salt{}.

Simmer ~gently{10%minutes} in a #pan{2}, then rest ~{5%minutes}.

> Rest the dough somewhere warm, covered.

== Finishing ==

Dust with @icing sugar{1%tbsp}(sifted) using a #fine sieve.
";

    /// A structural summary of everything this formatter has to preserve.
    /// Comparing summaries rather than `Recipe` values keeps the failure
    /// message readable and ignores spans, which legitimately move.
    fn shape(recipe: &Recipe) -> String {
        let mut s = String::new();
        for i in &recipe.ingredients {
            writeln!(
                s,
                "ingredient name={:?} alias={:?} qty={:?} note={:?} modifiers={:?}",
                i.name,
                i.alias,
                i.quantity.as_ref().map(ToString::to_string),
                i.note,
                i.modifiers()
            )
            .unwrap();
        }
        for c in &recipe.cookware {
            writeln!(
                s,
                "cookware name={:?} qty={:?} note={:?}",
                c.name,
                c.quantity.as_ref().map(ToString::to_string),
                c.note
            )
            .unwrap();
        }
        for t in &recipe.timers {
            writeln!(
                s,
                "timer name={:?} qty={:?}",
                t.name,
                t.quantity.as_ref().map(ToString::to_string)
            )
            .unwrap();
        }
        for section in &recipe.sections {
            writeln!(s, "section name={:?}", section.name).unwrap();
            for content in &section.content {
                match content {
                    cooklang::Content::Step(step) => writeln!(
                        s,
                        "  step {} text={:?}",
                        step.number,
                        step_text(recipe, step)
                    )
                    .unwrap(),
                    cooklang::Content::Text(t) => writeln!(s, "  text {:?}", t.trim()).unwrap(),
                }
            }
        }
        s
    }

    /// The step as a reader sees it, with components replaced by their names.
    /// Whitespace is collapsed because the formatter re-wraps steps, so line
    /// breaks legitimately land in different places.
    fn step_text(recipe: &Recipe, step: &Step) -> String {
        let mut s = String::new();
        for item in &step.items {
            match item {
                Item::Text { value } => s.push_str(value),
                &Item::Ingredient { index } => {
                    s.push_str(recipe.ingredients[index].display_name().as_ref())
                }
                &Item::Cookware { index } => s.push_str(&recipe.cookware[index].name),
                &Item::Timer { index } => {
                    let t = &recipe.timers[index];
                    if let Some(name) = &t.name {
                        s.push_str(name);
                    }
                    if let Some(q) = &t.quantity {
                        write!(s, "{q}").unwrap();
                    }
                }
                &Item::InlineQuantity { index } => {
                    write!(s, "{}", recipe.inline_quantities[index]).unwrap()
                }
            }
        }
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn format_to_string(recipe: &Recipe) -> String {
        let mut buf = Vec::new();
        print_cooklang(recipe, &mut buf).expect("formats");
        String::from_utf8(buf).expect("utf-8")
    }

    /// The formatter's whole contract: what it writes must parse back into
    /// the same recipe. This catches escaping, `{}` placement and wrapping
    /// regressions in one assertion.
    #[test]
    fn output_reparses_into_an_equivalent_recipe() {
        let original = parse_recipe(FIXTURE, "round trip", 1.0).expect("fixture parses");
        assert!(
            original.diagnostics.is_empty(),
            "fixture should parse cleanly: {:?}",
            original.diagnostics
        );

        let rendered = format_to_string(&original.value);
        let reparsed = parse_recipe(&rendered, "round trip", 1.0)
            .unwrap_or_else(|e| panic!("formatter emitted unparseable cooklang:\n{rendered}\n{e}"));
        assert!(
            reparsed.diagnostics.is_empty(),
            "reparse warned: {:?}\n--- rendered ---\n{rendered}",
            reparsed.diagnostics
        );

        assert_eq!(
            shape(&original.value),
            shape(&reparsed.value),
            "round trip changed the recipe\n--- rendered ---\n{rendered}"
        );
    }

    /// Formatting is idempotent, so that rewriting a stored `.cook` file does
    /// not churn it.
    ///
    /// It was not: a step whose wrap fell just after a component came out with
    /// a leading space, that space reparsed as ordinary step text, and the next
    /// pass added another — one space after pass 1, two after pass 2, three
    /// after pass 3, without bound. Any workflow that rewrites recipes through
    /// the formatter — normalising a collection, an editor's "format document",
    /// a pre-commit hook — degraded the source a little more on each run
    /// (<https://github.com/cooklang/cookcli/issues/414>).
    ///
    /// `FIXTURE`'s long step is built to reach this: it wraps, and it puts a
    /// multi-word component near the wrap point.
    #[test]
    fn formatting_is_idempotent() {
        let once = format_to_string(&parse_recipe(FIXTURE, "r", 1.0).unwrap().value);
        let twice = format_to_string(&parse_recipe(&once, "r", 1.0).unwrap().value);
        assert_eq!(once, twice, "second format pass differed");
    }

    /// The defect this guards grew by one space per pass, so two passes is the
    /// smallest case that shows it and proves nothing about the fourth. Run it
    /// out far enough that an accumulating change cannot hide.
    #[test]
    fn formatting_is_stable_over_many_passes() {
        let first = format_to_string(&parse_recipe(FIXTURE, "r", 1.0).unwrap().value);
        let mut current = first.clone();
        for pass in 2..=6 {
            current = format_to_string(&parse_recipe(&current, "r", 1.0).unwrap().value);
            assert_eq!(first, current, "pass {pass} differed from the first");
        }
    }

    /// The specific shape that broke: a step that wraps immediately after a
    /// component must not start the next line with a space.
    #[test]
    fn a_wrap_just_after_a_component_leaves_no_leading_space() {
        let rendered = format_to_string(&parse_recipe(FIXTURE, "r", 1.0).unwrap().value);
        assert!(
            rendered.lines().count() > 1,
            "fixture must wrap for this to mean anything: {rendered}"
        );
        for line in rendered.lines() {
            assert!(
                !line.starts_with(' '),
                "step lines must not be indented: {line:?}\n--- rendered ---\n{rendered}"
            );
        }
    }

    /// A recipe reference is re-emitted with `/` between its components on
    /// every platform.
    ///
    /// This writer produces Cooklang *source*, so the separator is syntax, not
    /// presentation: joining with `std::path::MAIN_SEPARATOR` wrote
    /// `@.\sub\sauce{}` on Windows, which does not parse back as the same
    /// reference (<https://github.com/cooklang/cookcli/issues/442>). The
    /// assertion holds trivially on Unix, where the platform separator was
    /// already `/`; it is the Windows leg of the CI matrix that it guards.
    #[test]
    fn a_reference_is_written_with_forward_slashes() {
        let source = "Make @./sub/sauce{200%ml} and stir.\n";
        let recipe = parse_recipe(source, "r", 1.0)
            .expect("fixture parses")
            .value;
        let rendered = format_to_string(&recipe);

        assert!(
            rendered.contains("@./sub/sauce{"),
            "expected a forward-slash reference, got: {rendered}"
        );
        assert!(
            !rendered.contains('\\'),
            "a reference path must never carry a backslash: {rendered}"
        );

        // And it still means the same thing when read back.
        let reparsed = parse_recipe(&rendered, "r", 1.0).expect("re-parses").value;
        let reference = reparsed.ingredients[0]
            .reference
            .as_ref()
            .expect("still a recipe reference");
        assert_eq!(reference.path(crate::REFERENCE_SEPARATOR), "./sub/sauce");
    }

    /// A reference survives repeated rewrites unchanged.
    ///
    /// It did not: a reference's components already begin with `.`, and the
    /// writer prepended a second `./` on top, so each pass added one more —
    /// `@./sub/sauce{}`, `@././sub/sauce{}`, `@./././sub/sauce{}`. Every pass
    /// still resolved to the same file, so nothing failed; the source just got
    /// steadily uglier each time a collection was reformatted.
    ///
    /// Unlike [`formatting_is_idempotent`], which covers the wrapping defect,
    /// this holds for every pass and is not ignored.
    #[test]
    fn a_reference_does_not_accumulate_a_prefix_across_passes() {
        let mut source = "Make @./sub/sauce{200%ml} and stir.\n".to_string();
        for pass in 1..=4 {
            let recipe = parse_recipe(&source, "r", 1.0).expect("parses").value;
            let rendered = format_to_string(&recipe);
            assert!(
                rendered.contains("@./sub/sauce{"),
                "pass {pass} changed the reference: {rendered}"
            );
            source = rendered;
        }
    }

    /// Metadata survives as YAML front-matter, not as the deprecated `>>`
    /// syntax that would warn on reparse.
    #[test]
    fn metadata_round_trips_as_front_matter() {
        let rendered = format_to_string(&parse_recipe(FIXTURE, "r", 1.0).unwrap().value);
        assert!(
            rendered.starts_with("---\n"),
            "expected YAML front-matter, got: {rendered}"
        );
        assert!(!rendered.contains(">> "), "must not emit `>>` metadata");

        let reparsed = parse_recipe(&rendered, "r", 1.0).unwrap().value;
        assert_eq!(
            reparsed.metadata.get("title").and_then(|v| v.as_str()),
            Some("Round Trip")
        );
        assert_eq!(
            reparsed.metadata.get("servings").and_then(|v| v.as_u64()),
            Some(4)
        );
    }

    /// A recipe with no metadata must not emit an empty front-matter block,
    /// which would reparse as a stray text step.
    #[test]
    fn a_recipe_without_metadata_emits_no_front_matter() {
        let rendered =
            format_to_string(&parse_recipe("Boil @water{1%l}.\n", "r", 1.0).unwrap().value);
        assert!(
            !rendered.contains("---"),
            "expected no front-matter fence: {rendered:?}"
        );
        assert_eq!(rendered.trim(), "Boil @water{1%l}.");
    }
}
