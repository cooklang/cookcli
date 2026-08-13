//! The checks behind `cook doctor`.
//!
//! [`validate`] walks every `.cook` and `.menu` file under a root, parses it,
//! and reports what it found. Broken recipes are the *payload*, not a failure:
//! a collection full of syntax errors still validates successfully. See
//! [`Outcome`] for the rule. [`broken_references`] follows up on what it found,
//! resolving the recipe references a report collected.
//!
//! [`aisle_coverage`] and [`pantry_coverage`] answer the other two questions
//! `cook doctor` asks: which of a collection's ingredients are categorised in
//! `aisle.conf`, and which of them are already in the pantry.

use crate::{
    diagnostic::{parse_failure, Severity},
    find::{build_tree, listed_ingredients, parse_or_skip, walk},
    parser::{collect_diagnostics, render_report, PARSER},
    ConfigSource, Context, CoreError, Diagnostic, Outcome, Style,
};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang_find::RecipeEntry;
use std::collections::{BTreeMap, BTreeSet};

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
    /// The root that was walked: [`ValidateRequest::base_dir`], or the
    /// context's base path when that was unset.
    ///
    /// Carried so that a report is self-contained — every
    /// [`RecipeValidation::path`] is relative to this, and
    /// [`broken_references`] resolves against it without having to be told the
    /// root a second time and possibly told it wrong.
    pub base_dir: Utf8PathBuf,
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
    ///
    /// **Nothing here has been resolved**, and a reference that resolves to
    /// nothing raises no diagnostic, so it does not reach
    /// [`Outcome::diagnostics`] or [`has_errors`](Outcome::has_errors) either.
    /// Pass the report to [`broken_references`] to find out which of these lead
    /// anywhere.
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
/// # `has_errors` is not the whole verdict
///
/// **A broken recipe reference is not a diagnostic**, so a collection whose
/// only fault is a reference leading nowhere has an empty
/// [`Outcome::diagnostics`] and `has_errors() == false`. Resolving references
/// costs a filesystem lookup each and needs a decision this function does not
/// make, so it is [`broken_references`]'s separate job.
///
/// A caller gating on validation — a CI exit code, an editor's problem list —
/// therefore wants both:
///
/// ```no_run
/// # use cookcli_core::{doctor, Context};
/// # fn main() -> Result<(), cookcli_core::CoreError> {
/// # let ctx = Context::new("recipes".into());
/// let outcome = doctor::validate(&ctx, doctor::ValidateRequest::default())?;
/// let ok = !outcome.has_errors() && doctor::broken_references(&outcome.value).is_empty();
/// # let _ = ok;
/// # Ok(())
/// # }
/// ```
///
/// `cook doctor validate --strict` fails on either, which is why it does its
/// own arithmetic over the two.
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

    let tree = build_tree(&base_dir)?;

    let mut recipes: Vec<RecipeValidation> = walk(&tree)
        .into_iter()
        .map(|entry| validate_entry(entry, &base_dir, req.style))
        .collect();
    // `walk` already orders the entries by their full path, which under one
    // root is the same order; sorted again on the path as reported, because
    // that is what this crate promises and what makes it true whatever `walk`
    // decides to do.
    recipes.sort_by(|a, b| a.path.cmp(&b.path));

    let diagnostics = recipes
        .iter()
        .flat_map(|r| r.diagnostics.iter().cloned())
        .collect();

    Ok(Outcome::with_diagnostics(
        ValidationReport { base_dir, recipes },
        diagnostics,
    ))
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

// ---------------------------------------------------------------------------
// Recipe references
// ---------------------------------------------------------------------------

/// Resolve every reference a report collected, keeping the ones that lead
/// nowhere.
///
/// Keyed by the referring recipe, as [`ValidationReport::references`] is, and
/// carrying the references in the order that recipe writes them — so a recipe
/// making the same broken reference twice reports it twice. Recipes all of
/// whose references resolve are left out entirely, so an empty map means every
/// reference in the collection is good.
///
/// Every reference is resolved against [`ValidationReport::base_dir`], the root
/// that was validated, rather than against the directory of the recipe making
/// it. That is what `cook doctor validate` has always done, and it means a
/// reference is judged by whether *the collection* holds a recipe of that name.
///
/// "Broken" is the whole of "could not be resolved to a readable recipe": a
/// reference naming a file that exists but cannot be opened counts, exactly as
/// one naming nothing at all does. Reporting them apart would need a reason on
/// every entry, which nothing has yet asked for.
///
/// Nothing here fails, so there is no `Result`: this is the check, and what it
/// finds is the answer. It does touch the filesystem, once per reference —
/// which is why it is a function rather than a method on the report.
pub fn broken_references(report: &ValidationReport) -> BTreeMap<&Utf8Path, Vec<String>> {
    report
        .references()
        .into_iter()
        .filter_map(|(recipe, references)| {
            let broken: Vec<String> = references
                .iter()
                .filter(|reference| crate::find::get_recipe(&report.base_dir, reference).is_err())
                .cloned()
                .collect();
            (!broken.is_empty()).then_some((recipe, broken))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Ingredient coverage
// ---------------------------------------------------------------------------

/// Which collection to check a configuration against.
///
/// Not `#[non_exhaustive]`: consumers construct this. `..Default::default()`
/// keeps a literal working if it grows a field.
#[derive(Debug, Clone, Default)]
pub struct CoverageRequest {
    /// Directory whose recipes to scan. Defaults to the context base path.
    pub base_dir: Option<Utf8PathBuf>,
}

/// One ingredient a collection uses, and whether a configuration knows it.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedIngredient {
    /// The ingredient's name, spelled as the recipes write it. Where two
    /// recipes spell it differently — `Salt` and `salt` — both spellings are
    /// listed, even though the configuration is matched ignoring case.
    pub name: String,
    /// Whether the configuration names this ingredient.
    pub known: bool,
}

/// How much of a collection's ingredients a configuration accounts for.
///
/// The two views callers want — what is covered and what is not — are
/// [`known`](IngredientCoverage::known) and
/// [`unknown`](IngredientCoverage::unknown), derived from
/// [`ingredients`](IngredientCoverage::ingredients) rather than stored beside
/// it, so that they cannot disagree with it or with each other.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IngredientCoverage {
    /// How many recipes were scanned, including any that could not be read or
    /// parsed — those contribute no ingredients, and say so in
    /// [`Outcome::diagnostics`].
    pub total_recipes: usize,
    /// Every distinct ingredient the collection uses, each marked with whether
    /// the configuration knows it.
    ///
    /// Ordered by Unicode code point rather than alphabetically, so every
    /// capitalised name sorts before every lowercase one: `Beetroot`,
    /// `Zucchini`, `apple`. That is what CookCLI has always printed, and
    /// changing it would move output nobody asked to have moved — a consumer
    /// wanting a human ordering should sort these itself.
    ///
    /// References to other recipes are not ingredients and are left out.
    pub ingredients: Vec<CheckedIngredient>,
}

impl IngredientCoverage {
    /// How many distinct ingredients the collection uses.
    pub fn total_ingredients(&self) -> usize {
        self.ingredients.len()
    }

    /// The ingredients the configuration knows, in the order of
    /// [`ingredients`](IngredientCoverage::ingredients).
    pub fn known(&self) -> impl Iterator<Item = &str> {
        self.filtered(true)
    }

    /// The ingredients the configuration does not know, in the order of
    /// [`ingredients`](IngredientCoverage::ingredients).
    ///
    /// With no configuration at all this is every ingredient, since nothing is
    /// known. Ask [`ConfigSource::is_unset`] if you need to tell that from a
    /// configuration that simply covers nothing.
    pub fn unknown(&self) -> impl Iterator<Item = &str> {
        self.filtered(false)
    }

    fn filtered(&self, known: bool) -> impl Iterator<Item = &str> {
        self.ingredients
            .iter()
            .filter(move |ingredient| ingredient.known == known)
            .map(|ingredient| ingredient.name.as_str())
    }
}

/// Check the collection's ingredients against the aisle configuration
/// [`Context::aisle`] names — the question `cook doctor aisle` asks.
///
/// An ingredient is known when the configuration names it, or names it as a
/// synonym of something else, compared lowercased and otherwise exactly. With
/// no aisle configuration nothing is known; see
/// [`unknown`](IngredientCoverage::unknown).
///
/// The fold is `str::to_lowercase`, so it is case-insensitive over the whole of
/// Unicode rather than ASCII alone: `Öl` matches an entry spelled `öl`. `cook
/// doctor aisle` compared with `eq_ignore_ascii_case` before this moved here,
/// and so reported such a name as uncategorised.
///
/// A configuration that parses with warnings — a duplicate entry, say — is a
/// successful check carrying those warnings as [`Outcome::diagnostics`],
/// located in the file when it came from one.
///
/// # Errors
///
/// - [`CoreError::Io`] if the configuration is named but cannot be read.
/// - [`CoreError::Config`] if it cannot be parsed at all. `cooklang`'s aisle
///   parser is documented as never failing this way, so this is unreachable
///   today; it is listed because the signature admits it and
///   [`pantry_coverage`], which shares the code, does reach it.
/// - [`CoreError::Search`] if the collection cannot be walked, and
///   [`CoreError::Io`] if a file in it cannot be listed — as [`validate`]. A
///   recipe that cannot be *parsed* is not an error: it is left out, with a
///   warning in [`Outcome::diagnostics`].
pub fn aisle_coverage(
    ctx: &Context,
    req: CoverageRequest,
) -> Result<Outcome<IngredientCoverage>, CoreError> {
    let source = ctx.aisle();
    let mut diagnostics = Vec::new();
    let mut known = BTreeSet::new();

    if let Some(text) = source.read()? {
        let parsed = cooklang::aisle::parse_lenient(&text);
        diagnostics.extend(collect_diagnostics(parsed.report(), source.path()));
        let conf = parsed
            .output()
            .ok_or_else(|| config_error(source, "aisle", &diagnostics))?;
        // The map is keyed by each name and synonym, already lowercased.
        known.extend(conf.ingredients_info().into_keys());
    }

    coverage(ctx, req, &known, diagnostics)
}

/// Check the collection's ingredients against the pantry configuration
/// [`Context::pantry`] names — the question `cook doctor pantry` asks.
///
/// An ingredient is known when an item of that name is in stock, in any
/// section, compared lowercased and otherwise exactly; no quantity or date is
/// considered, so an item that has run out still counts as known. With no
/// pantry configuration nothing is known; see
/// [`unknown`](IngredientCoverage::unknown).
///
/// Note the direction: this reports on the *collection's* ingredients, so a
/// pantry item no recipe uses is not mentioned at all.
///
/// # Errors
///
/// Exactly as [`aisle_coverage`], except that [`CoreError::Config`] is
/// genuinely reachable here — a `pantry.conf` that is not TOML — and comes back
/// worded identically to [`pantry::load`](crate::pantry::load)'s, so that two
/// commands reading one broken file say the same thing about it.
pub fn pantry_coverage(
    ctx: &Context,
    req: CoverageRequest,
) -> Result<Outcome<IngredientCoverage>, CoreError> {
    let source = ctx.pantry();
    let mut diagnostics = Vec::new();
    let mut known = BTreeSet::new();

    if let Some(text) = source.read()? {
        let parsed = cooklang::pantry::parse_lenient(&text);
        diagnostics.extend(collect_diagnostics(parsed.report(), source.path()));
        let conf = parsed
            .output()
            .ok_or_else(|| config_error(source, "pantry", &diagnostics))?;
        known.extend(conf.all_items().map(|item| item.name().to_lowercase()));
    }

    coverage(ctx, req, &known, diagnostics)
}

/// The failure that leaves a lenient parse with no configuration at all.
///
/// The cause is taken from the diagnostics the parse did produce, through the
/// same [`parse_failure`] the pantry module words its own failures with — a
/// caller told only "it could not be parsed" has nothing to go and fix.
fn config_error(source: &ConfigSource, kind: &str, diagnostics: &[Diagnostic]) -> CoreError {
    CoreError::Config {
        path: source.path().map(ToOwned::to_owned),
        message: parse_failure(diagnostics, kind),
    }
}

/// Scan the collection and mark each ingredient against `known`, which holds
/// the configuration's names already lowercased.
fn coverage(
    ctx: &Context,
    req: CoverageRequest,
    known: &BTreeSet<String>,
    mut diagnostics: Vec<Diagnostic>,
) -> Result<Outcome<IngredientCoverage>, CoreError> {
    let base_dir = req
        .base_dir
        .unwrap_or_else(|| ctx.base_path().to_path_buf());

    let tree = build_tree(&base_dir)?;
    let entries = walk(&tree);
    let total_recipes = entries.len();

    // A set, so an ingredient two recipes share is one ingredient. Ordered, so
    // that the answer does not depend on the order `cooklang-find` happened to
    // yield the directories in.
    let mut names: BTreeSet<String> = BTreeSet::new();
    for entry in entries {
        let Some(recipe) = parse_or_skip(entry, &mut diagnostics) else {
            continue;
        };
        names.extend(listed_ingredients(&recipe));
    }

    let ingredients = names
        .into_iter()
        .map(|name| CheckedIngredient {
            known: known.contains(&name.to_lowercase()),
            name,
        })
        .collect();

    Ok(Outcome::with_diagnostics(
        IngredientCoverage {
            total_recipes,
            ingredients,
        },
        diagnostics,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::find::tree_error;
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

    // -----------------------------------------------------------------------
    // Recipe references
    // -----------------------------------------------------------------------

    /// The broken references of a collection, as pairs, for readable
    /// assertions.
    fn broken(report: &ValidationReport) -> Vec<(String, Vec<String>)> {
        broken_references(report)
            .into_iter()
            .map(|(recipe, missing)| (recipe.to_string(), missing))
            .collect()
    }

    #[test]
    fn the_report_records_the_root_it_was_validated_against() {
        let dir = fixture();
        assert_eq!(run(&base(&dir)).base_dir, base(&dir));

        let elsewhere = tempfile::TempDir::new().unwrap();
        let report = validate(
            &Context::new(base(&elsewhere)),
            ValidateRequest {
                base_dir: Some(base(&dir)),
                style: Style::Plain,
            },
        )
        .expect("validation succeeds")
        .into_value();
        assert_eq!(
            report.base_dir,
            base(&dir),
            "the root that was walked, not the context's"
        );
    }

    /// `with_ref.cook` makes two references: one to a recipe that is there and
    /// one to a recipe that is not. Only the second is reported.
    #[test]
    fn only_references_that_resolve_to_nothing_are_reported() {
        let dir = fixture();
        assert_eq!(
            broken(&run(&base(&dir))),
            [(
                "with_ref.cook".to_string(),
                vec!["./nonexistent".to_string()]
            )]
        );
    }

    #[test]
    fn a_collection_whose_references_all_resolve_reports_none() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = base(&dir);
        write(&base.join("sauce.cook"), CLEAN);
        write(
            &base.join("dish.cook"),
            "---\ntitle: Dish\n---\n\nMake @./sauce{}.\n",
        );

        assert!(
            broken_references(&run(&base)).is_empty(),
            "a resolvable reference must not be reported"
        );
    }

    /// A recipe that makes the same broken reference twice has something wrong
    /// with it twice, and the CLI counts one error per mention.
    #[test]
    fn a_reference_repeated_is_reported_once_per_mention() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = base(&dir);
        write(
            &base.join("dish.cook"),
            "---\ntitle: Dish\n---\n\nMake @./absent{}, then more @./absent{}.\n",
        );

        assert_eq!(
            broken(&run(&base)),
            [(
                "dish.cook".to_string(),
                vec!["./absent".to_string(), "./absent".to_string()]
            )]
        );
    }

    /// References are looked up in the collection as a whole, so a recipe in a
    /// subdirectory can reference one at the root. This is what makes the
    /// spelling `./sauce` work from anywhere, and it is the behaviour `cook
    /// doctor validate` has always had.
    #[test]
    fn references_resolve_against_the_validated_root_from_anywhere_in_it() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = base(&dir);
        write(&base.join("sauce.cook"), CLEAN);
        std::fs::create_dir(base.join("Dinner")).unwrap();
        write(
            &base.join("Dinner").join("dish.cook"),
            "---\ntitle: Dish\n---\n\nMake @./sauce{}.\n",
        );

        assert!(
            broken_references(&run(&base)).is_empty(),
            "a nested recipe must be able to reference the root's"
        );
    }

    /// A collection with nothing to check comes back empty rather than
    /// reporting anything.
    #[test]
    fn a_collection_with_no_references_has_none_broken() {
        let dir = tempfile::TempDir::new().unwrap();
        write(&base(&dir).join("sauce.cook"), CLEAN);
        assert!(broken_references(&run(&base(&dir))).is_empty());
    }

    // -----------------------------------------------------------------------
    // Ingredient coverage
    // -----------------------------------------------------------------------

    /// A collection of one recipe, so that a check has something to scan.
    fn one_recipe(text: &str) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        write(&base(&dir).join("dish.cook"), text);
        dir
    }

    fn aisle_ctx(dir: &tempfile::TempDir, conf: &str) -> Context {
        Context::new(base(dir)).with_aisle(ConfigSource::Inline(conf.to_string()))
    }

    fn pantry_ctx(dir: &tempfile::TempDir, conf: &str) -> Context {
        Context::new(base(dir)).with_pantry(ConfigSource::Inline(conf.to_string()))
    }

    fn checked(ctx: &Context, aisle: bool) -> Outcome<IngredientCoverage> {
        let request = CoverageRequest::default();
        if aisle {
            aisle_coverage(ctx, request)
        } else {
            pantry_coverage(ctx, request)
        }
        .expect("the check succeeds")
    }

    fn known(coverage: &IngredientCoverage) -> Vec<&str> {
        coverage.known().collect()
    }

    fn unknown(coverage: &IngredientCoverage) -> Vec<&str> {
        coverage.unknown().collect()
    }

    fn all(coverage: &IngredientCoverage) -> Vec<&str> {
        coverage
            .ingredients
            .iter()
            .map(|ingredient| ingredient.name.as_str())
            .collect()
    }

    #[test]
    fn an_aisle_splits_the_collection_into_categorised_and_not() {
        let dir = one_recipe("Add @salt{1%tsp}, @water{1%l} and @leek{1}.\n");
        let coverage = checked(
            &aisle_ctx(&dir, "[produce]\nleek\n\n[pantry]\nsalt\n"),
            true,
        )
        .value;

        assert_eq!(known(&coverage), ["leek", "salt"]);
        assert_eq!(unknown(&coverage), ["water"]);
        assert_eq!(coverage.total_ingredients(), 3);
        assert_eq!(coverage.total_recipes, 1);
    }

    /// An aisle entry naming several spellings of one thing knows all of them.
    #[test]
    fn an_aisle_synonym_counts_as_knowing_the_ingredient() {
        let dir = one_recipe("Add @aubergine{1}.\n");
        let coverage = checked(&aisle_ctx(&dir, "[produce]\neggplant|aubergine\n"), true).value;

        assert_eq!(known(&coverage), ["aubergine"]);
        assert!(unknown(&coverage).is_empty());
    }

    #[test]
    fn a_pantry_knows_its_items_from_every_section() {
        let dir = one_recipe("Add @salt{1%tsp}, @milk{1%l} and @water{1%l}.\n");
        let conf = "[pantry]\nsalt = \"1%kg\"\n\n[dairy]\nmilk = \"1%l\"\n";
        let coverage = checked(&pantry_ctx(&dir, conf), false).value;

        assert_eq!(known(&coverage), ["milk", "salt"]);
        assert_eq!(unknown(&coverage), ["water"]);
    }

    /// Nothing about the stock is considered: an item that has run out is
    /// still an item the pantry knows about.
    #[test]
    fn a_pantry_item_that_has_run_out_still_counts_as_known() {
        let dir = one_recipe("Add @honey{1%tbsp}.\n");
        let conf = "[pantry]\nhoney = { quantity = \"0\", low = \"100%g\" }\n";
        assert_eq!(
            known(&checked(&pantry_ctx(&dir, conf), false).value),
            ["honey"]
        );
    }

    /// Both checks compare names ignoring case, and both report the
    /// ingredient as the *recipe* spells it.
    #[test]
    fn names_are_matched_ignoring_case_and_reported_as_the_recipe_writes_them() {
        let dir = one_recipe("Add @Salt{1%tsp} and @PEPPER{}.\n");

        let aisle = checked(&aisle_ctx(&dir, "[pantry]\nsalt\npepper\n"), true).value;
        assert_eq!(known(&aisle), ["PEPPER", "Salt"]);
        assert!(unknown(&aisle).is_empty());

        let conf = "[pantry]\nsalt = \"1%kg\"\npepper = \"50%g\"\n";
        let pantry = checked(&pantry_ctx(&dir, conf), false).value;
        assert_eq!(known(&pantry), ["PEPPER", "Salt"]);
        assert!(unknown(&pantry).is_empty());
    }

    /// The fold is Unicode's, not ASCII's. Worth pinning because it is a
    /// deliberate change: the CLI compared with `eq_ignore_ascii_case` before
    /// this moved into core, which left `Öl` reported as uncategorised however
    /// the configuration spelled it.
    #[test]
    fn a_non_ascii_name_is_matched_ignoring_case_too() {
        let dir = one_recipe("Add @Öl{1%tbsp} and @Ärter{100%g}.\n");

        let aisle = checked(&aisle_ctx(&dir, "[pantry]\növerste|öl\närter\n"), true).value;
        assert_eq!(known(&aisle), ["Ärter", "Öl"]);
        assert!(unknown(&aisle).is_empty(), "{:?}", unknown(&aisle));

        // Non-ASCII keys have to be quoted to be valid TOML.
        let conf = "[pantry]\n\"öl\" = \"1%l\"\n\"ärter\" = \"1%kg\"\n";
        assert_eq!(
            known(&checked(&pantry_ctx(&dir, conf), false).value),
            ["Ärter", "Öl"]
        );
    }

    /// Two spellings of one ingredient are two entries, because the report
    /// says what the recipes say. Both are judged the same way.
    #[test]
    fn two_spellings_of_one_ingredient_are_both_listed() {
        let dir = tempfile::TempDir::new().unwrap();
        write(&base(&dir).join("a.cook"), "Add @Salt{1%tsp}.\n");
        write(&base(&dir).join("b.cook"), "Add @salt{1%tsp}.\n");

        let coverage = checked(&aisle_ctx(&dir, "[pantry]\nsalt\n"), true).value;
        assert_eq!(known(&coverage), ["Salt", "salt"]);
        assert_eq!(coverage.total_ingredients(), 2);
    }

    /// With nothing to check against, everything is unknown — and the
    /// collection is still scanned, which is what lets `cook doctor aisle`
    /// report the count before explaining that there is no configuration.
    #[test]
    fn without_a_configuration_nothing_is_known() {
        let dir = one_recipe("Add @salt{1%tsp}.\n");
        let ctx = Context::new(base(&dir));

        for aisle in [true, false] {
            let coverage = checked(&ctx, aisle).value;
            assert_eq!(coverage.total_recipes, 1);
            assert!(known(&coverage).is_empty(), "aisle: {aisle}");
            assert_eq!(unknown(&coverage), ["salt"], "aisle: {aisle}");
        }
    }

    /// A reference is a recipe to make, not a thing to have in, so it is not
    /// an ingredient — however the configuration happens to name it.
    #[test]
    fn references_to_other_recipes_are_not_ingredients() {
        let dir = one_recipe("Make @./sauce{} and add @water{1%l}.\n");
        write(&base(&dir).join("sauce.cook"), CLEAN);

        let coverage = checked(&aisle_ctx(&dir, "[pantry]\nsauce\nwater\noil\n"), true).value;
        assert_eq!(
            all(&coverage),
            ["oil", "water"],
            "the referenced recipe's own ingredients count; the reference does not"
        );
    }

    #[test]
    fn every_recipe_under_the_root_is_scanned_including_nested_ones() {
        let dir = one_recipe("Boil @water{1%l}.\n");
        std::fs::create_dir(base(&dir).join("Breakfast")).unwrap();
        write(
            &base(&dir).join("Breakfast").join("porridge.cook"),
            "Simmer @oats{50%g}.\n",
        );

        let coverage = checked(&aisle_ctx(&dir, "[pantry]\nwater\n"), true).value;
        assert_eq!(coverage.total_recipes, 2);
        assert_eq!(
            unknown(&coverage),
            ["oats"],
            "a subdirectory must be walked"
        );
    }

    /// A recipe that cannot be parsed still counts as scanned — the CLI has
    /// always said so — but contributes no ingredients, and says why.
    #[test]
    fn a_recipe_that_cannot_be_parsed_is_counted_but_contributes_nothing() {
        let dir = one_recipe("Add @salt{1%tsp}.\n");
        write(&base(&dir).join("broken.cook"), BROKEN);

        let outcome = checked(&aisle_ctx(&dir, "[pantry]\nsalt\n"), true);
        assert_eq!(outcome.value.total_recipes, 2);
        assert_eq!(outcome.value.total_ingredients(), 1);
        assert_eq!(known(&outcome.value), ["salt"]);

        let skipped = outcome
            .diagnostics
            .iter()
            .find(|d| d.message.contains("broken.cook"))
            .unwrap_or_else(|| {
                panic!(
                    "the skipped recipe must be named: {:?}",
                    outcome.diagnostics
                )
            });
        // A warning rather than an error: the check still produced its answer,
        // and `Outcome::has_errors` must not say otherwise.
        assert_eq!(skipped.severity, Severity::Warning);
        assert!(!outcome.has_errors());
    }

    /// `cooklang-find` holds a directory's entries in a `HashMap`, so the walk
    /// order changes from run to run. Asserted over several runs because one
    /// run of an unsorted walk can come out sorted by luck.
    ///
    /// The order is by code point, **not** alphabetical: `Zucchini` sorts
    /// before `apple` because `Z` is `U+005A` and `a` is `U+0061`. The mixed
    /// case in the fixture is what holds the documented behaviour to account —
    /// an all-lowercase fixture cannot tell the two orderings apart.
    #[test]
    fn ingredients_come_back_in_code_point_order_every_time() {
        let dir = tempfile::TempDir::new().unwrap();
        for (file, ingredient) in [
            ("a", "yeast"),
            ("b", "flour"),
            ("c", "sugar"),
            ("d", "Zucchini"),
            ("e", "apple"),
            ("f", "Beetroot"),
        ] {
            write(
                &base(&dir).join(format!("{file}.cook")),
                &format!("Add @{ingredient}{{1}}.\n"),
            );
        }
        let ctx = aisle_ctx(&dir, "[pantry]\nflour\n");

        for _ in 0..8 {
            let coverage = checked(&ctx, true).value;
            assert_eq!(
                all(&coverage),
                ["Beetroot", "Zucchini", "apple", "flour", "sugar", "yeast"],
                "capitalised names sort first, as CookCLI has always printed them"
            );
            assert_eq!(
                unknown(&coverage),
                ["Beetroot", "Zucchini", "apple", "sugar", "yeast"]
            );
        }
    }

    /// The two views are derived from one list, so between them they account
    /// for every ingredient exactly once.
    #[test]
    fn the_two_views_partition_the_ingredients() {
        let dir = one_recipe("Add @salt{1%tsp}, @water{1%l} and @leek{1}.\n");
        let coverage = checked(&aisle_ctx(&dir, "[produce]\nleek\n"), true).value;

        let mut both: Vec<&str> = known(&coverage)
            .into_iter()
            .chain(unknown(&coverage))
            .collect();
        both.sort_unstable();
        assert_eq!(both, all(&coverage));
        assert_eq!(
            known(&coverage).len() + unknown(&coverage).len(),
            coverage.total_ingredients()
        );
    }

    #[test]
    fn a_coverage_base_dir_overrides_the_context_base_path() {
        let scanned = one_recipe("Add @salt{1%tsp}.\n");
        let ignored = one_recipe("Add @decoy{1}.\n");

        let coverage = aisle_coverage(
            &aisle_ctx(&ignored, "[pantry]\nsalt\n"),
            CoverageRequest {
                base_dir: Some(base(&scanned)),
            },
        )
        .expect("the check succeeds")
        .into_value();

        assert_eq!(known(&coverage), ["salt"]);
        assert!(!all(&coverage).contains(&"decoy"), "{:?}", all(&coverage));
    }

    /// A warning in the configuration is carried back rather than logged, so
    /// that a caller other than the CLI can show it.
    #[test]
    fn a_warning_in_the_aisle_configuration_comes_back_as_a_diagnostic() {
        let dir = one_recipe("Add @leek{1}.\n");
        let outcome = checked(&aisle_ctx(&dir, "[produce]\nleek\n\n[dairy]\nleek\n"), true);

        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|d| d.message.contains("Duplicate ingredient")),
            "{:?}",
            outcome.diagnostics
        );
        // ...and the check still answers.
        assert_eq!(known(&outcome.value), ["leek"]);
    }

    /// The pantry's mirror of the test above. It exists because deleting
    /// `pantry_coverage`'s `collect_diagnostics` call left every other test in
    /// this module passing: the aisle twin does not cover it.
    #[test]
    fn a_warning_in_the_pantry_configuration_comes_back_as_a_diagnostic() {
        let dir = one_recipe("Add @ice{1}.\n");
        let conf = "[freezer]\nice = { quantity = \"1%kg\", colour = \"white\" }\n";
        let outcome = checked(&pantry_ctx(&dir, conf), false);

        assert!(
            !outcome.diagnostics.is_empty(),
            "the unknown attribute must be reported"
        );
        for diagnostic in &outcome.diagnostics {
            assert_eq!(diagnostic.severity, Severity::Warning, "{diagnostic:?}");
        }
        assert!(!outcome.has_errors());
        // ...and the check still answers.
        assert_eq!(known(&outcome.value), ["ice"]);
    }

    /// A warning carries the file it came from, so a caller showing it can say
    /// which configuration to go and edit.
    #[test]
    fn a_configuration_warning_is_located_in_the_file_it_came_from() {
        let dir = one_recipe("Add @ice{1}.\n");
        let path = base(&dir).join("pantry.conf");
        write(
            &path,
            "[freezer]\nice = { quantity = \"1%kg\", colour = \"white\" }\n",
        );

        let ctx = Context::new(base(&dir)).with_pantry(ConfigSource::Path(path.clone()));
        let outcome =
            pantry_coverage(&ctx, CoverageRequest::default()).expect("the check succeeds");

        let located = outcome
            .diagnostics
            .iter()
            .find(|d| d.location.is_some())
            .unwrap_or_else(|| panic!("expected a located warning: {:?}", outcome.diagnostics));
        assert_eq!(
            located.location.as_ref().and_then(|l| l.file.as_deref()),
            Some(path.as_path())
        );
    }

    /// A configuration the context names but cannot read is a failure, not an
    /// absent configuration: reporting it as "nothing is categorised" would
    /// send the user editing a file that is fine.
    #[test]
    fn a_configuration_that_cannot_be_read_is_reported() {
        let dir = one_recipe("Add @salt{1%tsp}.\n");
        let missing = base(&dir).join("config").join("aisle.conf");

        match aisle_coverage(
            &Context::new(base(&dir)).with_aisle(ConfigSource::Path(missing.clone())),
            CoverageRequest::default(),
        ) {
            Err(CoreError::Io { path, source }) => {
                assert_eq!(path, missing);
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected CoreError::Io, got {:?}", other.map(|o| o.value)),
        }
    }

    /// The same verdict *and the same wording* `pantry::load` reaches on the
    /// same file. Asserted against `load`'s own answer rather than a literal,
    /// because the point is that the two agree: a user told two different
    /// things about one file by two commands has to work out which is true.
    #[test]
    fn a_pantry_that_cannot_be_parsed_at_all_is_reported_as_pantry_load_reports_it() {
        let dir = one_recipe("Add @salt{1%tsp}.\n");
        let ctx = pantry_ctx(&dir, "this is not toml [");

        let from_load = match crate::pantry::load(&ctx) {
            Err(CoreError::Config { message, .. }) => message,
            other => panic!("expected CoreError::Config from load, got {other:?}"),
        };

        match pantry_coverage(&ctx, CoverageRequest::default()) {
            Err(CoreError::Config { path, message }) => {
                assert_eq!(path, None, "an inline configuration has no path");
                // The parser's own cause, not a constant: without this the
                // message degrades to "could not be parsed" and nobody notices.
                assert!(
                    message.contains("TOML parse error"),
                    "the cause must survive: {message}"
                );
                assert_eq!(message, from_load, "the two commands must agree");
            }
            other => panic!(
                "expected CoreError::Config, got {:?}",
                other.map(|o| o.value)
            ),
        }
    }

    /// A root that is not there fails the same way validation does.
    #[test]
    fn a_root_that_does_not_exist_is_reported_by_a_coverage_check() {
        let dir = tempfile::TempDir::new().unwrap();
        let missing = base(&dir).join("nope");

        match pantry_coverage(
            &Context::new(missing.clone()).with_pantry(ConfigSource::Inline(String::new())),
            CoverageRequest::default(),
        ) {
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
    fn an_empty_collection_covers_nothing() {
        let dir = tempfile::TempDir::new().unwrap();
        let coverage = checked(&aisle_ctx(&dir, "[pantry]\nsalt\n"), true).value;
        assert_eq!(coverage.total_recipes, 0);
        assert_eq!(coverage.total_ingredients(), 0);
        assert!(known(&coverage).is_empty());
        assert!(unknown(&coverage).is_empty());
    }
}
