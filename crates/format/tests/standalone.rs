//! The crate's promise: render a recipe using nothing but `cooklang-format`.
//!
//! An integration test, so it sees only the public API — if something a
//! consumer needs is private or unexported, this fails to compile, which is
//! exactly the failure worth catching.

use cooklang_format::cooklang::{Converter, CooklangParser, Extensions, Recipe};
use cooklang_format::{markdown_to_string, PaperSize, Style};

const RECIPE: &str = "---\ntitle: Tea\n---\n\nBoil @water{2%cups} in a #pot for ~{5%minutes}.\n";

/// The margin, in centimetres, that `cook recipe` defaults to for the two
/// typeset formats.
const MARGIN: f64 = 2.5;

fn parse(parser: &CooklangParser) -> Recipe {
    let (recipe, _) = parser
        .parse(RECIPE)
        .into_result()
        .expect("the fixture parses");
    recipe
}

#[test]
fn renders_markdown_without_cookcli_core() {
    let parser = CooklangParser::new(Extensions::empty(), Converter::default());
    let recipe = parse(&parser);

    let md = markdown_to_string(&recipe, "Tea", 1.0, parser.converter()).expect("formats");

    assert!(md.contains("water"), "ingredient missing from:\n{md}");
    assert!(md.contains("pot"), "cookware missing from:\n{md}");
}

#[test]
fn renders_every_format_the_crate_advertises() {
    let parser = CooklangParser::new(Extensions::empty(), Converter::default());
    let recipe = parse(&parser);
    let conv = parser.converter();

    let mut buf = Vec::new();
    cooklang_format::human::print_human(&recipe, "Tea", 1.0, conv, Style::Plain, &mut buf)
        .expect("human");
    assert!(!buf.is_empty(), "human output is empty");

    // The writer comes before the page setup in both typeset formatters, and
    // both take a margin — this mirrors the CLI's call in `src/recipe/read.rs`.
    let mut buf = Vec::new();
    cooklang_format::latex::print_latex(&recipe, "Tea", 1.0, conv, &mut buf, PaperSize::A4, MARGIN)
        .expect("latex");
    assert!(!buf.is_empty(), "latex output is empty");

    let mut buf = Vec::new();
    cooklang_format::typst::print_typst(&recipe, "Tea", 1.0, conv, &mut buf, PaperSize::A4, MARGIN)
        .expect("typst");
    assert!(!buf.is_empty(), "typst output is empty");

    let mut buf = Vec::new();
    cooklang_format::markdown::print_md(&recipe, "Tea", 1.0, conv, &mut buf).expect("markdown");
    assert!(!buf.is_empty(), "markdown output is empty");

    let mut buf = Vec::new();
    cooklang_format::schema::print_schema(&recipe, "Tea", 1.0, conv, &mut buf, false)
        .expect("schema");
    assert!(!buf.is_empty(), "schema output is empty");

    // `cooklang_source`, not `cooklang`: the bare name is the re-exported
    // parser crate.
    let mut buf = Vec::new();
    cooklang_format::cooklang_source::print_cooklang(&recipe, &mut buf).expect("cooklang");
    assert!(!buf.is_empty(), "cooklang output is empty");
}
