//! Pantry queries: the read half of `cook pantry`.
//!
//! [`load`] reads the configuration [`Context::pantry`] points at — a file, or
//! text an editor is holding — and the queries answer questions about it:
//! everything in it ([`list`]), what is running out ([`depleted`]), what is
//! about to go off ([`expiring`]), and which recipes it can already cook
//! ([`recipes`]).
//!
//! [`plan`] is the odd one out: it answers "what should I stock?" by looking at
//! the recipe collection alone, and never reads the pantry at all.
//!
//! Nothing here writes. Adding, removing and updating items still lives in the
//! CLI.

use crate::{
    diagnostic::Severity,
    find::tree_error,
    parser::{collect_diagnostics, parse_unscaled},
    Context, CoreError, Diagnostic, Outcome,
};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{Local, NaiveDate};
use cooklang::Recipe;
use cooklang_find::{RecipeEntry, RecipeTree};
use regex::Regex;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};

/// How [`ExpiringItem::expire_date`] is written, whatever the file used.
const ISO_DATE: &str = "%Y-%m-%d";

// ---------------------------------------------------------------------------
// What a pantry holds
// ---------------------------------------------------------------------------

/// One item in the pantry.
///
/// Every field is carried as it was written, without interpretation: a
/// quantity is `"500%g"` rather than a number and a unit, and a date is
/// whatever spelling the file used. The methods below are where interpretation
/// happens.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PantryItem {
    /// The ingredient's name, as written.
    pub name: String,
    /// The section it was written under. `cooklang` collects items written
    /// above the first section header into a section called `general`, so an
    /// item always has one.
    pub section: String,
    /// How much is in stock — `"500%g"`, `"2"` — or `None` for an item written
    /// without a quantity.
    pub quantity: Option<String>,
    /// When it was bought, if the file says.
    pub bought: Option<String>,
    /// When it expires, if the file says. See [`expiring`] for the spellings
    /// that can be read as a date.
    pub expire: Option<String>,
    /// The quantity at or below which this item counts as low, if the file
    /// sets one. See [`is_low`](PantryItem::is_low).
    pub low: Option<String>,
}

impl PantryItem {
    /// True when the stock has fallen to or below this item's *own* `low`
    /// threshold.
    ///
    /// False unless [`quantity`](PantryItem::quantity) and
    /// [`low`](PantryItem::low) are both set, both parse as a number with an
    /// optional unit, and those units are equal: a threshold written in
    /// different units from the stock is not compared, and neither is a
    /// quantity that is not a number. Such an item is not "not low" so much as
    /// unanswerable, and [`depleted`] falls back to its built-in thresholds
    /// for it.
    ///
    /// Computed rather than stored, so it cannot disagree with the fields it
    /// reads. The comparison is `cooklang`'s own, reached by handing it an
    /// equivalent item, so that there is one definition of it rather than two
    /// that can drift.
    pub fn is_low(&self) -> bool {
        cooklang::pantry::PantryItem::WithAttributes(cooklang::pantry::ItemWithAttributes {
            name: self.name.clone(),
            bought: None,
            expire: None,
            quantity: self.quantity.clone(),
            low: self.low.clone(),
        })
        .is_low()
    }

    fn from_cooklang(section: &str, item: &cooklang::pantry::PantryItem) -> Self {
        Self {
            name: item.name().to_string(),
            section: section.to_string(),
            quantity: item.quantity().map(ToOwned::to_owned),
            bought: item.bought().map(ToOwned::to_owned),
            expire: item.expire().map(ToOwned::to_owned),
            low: item.low().map(ToOwned::to_owned),
        }
    }
}

/// One section of the pantry, with the items written under it.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PantrySection {
    /// The section's name, as written. Equal to the
    /// [`section`](PantryItem::section) of every item in it, which is carried
    /// on the items too so that a single item taken out of here still says
    /// where it came from.
    pub name: String,
    /// The items written under it, in file order. May be empty, for a section
    /// header with nothing under it.
    pub items: Vec<PantryItem>,
}

/// A whole pantry configuration.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PantryContents {
    /// Every section, in the order the file wrote them.
    pub sections: Vec<PantrySection>,
}

impl PantryContents {
    /// Every item in every section, in file order.
    pub fn items(&self) -> impl Iterator<Item = &PantryItem> {
        self.sections.iter().flat_map(|section| &section.items)
    }

    fn from_conf(conf: &cooklang::pantry::PantryConf) -> Self {
        // `PantryConf::sections` is an `IndexMap`, so this is the file's own
        // order rather than an arbitrary one.
        Self {
            sections: conf
                .sections
                .iter()
                .map(|(name, items)| PantrySection {
                    name: name.clone(),
                    items: items
                        .iter()
                        .map(|item| PantryItem::from_cooklang(name, item))
                        .collect(),
                })
                .collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Read and parse the pantry configuration [`Context::pantry`] names.
///
/// Reads through [`ConfigSource`](crate::ConfigSource), so an editor can hand
/// over pantry text it has not saved instead of a path.
///
/// A pantry that parses with warnings — an unknown attribute on an item, say —
/// is a successful load carrying those warnings as [`Outcome::diagnostics`],
/// located in the pantry file when it came from one.
///
/// # Errors
///
/// - [`CoreError::MissingConfig`] if the context carries no pantry at all.
///   Every query below reports this the same way, because a pantry query with
///   no pantry has no answer — unlike `shopping_list::generate`, which simply
///   subtracts nothing.
/// - [`CoreError::Io`] if a path-backed configuration cannot be read.
/// - [`CoreError::Config`] if it cannot be parsed at all, naming the file it
///   came from.
pub fn load(ctx: &Context) -> Result<Outcome<PantryContents>, CoreError> {
    let source = ctx.pantry();
    let Some(text) = source.read()? else {
        return Err(CoreError::MissingConfig {
            kind: "pantry".to_string(),
        });
    };
    let path = source.path();
    tracing::trace!("loading pantry from {:?}", path);

    let parsed = cooklang::pantry::parse_lenient(&text);
    let diagnostics = collect_diagnostics(parsed.report(), path);

    match parsed.output() {
        Some(conf) => Ok(Outcome::with_diagnostics(
            PantryContents::from_conf(conf),
            diagnostics,
        )),
        None => Err(CoreError::Config {
            path: path.map(ToOwned::to_owned),
            message: parse_failure(&diagnostics),
        }),
    }
}

/// Word the failure that left `parse_lenient` with no configuration at all.
///
/// The causes are flattened onto one line because [`CoreError`]'s `Display` is
/// documented as being one: a TOML syntax error arrives as a multi-line report
/// with the offending line quoted underneath it.
fn parse_failure(diagnostics: &[Diagnostic]) -> String {
    let causes: Vec<String> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| one_line(&d.message))
        .collect();

    if causes.is_empty() {
        "the pantry configuration could not be parsed".to_string()
    } else {
        causes.join("; ")
    }
}

/// Collapse a multi-line message onto one line, keeping every part of it.
fn one_line(message: &str) -> String {
    message
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

/// Which of the pantry to list.
///
/// Not `#[non_exhaustive]`: consumers construct this. `..Default::default()`
/// keeps a literal working if it grows a field.
#[derive(Debug, Clone, Default)]
pub struct ListRequest {
    /// Keep only this section, compared ignoring ASCII case. `None` lists
    /// everything.
    pub section: Option<String>,
}

/// List the pantry, optionally narrowed to one section.
///
/// A filter that matches no section gives an empty list rather than an error:
/// core reports what is there, and whether "you asked for a section that does
/// not exist" deserves an error is the caller's policy. The CLI treats it as
/// one.
///
/// # Errors
///
/// As [`load`].
pub fn list(ctx: &Context, req: ListRequest) -> Result<Outcome<PantryContents>, CoreError> {
    let mut outcome = load(ctx)?;
    if let Some(section) = &req.section {
        outcome
            .value
            .sections
            .retain(|s| s.name.eq_ignore_ascii_case(section));
    }
    Ok(outcome)
}

// ---------------------------------------------------------------------------
// depleted
// ---------------------------------------------------------------------------

/// Which items count as running out.
///
/// Not `#[non_exhaustive]`: consumers construct this.
#[derive(Debug, Clone, Default)]
pub struct DepletedRequest {
    /// Also return items whose stock cannot be judged at all — no quantity, or
    /// a quantity that is not a number, or a `low` threshold in units that
    /// cannot be compared with it.
    pub all: bool,
}

/// The items that are low or out of stock, in file order.
///
/// An item is returned when [`PantryItem::is_low`] says so. When it does not,
/// the answer depends on what there is to go on:
///
/// - No quantity at all: only with [`DepletedRequest::all`], since there is
///   nothing to compare.
/// - A quantity, and a `low` threshold in the same units: `is_low` has already
///   compared them and said no, so only with [`DepletedRequest::all`].
/// - A quantity, and either no threshold or one in units that do not match:
///   the built-in thresholds decide — at or below 100 for `g` and `ml`, below
///   0.5 for `kg` and `l`, at or below 1 for anything else, including a bare
///   count.
///
/// # Errors
///
/// As [`load`].
pub fn depleted(
    ctx: &Context,
    req: DepletedRequest,
) -> Result<Outcome<Vec<PantryItem>>, CoreError> {
    let outcome = load(ctx)?;
    let items = outcome
        .value
        .items()
        .filter(|item| is_depleted(item, req.all))
        .cloned()
        .collect();
    Ok(Outcome::with_diagnostics(items, outcome.diagnostics))
}

/// The rule documented on [`depleted`].
fn is_depleted(item: &PantryItem, all: bool) -> bool {
    if item.is_low() {
        return true;
    }
    match &item.quantity {
        None => all,
        Some(quantity) => match &item.low {
            // A threshold in matching units has already been compared by
            // `is_low` above, and it said no.
            Some(low) if units_match(quantity, low) => all,
            _ => is_low_quantity(quantity),
        },
    }
}

// ---------------------------------------------------------------------------
// expiring
// ---------------------------------------------------------------------------

/// How far ahead to look for expiring items.
///
/// Not `#[non_exhaustive]`: consumers construct this.
#[derive(Debug, Clone)]
pub struct ExpiringRequest {
    /// How many days ahead to look. `0` returns only what has expired or
    /// expires today.
    pub days: u32,
    /// Also return items with no readable expiry date, which carry no
    /// [`ExpiringItem::days_until_expiry`].
    pub include_unknown: bool,
}

impl Default for ExpiringRequest {
    /// A week ahead, which is `cook pantry expiring`'s default, and only items
    /// that say when they expire.
    fn default() -> Self {
        Self {
            days: 7,
            include_unknown: false,
        }
    }
}

/// A pantry item that is expiring, with the arithmetic already done.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiringItem {
    /// The item itself, exactly as [`load`] read it.
    pub item: PantryItem,
    /// Its expiry date normalised to ISO 8601 (`2025-06-01`), whichever
    /// spelling the file used. `None` only for an item included by
    /// [`ExpiringRequest::include_unknown`].
    pub expire_date: Option<String>,
    /// Days from today until it expires: `0` today, negative once it has
    /// expired. `None` alongside an absent `expire_date`.
    pub days_until_expiry: Option<i64>,
}

/// The items expiring within [`ExpiringRequest::days`] of today, soonest
/// first.
///
/// Already-expired items are included, and sort first because their
/// [`days_until_expiry`](ExpiringItem::days_until_expiry) is negative. Items
/// with no readable date sort last, and are only present at all with
/// [`ExpiringRequest::include_unknown`].
///
/// # Dates
///
/// A date is read as `%Y-%m-%d`, `%d.%m.%Y`, `%d/%m/%Y`, `%m/%d/%Y`,
/// `%Y.%m.%d` or `%d-%m-%Y`, in that order — so `01/02/2025` is the 1st of
/// February, not the 2nd of January. Anything else counts as no date at all.
///
/// "Today" is the local date of the machine this runs on.
///
/// # Errors
///
/// As [`load`].
pub fn expiring(
    ctx: &Context,
    req: ExpiringRequest,
) -> Result<Outcome<Vec<ExpiringItem>>, CoreError> {
    let outcome = load(ctx)?;
    let items = expiring_on(&outcome.value, &req, Local::now().date_naive());
    Ok(Outcome::with_diagnostics(items, outcome.diagnostics))
}

/// [`expiring`] against a given date, so that the arithmetic can be tested
/// without the answer depending on the day the tests are run.
fn expiring_on(
    contents: &PantryContents,
    req: &ExpiringRequest,
    today: NaiveDate,
) -> Vec<ExpiringItem> {
    // A `days` big enough to run off the end of the calendar means every date
    // is within it. Saturating rather than panicking matters in a crate a NAPI
    // addon calls: `cook pantry expiring -d 4294967295` used to panic here.
    let threshold = today
        .checked_add_signed(chrono::Duration::days(i64::from(req.days)))
        .unwrap_or(NaiveDate::MAX);

    let mut items: Vec<ExpiringItem> = contents
        .items()
        .filter_map(|item| match item.expire.as_deref().and_then(parse_date) {
            Some(date) if date <= threshold => Some(ExpiringItem {
                item: item.clone(),
                expire_date: Some(date.format(ISO_DATE).to_string()),
                days_until_expiry: Some((date - today).num_days()),
            }),
            // Expires, but not yet.
            Some(_) => None,
            None if req.include_unknown => Some(ExpiringItem {
                item: item.clone(),
                expire_date: None,
                days_until_expiry: None,
            }),
            None => None,
        })
        .collect();

    // Stable, so items expiring on the same day stay in file order.
    items.sort_by_key(|item| item.days_until_expiry.unwrap_or(i64::MAX));
    items
}

// ---------------------------------------------------------------------------
// recipes
// ---------------------------------------------------------------------------

/// How complete a match has to be to be worth reporting.
///
/// Not `#[non_exhaustive]`: consumers construct this.
#[derive(Debug, Clone)]
pub struct RecipesRequest {
    /// The lowest percentage of a recipe's ingredients that may be in stock
    /// for it to count as a partial match, as a whole number out of 100.
    pub threshold: u8,
}

impl Default for RecipesRequest {
    /// 75%, which is `cook pantry recipes`'s default.
    fn default() -> Self {
        Self { threshold: 75 }
    }
}

/// A recipe most of whose ingredients are in stock.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartialMatch {
    /// The recipe's title, or its file stem when it has none.
    pub name: String,
    /// What percentage of its ingredients are in stock, rounded down.
    pub percentage: usize,
    /// The ingredients that are not, lowercased as they were compared, in
    /// alphabetical order.
    pub missing: Vec<String>,
}

/// What the pantry can cook.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecipeMatches {
    /// Recipes every one of whose ingredients is in stock, by title, in
    /// alphabetical order.
    pub full: Vec<String>,
    /// Recipes that are only partly covered, at or above
    /// [`RecipesRequest::threshold`], in alphabetical order.
    pub partial: Vec<PartialMatch>,
}

/// Work out which recipes under [`Context::base_path`] the pantry can cook.
///
/// An ingredient counts as in stock when its name matches a pantry item's,
/// compared lowercased and otherwise exactly — no unit or quantity is
/// considered, so a recipe needing a kilo of flour matches a pantry holding a
/// gram of it. References to other recipes are ignored, and a recipe left with
/// no ingredients at all matches nothing. See [`listed_ingredients`] for what
/// else is left out.
///
/// Recipes are found by walking the collection, `.menu` files included. A
/// recipe that cannot be read or parsed is left out, with a warning in
/// [`Outcome::diagnostics`] naming it — it is not counted as a match or a
/// miss. Warnings from recipes that *did* parse are not reported here, because
/// they cannot change the answer; `doctor::validate` is the command for those.
///
/// # Errors
///
/// - As [`load`], since this needs the pantry.
/// - [`CoreError::Search`] if the collection cannot be walked, and
///   [`CoreError::Io`] if a file in it cannot be listed — as
///   `doctor::validate`.
pub fn recipes(ctx: &Context, req: RecipesRequest) -> Result<Outcome<RecipeMatches>, CoreError> {
    let loaded = load(ctx)?;
    let mut diagnostics = loaded.diagnostics;
    let stocked: BTreeSet<String> = loaded
        .value
        .items()
        .map(|item| item.name.to_lowercase())
        .collect();

    let tree = build_tree(ctx.base_path())?;
    let mut matches = RecipeMatches::default();

    for entry in walk(&tree) {
        let Some(recipe) = parse_or_skip(entry, &mut diagnostics) else {
            continue;
        };
        // Lowercased into the set, so a recipe naming `Salt` and `salt` wants
        // one ingredient rather than two.
        let wanted: BTreeSet<String> = listed_ingredients(&recipe)
            .iter()
            .map(|name| name.to_lowercase())
            .collect();
        if wanted.is_empty() {
            continue;
        }

        let available = wanted.iter().filter(|name| stocked.contains(*name)).count();
        let percentage = available * 100 / wanted.len();
        let name = recipe_name(entry);

        if available == wanted.len() {
            matches.full.push(name);
        } else if percentage >= usize::from(req.threshold) {
            matches.partial.push(PartialMatch {
                name,
                percentage,
                // From a `BTreeSet`, so already in order.
                missing: wanted
                    .iter()
                    .filter(|name| !stocked.contains(*name))
                    .cloned()
                    .collect(),
            });
        }
    }

    // The walk yields directories in a `HashMap`'s order, which changes
    // between runs. Sorting is what makes the answer the same twice running.
    matches.full.sort();
    matches.partial.sort();

    Ok(Outcome::with_diagnostics(matches, diagnostics))
}

// ---------------------------------------------------------------------------
// plan
// ---------------------------------------------------------------------------

/// How far to take a pantry plan.
///
/// Not `#[non_exhaustive]`: consumers construct this. The default plans until
/// every recipe is covered.
#[derive(Debug, Clone, Default)]
pub struct PlanRequest {
    /// Stop after this many ingredients. `None` continues until every recipe
    /// is cookable, or until no ingredient is left to add.
    pub max_ingredients: Option<usize>,
    /// Count a recipe as cookable while it is still missing this many
    /// ingredients. `0` means everything it needs must be stocked.
    pub allow_missing: usize,
}

/// One ingredient to buy, and what buying it achieves.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngredientStep {
    /// The ingredient, as recipes write it.
    pub name: String,
    /// How many more recipes become cookable once it is in stock.
    pub new_recipes_unlocked: usize,
    /// How many recipes are cookable in total by this point — this step and
    /// every step before it.
    pub total_cookable: usize,
}

/// An order to stock a pantry in.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PantryPlan {
    /// The ingredients to buy, most useful first.
    pub steps: Vec<IngredientStep>,
    /// How many recipes the plan was worked out over: every recipe in the
    /// collection that lists at least one ingredient.
    pub total_recipes: usize,
}

impl PantryPlan {
    /// How many recipes are cookable once the whole plan is stocked.
    ///
    /// Read off the last step rather than stored alongside it, so it cannot
    /// disagree with the steps it summarises. Zero for an empty plan.
    pub fn cookable_recipes(&self) -> usize {
        self.steps.last().map_or(0, |step| step.total_cookable)
    }

    /// [`cookable_recipes`](PantryPlan::cookable_recipes) as a percentage of
    /// [`total_recipes`](PantryPlan::total_recipes), rounded down. Zero when
    /// there are no recipes at all.
    pub fn coverage_percentage(&self) -> usize {
        self.cookable_recipes() * 100 / self.total_recipes.max(1)
    }
}

/// Work out which ingredients to stock to cook as much of the collection as
/// possible.
///
/// **The pantry is not consulted.** This answers "what should I buy?" from the
/// recipes under [`Context::base_path`] alone, so it needs no pantry
/// configuration and will happily recommend something already in stock.
///
/// The plan is greedy: at each step it takes the ingredient wanted by the most
/// recipes that are not yet cookable, and ties are broken alphabetically. That
/// is an approximation — a greedy set cover is not guaranteed to be the
/// shortest plan — but it is deterministic, which the tie-break is there for.
///
/// Only `.cook` files are considered; `.menu` files are skipped, and so are
/// recipes that list no ingredients — and, as in [`recipes`], any that cannot
/// be read or parsed, each with a warning in [`Outcome::diagnostics`].
/// Ingredients are those of [`listed_ingredients`], compared exactly as
/// recipes write them, so `Flour` and `flour` are two ingredients — unlike
/// [`recipes`], which lowercases.
///
/// # Errors
///
/// [`CoreError::Search`] if the collection cannot be walked, and
/// [`CoreError::Io`] if a file in it cannot be listed. Never
/// [`CoreError::MissingConfig`].
pub fn plan(ctx: &Context, req: PlanRequest) -> Result<Outcome<PantryPlan>, CoreError> {
    let tree = build_tree(ctx.base_path())?;
    let mut diagnostics = Vec::new();

    // What each recipe still needs. A recipe drops out once it is cookable.
    let mut missing: Vec<BTreeSet<String>> = walk(&tree)
        .into_iter()
        .filter(|entry| !entry.is_menu())
        .filter_map(|entry| parse_or_skip(entry, &mut diagnostics))
        .map(|recipe| listed_ingredients(&recipe))
        .filter(|ingredients| !ingredients.is_empty())
        .collect();

    let total_recipes = missing.len();
    let max_ingredients = req.max_ingredients.unwrap_or(usize::MAX);
    let mut steps: Vec<IngredientStep> = Vec::new();
    let mut cookable = 0;

    while cookable < total_recipes && steps.len() < max_ingredients {
        let Some(best) = most_wanted(&missing) else {
            // Nothing left to choose: every remaining recipe wants nothing,
            // which `allow_missing` cannot satisfy.
            break;
        };

        let mut newly_cookable = 0;
        missing.retain_mut(|wanted| {
            wanted.remove(&best);
            if wanted.len() <= req.allow_missing {
                newly_cookable += 1;
                false
            } else {
                true
            }
        });
        cookable += newly_cookable;

        steps.push(IngredientStep {
            name: best,
            new_recipes_unlocked: newly_cookable,
            total_cookable: cookable,
        });
    }

    Ok(Outcome::with_diagnostics(
        PantryPlan {
            steps,
            total_recipes,
        },
        diagnostics,
    ))
}

/// The ingredient wanted by the most recipes, ties broken alphabetically.
fn most_wanted(missing: &[BTreeSet<String>]) -> Option<String> {
    let mut scores: BTreeMap<&str, usize> = BTreeMap::new();
    for wanted in missing {
        for ingredient in wanted {
            *scores.entry(ingredient.as_str()).or_insert(0) += 1;
        }
    }
    scores
        // Highest count wins; `Reverse` on the name turns "largest" into
        // "alphabetically first" for the tie, which is what makes two runs
        // over the same collection agree.
        .into_iter()
        .max_by_key(|&(name, count)| (count, Reverse(name)))
        .map(|(name, _)| name.to_string())
}

// ---------------------------------------------------------------------------
// Walking a collection
// ---------------------------------------------------------------------------

/// Build the recipe tree under `base_dir`, in this crate's error wording.
fn build_tree(base_dir: &Utf8Path) -> Result<RecipeTree, CoreError> {
    tracing::trace!("walking recipes under {base_dir}");
    cooklang_find::build_tree(base_dir).map_err(|e| tree_error(e, base_dir))
}

/// Every recipe in the tree, depth first, in path order.
///
/// Sorted because `cooklang-find` holds a directory's children in a `HashMap`,
/// so the walk itself yields them differently from run to run. Neither result
/// depends on this — [`recipes`] sorts what it returns and [`plan`] breaks its
/// ties alphabetically — but the diagnostics do: without it, a collection with
/// two unreadable recipes reports them in a different order each time.
fn walk(tree: &RecipeTree) -> Vec<&RecipeEntry> {
    fn collect<'a>(tree: &'a RecipeTree, out: &mut Vec<&'a RecipeEntry>) {
        if let Some(entry) = &tree.recipe {
            out.push(entry);
        }
        for subtree in tree.children.values() {
            collect(subtree, out);
        }
    }

    let mut entries = Vec::new();
    collect(tree, &mut entries);
    entries.sort_by_key(|entry| (entry.path().cloned(), entry.name().clone()));
    entries
}

/// Parse one recipe, or note that it was left out.
///
/// Nothing here fails the walk: one unreadable file in a collection must not
/// cost the caller the answer for the rest of it.
fn parse_or_skip(entry: &RecipeEntry, diagnostics: &mut Vec<Diagnostic>) -> Option<Recipe> {
    let display = entry
        .path()
        .map(ToString::to_string)
        .or_else(|| entry.name().clone())
        .unwrap_or_else(|| "unknown".to_string());

    let mut skipped = |reason: &str| {
        let diagnostic = Diagnostic::warning(format!(
            "could not {reason} {display}, so it was not considered"
        ));
        diagnostics.push(match entry.path() {
            Some(path) => diagnostic.at_file(path),
            None => diagnostic,
        });
        None
    };

    let content = match entry.content() {
        Ok(content) => content,
        Err(_) => return skipped("read"),
    };

    // Unscaled: scaling by one would only re-fit units, and nothing here reads
    // a quantity.
    match parse_unscaled(&content, &display, entry.path().map(Utf8PathBuf::as_path)) {
        Ok(outcome) => Some(outcome.value),
        Err(_) => skipped("parse"),
    }
}

/// The ingredients a recipe asks the reader to have, as it writes them.
///
/// A set, so a recipe using flour twice wants flour once.
///
/// References to other recipes are left out: they are a recipe to make, not a
/// thing to have in. Ingredients the parser marks as not to be listed are left
/// out too, though with [`PARSER`](crate::PARSER)'s extensions there are none
/// — `@-salt{}` parses as an ingredient *named* `-salt` rather than a hidden
/// one, so it counts like any other. The filter is kept for the day that
/// changes.
fn listed_ingredients(recipe: &Recipe) -> BTreeSet<String> {
    recipe
        .ingredients
        .iter()
        .filter(|ingredient| ingredient.reference.is_none())
        .filter(|ingredient| ingredient.modifiers().should_be_listed())
        .map(|ingredient| ingredient.display_name().to_string())
        .collect()
}

/// What to call a recipe in the results: its title, or its file stem.
///
/// The fallback is `cooklang-find`'s job and it always manages one for a
/// file-backed entry, so "unknown" is unreachable through a walk. Kept because
/// dropping a nameless recipe from the results would be worse than naming it
/// badly.
fn recipe_name(entry: &RecipeEntry) -> String {
    entry
        .name()
        .clone()
        .unwrap_or_else(|| "unknown".to_string())
}

// ---------------------------------------------------------------------------
// Reading quantities and dates
// ---------------------------------------------------------------------------

/// A quantity as pantry files write it: a number, an optional `%`, then an
/// optional unit. The unit group matches the empty string, so a bare count
/// parses with no unit rather than failing.
static QUANTITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d+(?:\.\d+)?)\s*%?\s*(.*)$").expect("the quantity pattern is valid")
});

/// The unit of a quantity, lowercased, or `None` if it is not a quantity at
/// all. A bare count has an empty unit.
fn unit_of(quantity: &str) -> Option<String> {
    QUANTITY
        .captures(quantity)
        .map(|captures| captures[2].to_lowercase())
}

/// Whether two quantities are written in the same unit, and so can be
/// compared. False if either is not a quantity.
fn units_match(quantity: &str, low_threshold: &str) -> bool {
    match (unit_of(quantity), unit_of(low_threshold)) {
        (Some(quantity), Some(threshold)) => quantity == threshold,
        _ => false,
    }
}

/// The built-in "running out" thresholds, for items that set none of their
/// own. False for anything that is not a quantity.
fn is_low_quantity(quantity: &str) -> bool {
    let Some(captures) = QUANTITY.captures(quantity) else {
        return false;
    };
    let Ok(amount) = captures[1].parse::<f64>() else {
        return false;
    };

    match captures[2].to_lowercase().as_str() {
        "g" | "ml" => amount <= 100.0,
        "kg" | "l" => amount < 0.5,
        // A bare count, `item`, `items`, and every unit not listed above.
        _ => amount <= 1.0,
    }
}

/// The date spellings a pantry file may use, tried in this order.
const DATE_FORMATS: [&str; 6] = [
    "%Y-%m-%d", "%d.%m.%Y", "%d/%m/%Y", "%m/%d/%Y", "%Y.%m.%d", "%d-%m-%Y",
];

/// Read a date in any of [`DATE_FORMATS`], or `None`.
fn parse_date(date: &str) -> Option<NaiveDate> {
    DATE_FORMATS
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(date, format).ok())
}

#[cfg(test)]
mod tests;
