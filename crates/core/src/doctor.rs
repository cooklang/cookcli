//! Recipe validation: the check behind `cook doctor validate`.
//!
//! [`validate`] walks every `.cook` and `.menu` file under a root, parses it,
//! and reports what it found. Broken recipes are the *payload*, not a failure:
//! a collection full of syntax errors still validates successfully. See
//! [`Outcome`] for the rule.

use crate::{
    diagnostic::Severity,
    find::tree_error,
    parser::{collect_diagnostics, render_report, PARSER},
    Context, CoreError, Diagnostic, Outcome, Style,
};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang_find::{RecipeEntry, RecipeTree};
use std::collections::BTreeMap;

/// A validation run.
///
/// Not `#[non_exhaustive]`: consumers construct this. `..Default::default()`
/// keeps a literal working if it grows a field.
#[derive(Debug, Clone, Default)]
pub struct ValidateRequest {
    /// Directory whose recipes to validate. Defaults to the context base path.
    ///
    /// Every [`RecipeValidation::path`] is expressed against this, so it
    /// doubles as the root results are reported relative to.
    pub base_dir: Option<Utf8PathBuf>,
    /// Whether [`RecipeValidation::rendered`] carries ANSI escape codes.
    ///
    /// Defaults to [`Style::Plain`], because a library must not put escape
    /// codes in a string a web view or a log file might receive. The CLI passes
    /// [`Style::Ansi`] for its terminal output. Nothing else about the result
    /// depends on this.
    pub style: Style,
}

/// What validating one recipe found.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct RecipeValidation {
    /// Where the recipe sits under the validation root.
    ///
    /// Always relative to that root, and the same path [`diagnostics`] and
    /// [`rendered`] name. `cooklang-find` takes every path relative to the root
    /// while building the tree, and fails the whole walk if it cannot, so there
    /// is no way for an absolute path to reach this field.
    ///
    /// [`diagnostics`]: RecipeValidation::diagnostics
    /// [`rendered`]: RecipeValidation::rendered
    pub path: Utf8PathBuf,
    /// Every problem the parser raised, errors and warnings alike, in the order
    /// the parser produced them. Empty for a recipe with nothing wrong with it.
    ///
    /// A recipe that could not be read at all carries a single error
    /// diagnostic saying so, rather than dropping out of the report.
    pub diagnostics: Vec<Diagnostic>,
    /// The parser's own multi-line report, with the offending source lines
    /// quoted, ready to print verbatim. Empty exactly when [`diagnostics`] is —
    /// except for a recipe that could not be read, which has a diagnostic but
    /// no source to quote, so nothing to render.
    ///
    /// Carries ANSI escape codes when [`ValidateRequest::style`] is
    /// [`Style::Ansi`], and none when it is [`Style::Plain`]. This is the one
    /// difference from [`CoreError::Parse::rendered`], which is always plain.
    ///
    /// [`diagnostics`]: RecipeValidation::diagnostics
    /// [`CoreError::Parse::rendered`]: crate::CoreError::Parse
    pub rendered: String,
    /// The recipes this one references, spelled as they are written in it —
    /// `./sauce`, say. In source order, with a recipe referenced twice listed
    /// twice.
    ///
    /// Nothing here has been resolved: a name in this list need not exist, and
    /// checking that is the caller's job. Empty for a recipe with errors, since
    /// `cooklang` produces no recipe to read them off — so a broken recipe's
    /// references go unchecked rather than being reported as missing.
    pub references: Vec<String>,
}

impl RecipeValidation {
    /// How many of this recipe's diagnostics have the given severity.
    fn count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count()
    }
}

/// Everything [`validate`] found under one root.
///
/// The five totals the CLI prints are **methods rather than fields**. They hold
/// nothing that [`recipes`](ValidationReport::recipes) does not, and computing
/// them on demand is what makes it impossible for a total to disagree with the
/// recipes it counts.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Every recipe found under the root, clean ones included, in path order.
    ///
    /// The order is this crate's, not the walk's: `cooklang-find` holds a
    /// directory's entries in a `HashMap`, so the walk itself yields them in an
    /// order that changes between runs. Sorting makes a printed report
    /// diffable.
    pub recipes: Vec<RecipeValidation>,
}

impl ValidationReport {
    /// How many recipes were scanned, valid or not.
    pub fn total_recipes(&self) -> usize {
        self.recipes.len()
    }

    /// How many recipes have at least one error.
    pub fn recipes_with_errors(&self) -> usize {
        self.recipes
            .iter()
            .filter(|r| r.count(Severity::Error) > 0)
            .count()
    }

    /// How many recipes have at least one warning.
    pub fn recipes_with_warnings(&self) -> usize {
        self.recipes
            .iter()
            .filter(|r| r.count(Severity::Warning) > 0)
            .count()
    }

    /// How many errors there are in total, across every recipe.
    pub fn total_errors(&self) -> usize {
        self.recipes.iter().map(|r| r.count(Severity::Error)).sum()
    }

    /// How many warnings there are in total, across every recipe.
    pub fn total_warnings(&self) -> usize {
        self.recipes
            .iter()
            .map(|r| r.count(Severity::Warning))
            .sum()
    }

    /// The references each recipe makes, keyed by recipe path, for callers that
    /// want to check them all at once. Recipes that reference nothing are left
    /// out entirely.
    ///
    /// A view over [`RecipeValidation::references`], borrowed rather than
    /// cloned, and ordered so that a caller reporting broken references reports
    /// them the same way twice running.
    pub fn references(&self) -> BTreeMap<&Utf8Path, &[String]> {
        self.recipes
            .iter()
            .filter(|r| !r.references.is_empty())
            .map(|r| (r.path.as_path(), r.references.as_slice()))
            .collect()
    }
}

/// Validate every recipe under `req`'s root.
///
/// The root is [`ValidateRequest::base_dir`], or [`Context::base_path`] when
/// that is unset. Nothing else on the context is consulted.
///
/// # Errors are data
///
/// This returns `Ok` for a collection of entirely broken recipes: finding those
/// errors *is* the job, and they come back in the report. It returns `Err` only
/// when the walk could not happen at all. The returned [`Outcome`] also carries
/// every diagnostic as one flat list, so that
/// [`has_errors`](Outcome::has_errors) means what it says — the same
/// diagnostics as in the report, each naming its own file.
///
/// # Errors
///
/// - [`CoreError::Search`] if the root does not exist, is not a directory, or
///   cannot be turned into a search pattern.
/// - [`CoreError::Io`] if a file under the root turned up in the walk and could
///   not be listed. A file that is listed and then cannot be *read* is not an
///   error: it is one recipe in the report carrying one error diagnostic.
pub fn validate(
    ctx: &Context,
    req: ValidateRequest,
) -> Result<Outcome<ValidationReport>, CoreError> {
    let base_dir = req
        .base_dir
        .unwrap_or_else(|| ctx.base_path().to_path_buf());

    tracing::trace!("validating recipes under {base_dir}");

    let tree = cooklang_find::build_tree(&base_dir).map_err(|e| tree_error(e, &base_dir))?;

    let mut recipes = Vec::new();
    collect(&tree, &base_dir, req.style, &mut recipes);
    recipes.sort_by(|a, b| a.path.cmp(&b.path));

    let diagnostics = recipes
        .iter()
        .flat_map(|r| r.diagnostics.iter().cloned())
        .collect();

    Ok(Outcome::with_diagnostics(
        ValidationReport { recipes },
        diagnostics,
    ))
}

/// Walk the tree depth-first, validating every recipe node.
fn collect(tree: &RecipeTree, base_dir: &Utf8Path, style: Style, out: &mut Vec<RecipeValidation>) {
    if let Some(entry) = &tree.recipe {
        out.push(validate_entry(entry, base_dir, style));
    }
    for subtree in tree.children.values() {
        collect(subtree, base_dir, style, out);
    }
}

/// Read, parse and describe one recipe. Never fails: a recipe that cannot be
/// read is described as such, so that one bad file does not end the walk.
fn validate_entry(entry: &RecipeEntry, base_dir: &Utf8Path, style: Style) -> RecipeValidation {
    // `build_tree` only ever produces named, file-backed entries, so neither
    // fallback is reachable through it. They are kept because skipping an entry
    // instead would make `total_recipes` disagree with the tree that was
    // walked, which is worse than reporting a file that cannot be read.
    let name = entry
        .name()
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let full_path = entry.path().cloned().unwrap_or_else(|| base_dir.join(name));
    let path = relative_to(base_dir, &full_path);

    let content = match std::fs::read_to_string(&full_path) {
        Ok(content) => content,
        Err(e) => {
            return RecipeValidation {
                // Phrased as the CLI has always printed it. The path is not
                // repeated into the message: it is on the location, and on the
                // header every caller prints above it.
                diagnostics: vec![
                    Diagnostic::error(format!("Failed to read file: {e}")).at_file(path.clone())
                ],
                path,
                rendered: String::new(),
                references: Vec::new(),
            };
        }
    };

    let parsed = PARSER.parse(&content);
    let diagnostics = collect_diagnostics(parsed.report(), Some(&path));

    // `write` on an empty report produces an empty string anyway; the guard is
    // to skip indexing the source lines of every healthy recipe in a
    // collection, which is the common case.
    let rendered = if diagnostics.is_empty() {
        String::new()
    } else {
        render_report(parsed.report(), path.as_str(), &content, style.is_ansi())
    };

    // No output means no references: `cooklang` produces none for a recipe
    // with errors, so a broken recipe contributes nothing here.
    let references = parsed
        .output()
        .map(|recipe| {
            recipe
                .ingredients
                .iter()
                .filter_map(|ingredient| ingredient.reference.as_ref())
                .map(|reference| {
                    if reference.components.is_empty() {
                        reference.name.clone()
                    } else {
                        reference.path("/")
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    RecipeValidation {
        path,
        diagnostics,
        rendered,
        references,
    }
}

/// Express `path` relative to `base_dir`, leaving it whole when it does not
/// start with it.
///
/// Unlike the search module's namesake, the fallback here cannot fire:
/// `build_tree` takes the same prefix off every path it yields, and fails the
/// whole walk with `TreeError::StripPrefixError` rather than yielding one that
/// will not strip. A root spelled `./recipes` is what provokes that — the walk
/// resolves the pattern to `recipes/soup.cook`, losing the `./` — and it comes
/// back as [`CoreError::Search`] from [`validate`], never as a path here.
/// Returning the path whole still beats unwrapping, in a crate a NAPI addon
/// calls.
fn relative_to(base_dir: &Utf8Path, path: &Utf8Path) -> Utf8PathBuf {
    path.strip_prefix(base_dir).unwrap_or(path).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cooklang_find::tree::TreeError;

    const CLEAN: &str = "---\ntitle: Basic Sauce\n---\n\nHeat @oil{2%tbsp} in a #pan.\n";
    /// Deprecated `>>` metadata parses, with one warning.
    const DEPRECATED: &str = ">> title: Old Style\n\nBoil @water{1%l}.\n";
    /// Two ingredients with quantities but no name: two hard parse errors.
    const BROKEN: &str = "---\ntitle: Broken\n---\n\nAdd @{1%tsp} and @{2%tsp}.\n";

    /// A collection with one of everything: a clean recipe in a subdirectory,
    /// a clean recipe making references, one that warns and one that errors.
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let base = base(&dir);
        std::fs::create_dir(base.join("Breakfast")).unwrap();
        write(
            &base.join("Breakfast").join("pancakes.cook"),
            "---\ntitle: Pancakes\n---\n\nMix @flour{2%cups}.\n",
        );
        write(&base.join("sauce.cook"), CLEAN);
        write(
            &base.join("with_ref.cook"),
            "---\ntitle: With Reference\n---\n\nMake @./sauce{} and @./nonexistent{}.\n",
        );
        write(&base.join("deprecated.cook"), DEPRECATED);
        write(&base.join("broken.cook"), BROKEN);
        dir
    }

    fn write(path: &Utf8Path, text: &str) {
        std::fs::write(path, text).unwrap();
    }

    fn base(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
    }

    /// Validate a root, through the context base path.
    fn run(base_dir: &Utf8Path) -> ValidationReport {
        run_styled(base_dir, Style::Plain)
    }

    fn run_styled(base_dir: &Utf8Path, style: Style) -> ValidationReport {
        validate(
            &Context::new(base_dir.to_owned()),
            ValidateRequest {
                base_dir: None,
                style,
            },
        )
        .expect("validation succeeds")
        .into_value()
    }

    fn paths(report: &ValidationReport) -> Vec<String> {
        report.recipes.iter().map(|r| r.path.to_string()).collect()
    }

    fn recipe<'a>(report: &'a ValidationReport, path: &str) -> &'a RecipeValidation {
        report
            .recipes
            .iter()
            .find(|r| r.path == path)
            .unwrap_or_else(|| panic!("{path} missing from {:?}", paths(report)))
    }

    /// Subdirectories are walked, and every recipe is reported whether or not
    /// there is anything wrong with it.
    #[test]
    fn every_recipe_under_the_root_is_reported_including_nested_ones() {
        let dir = fixture();
        let report = run(&base(&dir));
        assert_eq!(
            paths(&report),
            [
                "Breakfast/pancakes.cook",
                "broken.cook",
                "deprecated.cook",
                "sauce.cook",
                "with_ref.cook",
            ]
        );
        assert_eq!(report.total_recipes(), 5);
    }

    /// The five totals the CLI prints, pinned together so that one of them
    /// going wrong cannot hide behind another.
    #[test]
    fn totals_count_diagnostics_and_the_recipes_carrying_them() {
        let dir = fixture();
        let report = run(&base(&dir));

        assert_eq!(report.total_recipes(), 5);
        assert_eq!(report.total_errors(), 2, "both errors in broken.cook");
        assert_eq!(report.recipes_with_errors(), 1);
        assert_eq!(report.total_warnings(), 1, "deprecated.cook warns once");
        assert_eq!(report.recipes_with_warnings(), 1);
    }

    #[test]
    fn a_failing_recipe_carries_both_diagnostics_and_a_rendered_report() {
        let dir = fixture();
        let report = run(&base(&dir));
        let broken = recipe(&report, "broken.cook");

        assert_eq!(broken.diagnostics.len(), 2);
        for d in &broken.diagnostics {
            assert_eq!(d.severity, Severity::Error, "expected errors: {d:?}");
        }
        // The structured half locates the problem in the file it came from.
        let location = broken.diagnostics[0]
            .location
            .as_ref()
            .expect("location set");
        assert_eq!(location.file.as_deref(), Some(Utf8Path::new("broken.cook")));
        assert!(location.span.is_some(), "an error must carry its span");

        // ...and the rendered half quotes the source, which is the whole
        // reason it is carried alongside.
        assert!(
            broken.rendered.contains("broken.cook"),
            "report should name the file: {}",
            broken.rendered
        );
        assert!(
            broken.rendered.contains("Add @{1%tsp} and @{2%tsp}."),
            "report should quote the source line: {}",
            broken.rendered
        );
    }

    #[test]
    fn a_warning_is_reported_as_a_warning_not_an_error() {
        let dir = fixture();
        let report = run(&base(&dir));
        let deprecated = recipe(&report, "deprecated.cook");

        assert_eq!(deprecated.diagnostics.len(), 1);
        assert_eq!(deprecated.diagnostics[0].severity, Severity::Warning);
        assert!(
            !deprecated.rendered.is_empty(),
            "a warning is worth showing"
        );
    }

    #[test]
    fn a_clean_recipe_carries_neither_diagnostics_nor_a_rendered_report() {
        let dir = fixture();
        let report = run(&base(&dir));
        let clean = recipe(&report, "sauce.cook");

        assert!(clean.diagnostics.is_empty(), "{:?}", clean.diagnostics);
        assert!(clean.rendered.is_empty(), "{:?}", clean.rendered);
    }

    #[test]
    fn references_are_collected_as_they_are_written() {
        let dir = fixture();
        let report = run(&base(&dir));

        assert_eq!(
            recipe(&report, "with_ref.cook").references,
            ["./sauce", "./nonexistent"],
            "in source order, unresolved"
        );
        assert!(recipe(&report, "sauce.cook").references.is_empty());
    }

    /// A recipe referenced twice is listed twice: the list is what the recipe
    /// says, not a set. Deduplicating here would quietly hide a double
    /// reference from anything counting them.
    #[test]
    fn a_reference_made_twice_is_listed_twice() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = base(&dir);
        write(&base.join("sauce.cook"), CLEAN);
        write(
            &base.join("dup.cook"),
            "---\ntitle: Dup\n---\n\nMake @./sauce{}, then more @./sauce{}.\n",
        );

        let report = run(&base);
        assert_eq!(
            recipe(&report, "dup.cook").references,
            ["./sauce", "./sauce"]
        );
    }

    /// `cooklang` produces no recipe at all for one with errors, so there is
    /// nothing to read references off. Pinned because it is the reason a broken
    /// recipe's references go unchecked, which reads like a bug until you know
    /// it is deliberate.
    #[test]
    fn a_recipe_with_errors_contributes_no_references() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = base(&dir);
        write(
            &base.join("broken_ref.cook"),
            "---\ntitle: Broken\n---\n\nMake @./sauce{} and add @{1%tsp}.\n",
        );

        let report = run(&base);
        let broken = recipe(&report, "broken_ref.cook");
        assert_eq!(broken.count(Severity::Error), 1, "{:?}", broken.diagnostics);
        assert!(
            broken.references.is_empty(),
            "expected no references, got {:?}",
            broken.references
        );
        assert!(report.references().is_empty());
    }

    /// The map view leaves out recipes that reference nothing, so a caller
    /// checking references does not have to.
    #[test]
    fn the_reference_map_holds_only_recipes_that_reference_something() {
        let dir = fixture();
        let report = run(&base(&dir));
        let references = report.references();

        assert_eq!(
            references.keys().copied().collect::<Vec<_>>(),
            [Utf8Path::new("with_ref.cook")]
        );
        assert_eq!(references[Utf8Path::new("with_ref.cook")].len(), 2);
    }

    /// A file listed by the walk that cannot then be read is one error, and the
    /// walk carries on. The file is valid UTF-8 through its front matter — so
    /// the walk lists it — and invalid after, so reading the whole of it fails.
    /// That needs no permission games, and so behaves the same on every
    /// platform.
    #[test]
    fn a_file_that_cannot_be_read_is_one_error_and_does_not_end_the_walk() {
        let dir = fixture();
        let base = base(&dir);
        std::fs::write(
            base.join("bad_bytes.cook"),
            b"---\ntitle: Bad Bytes\n---\n\nBoil @water{1%l} \xFF.\n",
        )
        .unwrap();

        let report = run(&base);
        let unreadable = recipe(&report, "bad_bytes.cook");

        assert_eq!(unreadable.diagnostics.len(), 1);
        assert_eq!(unreadable.diagnostics[0].severity, Severity::Error);
        assert!(
            unreadable.diagnostics[0]
                .message
                .starts_with("Failed to read file"),
            "{:?}",
            unreadable.diagnostics[0]
        );
        assert!(
            unreadable.rendered.is_empty(),
            "there is no source to quote: {:?}",
            unreadable.rendered
        );
        assert_eq!(unreadable.references, Vec::<String>::new());

        // It counts, once, as one error in one recipe...
        assert_eq!(report.total_recipes(), 6);
        assert_eq!(report.total_errors(), 3);
        assert_eq!(report.recipes_with_errors(), 2);
        // ...and everything else was still validated.
        assert_eq!(recipe(&report, "broken.cook").diagnostics.len(), 2);
        assert!(recipe(&report, "sauce.cook").diagnostics.is_empty());
    }

    /// Broken recipes are the payload. Only the walk failing is an `Err`.
    #[test]
    fn a_collection_full_of_errors_still_validates_successfully() {
        let dir = tempfile::TempDir::new().unwrap();
        write(&base(&dir).join("broken.cook"), BROKEN);

        let outcome = validate(&Context::new(base(&dir)), ValidateRequest::default())
            .expect("errors in recipes are data, not a failed command");

        assert_eq!(outcome.value.total_errors(), 2);
        assert!(
            outcome.has_errors(),
            "the outcome must carry the errors it found: {:?}",
            outcome.diagnostics
        );
        assert_eq!(
            outcome.diagnostics.len(),
            2,
            "every diagnostic in the report, flat"
        );
    }

    /// A clean collection says so in both places.
    #[test]
    fn a_clean_collection_has_no_diagnostics_at_all() {
        let dir = tempfile::TempDir::new().unwrap();
        write(&base(&dir).join("sauce.cook"), CLEAN);

        let outcome = validate(&Context::new(base(&dir)), ValidateRequest::default()).unwrap();
        assert!(!outcome.has_errors());
        assert!(outcome.diagnostics.is_empty());
        assert_eq!(outcome.value.total_errors(), 0);
        assert_eq!(outcome.value.total_warnings(), 0);
        assert_eq!(outcome.value.total_recipes(), 1);
    }

    #[test]
    fn style_decides_whether_the_rendered_report_is_coloured() {
        let dir = fixture();
        let base = base(&dir);

        let plain = run_styled(&base, Style::Plain);
        let plain = &recipe(&plain, "broken.cook").rendered;
        assert!(
            !plain.contains('\u{1b}'),
            "Style::Plain must emit no escape codes: {plain:?}"
        );

        let coloured = run_styled(&base, Style::Ansi);
        let coloured = &recipe(&coloured, "broken.cook").rendered;
        assert!(
            coloured.contains('\u{1b}'),
            "Style::Ansi must emit escape codes: {coloured:?}"
        );
        assert_eq!(
            *plain,
            anstream::adapter::strip_str(coloured).to_string(),
            "the two must differ only in the escape codes"
        );
    }

    /// The default is the safe one, because a library must not hand escape
    /// codes to a caller that never asked for them.
    #[test]
    fn the_default_style_is_plain() {
        assert_eq!(ValidateRequest::default().style, Style::Plain);
    }

    #[test]
    fn base_dir_overrides_the_context_base_path() {
        let validated = fixture();
        let ignored = tempfile::TempDir::new().unwrap();
        write(&base(&ignored).join("decoy.cook"), BROKEN);

        let report = validate(
            &Context::new(base(&ignored)),
            ValidateRequest {
                base_dir: Some(base(&validated)),
                style: Style::Plain,
            },
        )
        .expect("validation succeeds")
        .into_value();

        assert_eq!(report.total_recipes(), 5);
        assert!(
            !paths(&report).contains(&"decoy.cook".to_string()),
            "{:?}",
            paths(&report)
        );
    }

    #[test]
    fn without_a_base_dir_the_context_base_path_is_validated() {
        let dir = fixture();
        assert_eq!(run(&base(&dir)).total_recipes(), 5);
    }

    /// `cooklang-find` holds a directory's entries in a `HashMap`, so the walk
    /// order changes from run to run. Sorting is what makes a printed report
    /// diffable, and it is asserted over several runs because a single run of
    /// an unsorted walk can come out sorted by luck.
    #[test]
    fn recipes_come_back_in_path_order_every_time() {
        let dir = fixture();
        let base = base(&dir);
        let expected = paths(&run(&base));
        let mut sorted = expected.clone();
        sorted.sort();
        assert_eq!(expected, sorted);

        for _ in 0..8 {
            assert_eq!(paths(&run(&base)), expected, "order must not vary");
        }
    }

    #[test]
    fn an_empty_directory_validates_to_an_empty_report() {
        let dir = tempfile::TempDir::new().unwrap();
        let report = run(&base(&dir));
        assert_eq!(report.total_recipes(), 0);
        assert!(report.references().is_empty());
    }

    /// Unlike a search, a root that is not there is a real failure: there is
    /// nothing to validate and the caller almost certainly mistyped it.
    #[test]
    fn a_root_that_does_not_exist_is_reported() {
        let dir = tempfile::TempDir::new().unwrap();
        let missing = base(&dir).join("nope");

        match validate(&Context::new(missing.clone()), ValidateRequest::default()) {
            Err(CoreError::Search { base_dir, message }) => {
                assert_eq!(base_dir, missing);
                assert_eq!(message, "no such directory");
            }
            other => panic!(
                "expected CoreError::Search, got {:?}",
                other.map(|o| o.value)
            ),
        }
    }

    #[test]
    fn a_root_that_is_a_file_is_reported() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = base(&dir).join("sauce.cook");
        write(&file, CLEAN);

        match validate(&Context::new(file.clone()), ValidateRequest::default()) {
            Err(CoreError::Search { base_dir, message }) => {
                assert_eq!(base_dir, file);
                assert_eq!(message, "not a directory");
            }
            other => panic!(
                "expected CoreError::Search, got {:?}",
                other.map(|o| o.value)
            ),
        }
    }

    /// A root whose name contains glob syntax is a real directory a user can
    /// really have, and it cannot be turned into a pattern.
    #[test]
    fn a_root_that_is_not_a_valid_glob_pattern_is_reported() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = base(&dir).join("re[ci");
        std::fs::create_dir(&root).unwrap();
        write(&root.join("sauce.cook"), CLEAN);

        match validate(&Context::new(root.clone()), ValidateRequest::default()) {
            Err(CoreError::Search { base_dir, message }) => {
                assert_eq!(base_dir, root);
                assert!(
                    message.contains("attern"),
                    "the cause must survive: {message}"
                );
            }
            other => panic!(
                "expected CoreError::Search, got {:?}",
                other.map(|o| o.value)
            ),
        }
    }

    /// The remaining mappings need a failure that cannot be arranged from a
    /// test, so they are pinned through `tree_error` directly.
    #[test]
    fn a_listing_failure_names_the_file_rather_than_the_root() {
        let root = Utf8Path::new("/recipes");

        let unusable = tree_error(
            TreeError::RecipeEntryError(cooklang_find::RecipeEntryError::MetadataError(
                "bad front matter".to_string(),
            )),
            root,
        );
        match unusable {
            CoreError::Io { path, source } => {
                assert_eq!(path, root);
                assert!(
                    source.to_string().contains("bad front matter"),
                    "the cause must survive: {source}"
                );
            }
            other => panic!("expected CoreError::Io, got {other:?}"),
        }

        let unstrippable = tree_error(
            TreeError::StripPrefixError("recipes/soup.cook".to_string()),
            root,
        );
        match unstrippable {
            CoreError::Search { base_dir, message } => {
                assert_eq!(base_dir, root);
                assert!(message.contains("recipes/soup.cook"), "{message}");
            }
            other => panic!("expected CoreError::Search, got {other:?}"),
        }
    }

    #[test]
    fn a_path_under_the_root_is_stripped_and_one_outside_is_left_alone() {
        assert_eq!(
            relative_to(
                Utf8Path::new("/recipes"),
                Utf8Path::new("/recipes/Breakfast/pancakes.cook")
            ),
            "Breakfast/pancakes.cook"
        );
        assert_eq!(
            relative_to(
                Utf8Path::new("./recipes"),
                Utf8Path::new("recipes/soup.cook")
            ),
            "recipes/soup.cook"
        );
    }
}
