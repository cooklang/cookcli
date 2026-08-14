//! Golden-output tests for the document formats: LaTeX, Typst and Cooklang.
//!
//! # Why this file exists
//!
//! These three had no test that looked at the document *body*. The five
//! existing LaTeX and Typst tests assert only that the paper-size and margin
//! substrings appear in the preamble, and the Cooklang one checks that the
//! output contains `@water` and a `#`. A regression that broke `\begin`/`\end`
//! pairing, emitted a malformed preamble, mangled ingredient markup or dropped
//! a whole section would have passed every one of them
//! (<https://github.com/cooklang/cookcli/issues/416>).
//!
//! For comparison, `-f jsonld` has 18 tests parsing the JSON and asserting
//! fields, and human, Markdown, JSON and YAML have insta snapshots pinning
//! exact output.
//!
//! # What is pinned
//!
//! One snapshot per format over [`FIXTURE`], which is built to exercise
//! everything a writer has to handle: front matter, multi-word ingredient
//! names, an ingredient with no quantity, cookware with and without an amount,
//! named and unnamed timers, a text block, a recipe reference, and both an
//! unnamed and a named section.
//!
//! Snapshots pin exact output, so they are noisy by design: any change to these
//! formats shows up as a diff to review rather than as silence. That is the
//! point — reviewing an intended change is cheap, and it is the unintended ones
//! this is here to catch.
//!
//! The structural assertions after each snapshot are the part that must hold
//! whatever the formatting: a snapshot can be re-recorded wrong, and
//! `\begin{document}` without `\end{document}` is not a matter of taste.

use assert_cmd::Command;
use insta::assert_snapshot;
use std::fs;
use tempfile::TempDir;

/// A recipe exercising every construct these writers have to render.
const FIXTURE: &str = "\
---
title: Test Kitchen
servings: 4
tags:
  - dinner
---

Mix @plain flour{200%g} with @whole milk{250%ml} in a #large mixing bowl{} and \
beat until smooth, then add @fine sea salt{}.

Simmer ~gently{10%minutes} in a #pan{2}, then rest ~{5%minutes}.

> Rest the dough somewhere warm, covered.

== Finishing ==

Serve with @./sauce{} and dust with @icing sugar{1%tbsp}(sifted).
";

/// A directory holding [`FIXTURE`] and the recipe it references.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("dish.cook"), FIXTURE).unwrap();
    fs::write(dir.path().join("sauce.cook"), "Simmer @tomatoes{4}.\n").unwrap();
    dir
}

/// `cook recipe read -f <format> dish.cook`, as text.
fn render(format: &str) -> String {
    let dir = fixture();
    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(dir.path())
        .args(["recipe", "read", "-f", format, "dish.cook"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "-f {format} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8")
}

// ---------------------------------------------------------------------------
// LaTeX
// ---------------------------------------------------------------------------

#[test]
fn latex_output() {
    assert_snapshot!(render("latex"));
}

/// The parts of a LaTeX document that are wrong rather than merely different:
/// an unbalanced environment, a missing preamble, an unclosed brace.
#[test]
fn latex_output_is_a_well_formed_document() {
    let out = render("latex");

    assert!(out.starts_with("\\documentclass"), "{out}");
    assert_eq!(
        out.matches("\\begin{document}").count(),
        1,
        "exactly one document environment: {out}"
    );
    assert_eq!(
        out.matches("\\begin{").count(),
        out.matches("\\end{").count(),
        "every environment must be closed: {out}"
    );
    assert_eq!(
        out.matches('{').count(),
        out.matches('}').count(),
        "braces must balance: {out}"
    );
    assert!(
        out.trim_end().ends_with("\\end{document}"),
        "the document must be closed last: {out}"
    );

    // And the body is actually there, not just the preamble.
    assert!(out.contains("Test Kitchen"), "the title: {out}");
    assert!(out.contains("plain flour"), "an ingredient: {out}");
    assert!(out.contains("large mixing bowl"), "cookware: {out}");
    assert!(out.contains("Finishing"), "the named section: {out}");
    assert!(out.contains("Rest the dough"), "the text block: {out}");
}

// ---------------------------------------------------------------------------
// Typst
// ---------------------------------------------------------------------------

#[test]
fn typst_output() {
    assert_snapshot!(render("typst"));
}

#[test]
fn typst_output_is_a_well_formed_document() {
    let out = render("typst");

    assert!(out.contains("#set page("), "the page setup: {out}");
    assert_eq!(
        out.matches('[').count(),
        out.matches(']').count(),
        "content blocks must balance: {out}"
    );
    assert_eq!(
        out.matches('{').count(),
        out.matches('}').count(),
        "braces must balance: {out}"
    );

    assert!(out.contains("Test Kitchen"), "the title: {out}");
    assert!(out.contains("plain flour"), "an ingredient: {out}");
    assert!(out.contains("large mixing bowl"), "cookware: {out}");
    assert!(out.contains("Finishing"), "the named section: {out}");
    assert!(out.contains("Rest the dough"), "the text block: {out}");
}

// ---------------------------------------------------------------------------
// Cooklang
// ---------------------------------------------------------------------------

#[test]
fn cooklang_output() {
    assert_snapshot!(render("cooklang"));
}

/// The Cooklang writer's contract is stronger than the others': what it emits
/// is *source*, so it has to parse back into the same recipe. Pinned at the CLI
/// level as well as in the crate, because the wrap width is taken from the
/// terminal and only this path exercises the real one.
#[test]
fn cooklang_output_parses_back_to_the_same_recipe() {
    let out = render("cooklang");
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("round-trip.cook"), &out).unwrap();
    fs::write(dir.path().join("sauce.cook"), "Simmer @tomatoes{4}.\n").unwrap();

    let again = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(dir.path())
        .args(["recipe", "read", "-f", "cooklang", "round-trip.cook"])
        .output()
        .unwrap();

    assert!(
        again.status.success(),
        "re-reading the formatter's own output failed: {}",
        String::from_utf8_lossy(&again.stderr)
    );
    assert_eq!(
        String::from_utf8(again.stdout).expect("utf-8"),
        out,
        "formatting must be idempotent (#414)"
    );
}

/// No step line may begin with a space. This is the shape #414 broke: a wrap
/// falling just after a component left one behind, and it grew by one on every
/// pass.
#[test]
fn cooklang_output_has_no_leading_space_on_a_wrapped_line() {
    let out = render("cooklang");
    for line in out.lines() {
        assert!(
            !line.starts_with(' '),
            "step lines must not be indented: {line:?}\n{out}"
        );
    }
}
