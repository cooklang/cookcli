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

//! Aggregating several recipes into one shopping list.
//!
//! [`generate`] is the whole command: it loads the aisle and pantry
//! configuration from the [`Context`], expands each recipe's references,
//! merges duplicate ingredients, subtracts what the pantry already holds, and
//! returns an [`AggregatedList`].
//!
//! [`extract_ingredients`] is the accumulation step on its own, for callers
//! that build a list incrementally — the web server adds one recipe at a time
//! and picks which references to follow per recipe.
//!
//! [`ShoppingListStore`] is the other half: the `.shopping-list` and
//! `.shopping-checked` files that remember which recipes someone put on their
//! list and what they have already ticked off while shopping.

mod store;

pub use store::{recipe_display_name, ShoppingListStore, StoredEntry};

use crate::{
    find,
    format::{quantity::ordered_components, shopping_list::quantity_fmt},
    parser::{parse_recipe_at, parse_unscaled, PARSER},
    ConfigSource, Context, CoreError, Diagnostic, Outcome, RecipeSource,
};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::{
    aisle::AisleConf, convert::Converter, ingredient_list::IngredientList, pantry::PantryConf,
    quantity::GroupedQuantity, quantity::Value, Recipe,
};
use cooklang_find::RecipeEntry;
use serde::{Deserialize, Serialize};

/// Where to find the aisle configuration format, quoted when there is none.
const AISLE_DOCS: &str = "https://cooklang.org/docs/spec/#shopping-lists";

/// One recipe to include in a shopping list, with its scaling factor.
///
/// The recipe is a [`RecipeSource`], matching
/// [`recipe::read`](crate::recipe::read): a [`RecipeSource::Path`] is looked up
/// under [`Context::base_path`], and a [`RecipeSource::Content`] is used as it
/// stands, so an editor can put an unsaved buffer on a shopping list.
///
/// CookCLI's `name:factor` argument spelling is a *command-line* convention
/// rather than a property of a recipe name, so it is not parsed here — callers
/// that accept arguments in that form split them with
/// [`split_name_and_scale`](crate::recipe::split_name_and_scale) first.
///
/// # In-memory recipes and their references
///
/// Only the recipe *itself* comes from memory. A [`RecipeSource::Content`]
/// recipe that references another recipe (`@./sauce{}`) still has that
/// reference resolved from disk under [`Context::base_path`], because a
/// reference names a file and nothing in this API carries a second buffer to
/// resolve it against. So a buffer whose references all exist on disk works;
/// a wholly in-memory recipe *graph* does not, and a reference that names an
/// unsaved file fails with [`CoreError::RecipeNotFound`] exactly as a path
/// recipe would.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaledRecipe {
    /// Where the recipe comes from.
    pub source: RecipeSource,
    /// Scaling factor applied to the recipe's quantities. Pass `1.0` to leave
    /// them alone.
    pub scale: f64,
}

impl ScaledRecipe {
    /// A recipe at its authored scale.
    pub fn new(source: RecipeSource) -> Self {
        Self { source, scale: 1.0 }
    }

    /// A recipe scaled by `scale`.
    pub fn scaled(source: RecipeSource, scale: f64) -> Self {
        Self { source, scale }
    }
}

/// What to put on the shopping list.
#[derive(Debug, Clone, Default)]
pub struct GenerateRequest {
    /// The recipes to include, each with its own scaling factor.
    pub recipes: Vec<ScaledRecipe>,
    /// Leave recipes referenced from within a recipe unexpanded.
    ///
    /// This does not drop the reference: it stays on the list as an item named
    /// after the referenced recipe, with no quantity.
    pub ignore_references: bool,
    /// Extra items to add to the list that no recipe calls for — the paper
    /// towels and bin bags that belong on the same shopping trip.
    ///
    /// Each entry is a Cooklang ingredient written without its leading `@`: a
    /// bare name (`paper towels`) for something with no amount, or the brace
    /// form (`flour{200%g}`, `eggs{12}`) to give one. They are aggregated with
    /// the recipe ingredients rather than kept apart, so an extra item sharing
    /// a name with one a recipe already asks for merges into it, lands in its
    /// aisle category, and is subtracted from by the pantry — exactly as that
    /// ingredient would be.
    pub extra_items: Vec<String>,
}

/// How [`extract_ingredients`] should treat recipe references.
#[derive(Debug, Clone, Default)]
pub struct ExtractOptions<'a> {
    /// Leave referenced recipes unexpanded. See
    /// [`GenerateRequest::ignore_references`].
    pub ignore_references: bool,
    /// Which references to follow, by their display path (`sauces/tomato`).
    /// `None` follows all of them. Ignored when `ignore_references` is set.
    ///
    /// Both sides of the comparison are put through
    /// [`find::resolve_reference`] first, so a reference written `@./sauce{}`
    /// is named by `./sauce` and by `sauce` alike, and one written
    /// `@../shared/sauce{}` by the `shared/sauce` it resolves to.
    pub included_references: Option<&'a [String]>,
}

/// One ingredient on a shopping list.
///
/// `#[non_exhaustive]` because this is an output type that consumers read
/// rather than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    /// The ingredient's name, after aisle synonyms have been folded onto the
    /// configuration's preferred spelling.
    pub name: String,
    /// How much to buy, rendered the way the human and markdown output show it
    /// — `"200 g"`, or just `"3"` for an ingredient with no unit.
    ///
    /// More than one entry when the recipes asked for units that do not
    /// convert into each other (`200 g` of flour plus `1 cup` of flour), and
    /// empty when no recipe gave a quantity at all. Several entries are ordered
    /// by unit name with the unitless one first, as
    /// [`ordered_components`](crate::format::quantity::ordered_components)
    /// describes.
    pub quantities: Vec<String>,
}

/// A group of ingredients sharing an aisle category.
///
/// `#[non_exhaustive]` because this is an output type that consumers read
/// rather than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListCategory {
    /// The category's name, as spelled in the aisle configuration. Ingredients
    /// the configuration does not mention land in a trailing `"other"`.
    pub name: String,
    /// The ingredients in this category.
    pub items: Vec<ListItem>,
}

/// A finished shopping list.
///
/// Both views describe the same ingredients: [`items`](Self::items) in the
/// order the recipes introduced them, and [`categories`](Self::categories)
/// grouped and ordered by the aisle configuration. Which one to show is the
/// caller's choice — CookCLI's `--plain` picks the former.
///
/// `#[non_exhaustive]` because this is an output type that consumers read
/// rather than construct.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize)]
pub struct AggregatedList {
    /// Every ingredient, uncategorised, in the order the recipes introduced
    /// them.
    pub items: Vec<ListItem>,
    /// The same ingredients grouped by aisle category. Empty categories are
    /// dropped, so this is empty when the list is.
    pub categories: Vec<ListCategory>,

    /// The same as [`items`](Self::items), keeping `cooklang`'s own quantity
    /// model. The formatters in [`crate::format::shopping_list`] need the
    /// structured values; consumers outside this crate get the rendered
    /// strings above.
    #[serde(skip)]
    pub(crate) raw_items: Vec<(String, GroupedQuantity)>,
    /// The same as [`categories`](Self::categories), likewise unrendered.
    #[serde(skip)]
    pub(crate) raw_categories: Vec<(String, Vec<(String, GroupedQuantity)>)>,
}

impl AggregatedList {
    /// True when nothing needs buying.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Build both views from an aggregated `cooklang` list.
    ///
    /// The uncategorised pairs are taken first because
    /// [`IngredientList::categorize`] consumes the list and reorders what it
    /// keeps, so the insertion order cannot be recovered afterwards.
    fn build(list: IngredientList, aisle: &AisleConf) -> Self {
        let raw_items: Vec<(String, GroupedQuantity)> = list
            .iter()
            .map(|(name, quantity)| (name.clone(), quantity.clone()))
            .collect();
        let items = raw_items.iter().map(ListItem::render).collect();

        let raw_categories: Vec<(String, Vec<(String, GroupedQuantity)>)> = list
            .categorize(aisle)
            .into_iter()
            .map(|(category, items)| (category, items.into_iter().collect()))
            .collect();
        let categories = raw_categories
            .iter()
            .map(|(name, items)| ListCategory {
                name: name.clone(),
                items: items.iter().map(ListItem::render).collect(),
            })
            .collect();

        Self {
            items,
            categories,
            raw_items,
            raw_categories,
        }
    }
}

impl ListItem {
    fn render((name, quantity): &(String, GroupedQuantity)) -> Self {
        Self {
            name: name.clone(),
            quantities: ordered_components(quantity)
                .into_iter()
                .map(quantity_fmt)
                .collect(),
        }
    }
}

/// Build a shopping list from several recipes.
///
/// Ingredients with the same name are merged, converting units where they
/// convert; the aisle configuration from [`Context::aisle`] folds synonyms onto
/// one spelling and supplies the categories; the pantry configuration from
/// [`Context::pantry`] is subtracted from what is left. Pass
/// [`ConfigSource::None`] for either to skip that step — that is how CookCLI's
/// `--ignore-pantry` works.
///
/// Warnings — a recipe with suspect syntax, a configuration file that could not
/// be parsed, no aisle configuration at all — come back as
/// [`Outcome::diagnostics`] rather than being logged, each attributed to the
/// file it came from.
///
/// # Errors
///
/// - [`CoreError::Io`] if a configuration file or recipe is named but cannot be
///   read. A configuration file that is simply absent is not an error: pass
///   [`ConfigSource::None`].
/// - [`CoreError::RecipeNotFound`] if a recipe, or a recipe it references,
///   does not exist.
/// - [`CoreError::Parse`] if any recipe reached has parse errors.
/// - [`CoreError::Reference`] if a recipe reference cannot be scaled.
///
/// A recipe that reaches itself is *not* an error. See
/// [`extract_ingredients`].
pub fn generate(ctx: &Context, req: GenerateRequest) -> Result<Outcome<AggregatedList>, CoreError> {
    let mut diagnostics = Vec::new();

    // Both configurations are read up front and held for the whole call:
    // `AisleConf` borrows its category and ingredient names straight out of
    // the text it was parsed from.
    let aisle_text = ctx.aisle().read()?;
    let aisle = load_aisle(aisle_text.as_deref(), ctx.aisle(), &mut diagnostics);
    let pantry_text = ctx.pantry().read()?;
    let pantry = load_pantry(pantry_text.as_deref(), ctx.pantry(), &mut diagnostics);

    let options = ExtractOptions {
        ignore_references: req.ignore_references,
        included_references: None,
    };

    let mut list = IngredientList::new();
    for recipe in &req.recipes {
        diagnostics.extend(extract_ingredients(ctx, recipe, &options, &mut list)?);
    }

    // Extra items join the list after the recipes, so they keep the order the
    // user wrote them in and fall below the ingredients on the uncategorised
    // view. From here on they are indistinguishable from a recipe ingredient:
    // `use_common_names`, categorisation and the pantry all treat them the
    // same, which is what lets an extra "milk" merge with a recipe's.
    for spec in &req.extra_items {
        let outcome = parse_extra_item(spec)?;
        diagnostics.extend(outcome.diagnostics);
        list.add_recipe(&outcome.value, PARSER.converter(), false);
    }

    let mut list = list.use_common_names(&aisle, PARSER.converter());
    if let Some(pantry) = &pantry {
        list = list.subtract_pantry(pantry, PARSER.converter());
    }

    Ok(Outcome::with_diagnostics(
        AggregatedList::build(list, &aisle),
        diagnostics,
    ))
}

/// Read one [`GenerateRequest::extra_items`] entry as a single-ingredient
/// recipe.
///
/// The text is a Cooklang ingredient with the leading `@` left off, so an `@`
/// the user typed anyway is tolerated rather than doubled. A name with no
/// braces is one ingredient however many words long, but only when a `{}`
/// closes it — otherwise `paper towels` parses as `paper` followed by the loose
/// word `towels`. So the empty braces are supplied when the entry carries no
/// amount of its own, and left to the user when it does (`flour{200%g}`).
///
/// Errors as [`CoreError::Parse`], naming the entry as the user wrote it, when
/// the result is not a valid ingredient — an empty entry, or braces that do not
/// close.
fn parse_extra_item(spec: &str) -> Result<Outcome<Recipe>, CoreError> {
    let body = spec.trim().strip_prefix('@').unwrap_or(spec.trim()).trim();
    let line = if body.contains('{') {
        format!("@{body}")
    } else {
        format!("@{body}{{}}")
    };
    parse_unscaled(&line, spec, None)
}

/// Parse an aisle configuration, degrading to an empty one rather than failing.
///
/// An unparseable aisle file is reported as a warning and the list comes out
/// uncategorised. That is deliberately the behaviour CookCLI has today; see
/// <https://github.com/cooklang/cookcli/issues/416>.
fn load_aisle<'a>(
    text: Option<&'a str>,
    source: &ConfigSource,
    diagnostics: &mut Vec<Diagnostic>,
) -> AisleConf<'a> {
    let Some(text) = text else {
        let mut diagnostic = Diagnostic::warning(
            "no aisle configuration found, so the list will not be categorised",
        );
        diagnostic.hints = vec![format!(
            "the aisle file format is documented at {AISLE_DOCS}"
        )];
        diagnostics.push(diagnostic);
        return AisleConf::default();
    };

    let parsed = cooklang::aisle::parse_lenient(text);
    for warning in parsed.report().warnings() {
        diagnostics.push(at_source(
            Diagnostic::warning(format!("aisle configuration: {warning}")),
            source,
        ));
    }
    parsed.output().cloned().unwrap_or_else(|| {
        diagnostics.push(at_source(
            Diagnostic::warning(
                "aisle configuration could not be parsed, so the list will not be categorised",
            ),
            source,
        ));
        AisleConf::default()
    })
}

/// Parse a pantry configuration, degrading to none rather than failing.
fn load_pantry(
    text: Option<&str>,
    source: &ConfigSource,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<PantryConf> {
    let text = text?;

    let parsed = cooklang::pantry::parse_lenient(text);
    for warning in parsed.report().warnings() {
        diagnostics.push(at_source(
            Diagnostic::warning(format!("pantry configuration: {warning}")),
            source,
        ));
    }

    match parsed.output().cloned() {
        Some(mut pantry) => {
            // Redundant today: `parse_lenient` already builds the name index
            // that `subtract_pantry` looks ingredients up through, so removing
            // this call breaks no test. Kept because the CLI has always made it
            // and `rebuild_index` is documented as the way to resync the index
            // with `sections` — cheap insurance if that ever stops holding.
            pantry.rebuild_index();
            Some(pantry)
        }
        None => {
            diagnostics.push(at_source(
                Diagnostic::warning(
                    "pantry configuration could not be parsed, so nothing will be subtracted",
                ),
                source,
            ));
            None
        }
    }
}

/// Attribute a configuration diagnostic to its file, when it came from one.
fn at_source(diagnostic: Diagnostic, source: &ConfigSource) -> Diagnostic {
    match source.path() {
        Some(path) => diagnostic.at_file(path),
        None => diagnostic,
    }
}

/// Add one recipe's ingredients to `list`, expanding the recipes it references.
///
/// Quantities are merged into whatever `list` already holds, so several calls
/// accumulate one combined list. Returns the parse warnings raised on the way,
/// each attributed to the file it came from — with several recipes going into
/// one list, an unattributed warning says nothing useful.
///
/// # Reference expansion
///
/// A recipe reference (`@./sauce{}`) is replaced by the referenced recipe's
/// ingredients. A quantity on the reference (`@./sauce{500%ml}`) scales the
/// referenced recipe to that target rather than multiplying it.
///
/// Expansion is recursive: every reference is followed, and so is every
/// reference inside what it leads to, however deep the chain runs. It used to
/// stop three files in, and anything below that fell off the list with no
/// warning (<https://github.com/cooklang/cookcli/issues/509>). Two things bound
/// the descent: the cycle check below, and a hard depth limit for a chain long
/// enough to exhaust the stack without ever repeating itself — a hundred
/// levels, which no collection written by hand comes near.
///
/// A reference with no quantity of its own inherits the factor the recipe that
/// named it was scaled by, so scaling a menu scales everything under it. One
/// with a quantity names an absolute target instead, and the factor *that*
/// reaches is what carries on down from there.
///
/// **Every reference is resolved from disk**, under [`Context::base_path`],
/// including the references of a [`RecipeSource::Content`] recipe. Only the
/// starting recipe can come from memory; see [`ScaledRecipe`].
///
/// # Cycles
///
/// A reference back to a recipe already being expanded — the starting recipe,
/// or anything between it and here — is **not followed**. It raises a
/// [`Severity::Warning`] diagnostic naming the cycle instead.
///
/// Without that, two recipes referencing each other did not loop — expansion is
/// bounded, not recursive — but they silently double-counted: the starting
/// recipe's ingredients went in once directly and once more when the cycle led
/// back to it, and the deeper the mutual references the further the quantities
/// drifted (<https://github.com/cooklang/cookcli/issues/424>). A shopping list
/// with quietly wrong quantities is worse than one that says something is
/// wrong, so this warns rather than failing: the list is still produced, and it
/// is now correct.
///
/// What is tracked is the **chain of ancestors**, not everything seen. A recipe
/// two dishes both reference — one sauce for the pasta and the salad — is still
/// expanded for each of them, because neither is inside the other. Only a
/// reference that leads back up its own chain is refused.
///
/// Identity is the resolved file path, so a [`RecipeSource::Content`] starting
/// recipe has none: a reference cycle leading back to an unsaved buffer cannot
/// be recognised, since nothing on disk is that buffer.
///
/// # Errors
///
/// See [`generate`], which reports the same failures.
pub fn extract_ingredients(
    ctx: &Context,
    recipe: &ScaledRecipe,
    options: &ExtractOptions<'_>,
    list: &mut IngredientList,
) -> Result<Vec<Diagnostic>, CoreError> {
    let base_path = ctx.base_path();
    let converter = PARSER.converter();

    let (parsed, mut diagnostics) = parse_source(base_path, &recipe.source, recipe.scale)?;
    let ref_indices = list.add_recipe(&parsed, converter, options.ignore_references);

    // The chain of recipes currently being expanded, innermost last, by
    // resolved file path. A reference resolving to something already on it
    // would lead back up its own chain, so it is refused rather than followed.
    // See "Cycles" above for why this is the ancestor chain and not every
    // recipe seen.
    let starting_path = match &recipe.source {
        RecipeSource::Path(path) => find::get_recipe(base_path, path.as_str())
            .ok()
            .and_then(|entry| entry.path().cloned()),
        // An unsaved buffer is no file, so nothing can reference it.
        RecipeSource::Content { .. } => None,
    };
    let ancestors: Vec<Utf8PathBuf> = starting_path.into_iter().collect();

    tracing::debug!(
        "ignore_references = {}, ref_indices.len() = {}",
        options.ignore_references,
        ref_indices.len()
    );

    if !options.ignore_references {
        Expansion {
            base_path,
            converter,
            list,
            diagnostics: &mut diagnostics,
        }
        .expand(
            &parsed,
            &ref_indices,
            recipe.scale,
            options.included_references,
            &ancestors,
        )?;
    }

    Ok(diagnostics)
}

/// How deep a chain of recipe references is followed before the expansion gives
/// up and says so.
///
/// The ancestor chain refuses a reference that leads back on itself, so the
/// recursion is already bounded — but it is bounded at one level per recipe in
/// the collection, and expanding a few thousand of them as native recursion
/// overflows the stack and aborts the process. This is the second bound, and
/// the one that keeps a pathological collection a diagnostic rather than a
/// crash.
///
/// A menu of dinners of sauces of preparations is four. A hundred is not a
/// number of levels anyone reaches by writing recipes, and it leaves an order
/// of magnitude of headroom below where an unoptimised build runs out of stack.
const MAX_REFERENCE_DEPTH: usize = 100;

/// What stays the same for the whole of one expansion — where to look recipes
/// up, and what to accumulate into — so the recursion carries only what
/// actually changes as it goes down.
struct Expansion<'a> {
    base_path: &'a Utf8Path,
    converter: &'a Converter,
    list: &'a mut IngredientList,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl Expansion<'_> {
    /// The directory holding the recipe currently being expanded, relative to
    /// the recipe directory — what a `..` in one of its references steps up
    /// from.
    ///
    /// Empty for a recipe sitting at the recipe directory itself. Empty too
    /// when there is no such file to speak of — an unsaved buffer passed as
    /// [`RecipeSource::Content`] — or when its path does not sit under
    /// `base_path` after all, which leaves a `..` nothing to spend and so
    /// refuses it rather than guessing where it meant to land.
    fn reference_dir(&self, ancestors: &[Utf8PathBuf]) -> Utf8PathBuf {
        ancestors
            .last()
            .and_then(|path| path.strip_prefix(self.base_path).ok())
            .and_then(|relative| relative.parent())
            .map(Utf8Path::to_owned)
            .unwrap_or_default()
    }

    /// Expand the recipes `recipe` references into `list`, and the recipes *those*
    /// reference, all the way down.
    ///
    /// `ref_indices` is what [`IngredientList::add_recipe`] returned for `recipe`:
    /// its own ingredients are already on the list, and the references it skipped
    /// are what is left to follow.
    ///
    /// `scale` is the factor `recipe` was scaled by. A reference that carries no
    /// quantity of its own inherits it, so scaling a menu scales every recipe
    /// underneath it.
    ///
    /// `included` selects which references to follow, and applies to **this level
    /// only** — the web UI's checkboxes name the references of the recipe a shopper
    /// is looking at, and nothing deeper is theirs to name. The recursion passes
    /// `None`.
    ///
    /// `ancestors` is the chain of recipes currently being expanded, innermost
    /// last; see "Cycles" on [`extract_ingredients`] for why it is the chain rather
    /// than everything seen. It is the first of the two things that bound this
    /// recursion: a reference leading back into its own chain is refused, so a
    /// finite collection of recipes is a finite descent however the references
    /// are wired. Its length is also the depth, which is how
    /// [`MAX_REFERENCE_DEPTH`] — the second — is checked.
    fn expand(
        &mut self,
        recipe: &Recipe,
        ref_indices: &[usize],
        scale: f64,
        included: Option<&[String]>,
        ancestors: &[Utf8PathBuf],
    ) -> Result<(), CoreError> {
        for &ref_index in ref_indices {
            let ingredient = &recipe.ingredients[ref_index];
            let Some(reference) = ingredient.reference.as_ref() else {
                continue;
            };

            // The display-style path, matching what the web UI shows.
            let ref_display_path = if reference.components.is_empty() {
                reference.name.clone()
            } else {
                format!("{}/{}", reference.components.join("/"), reference.name)
            };

            // Where the reference points, as a path relative to the recipe
            // directory. `..` is resolved against the directory of the recipe
            // writing it, and a reference that climbs out of the collection —
            // or names an absolute path of its own — resolves to nothing. See
            // [`find::resolve_reference`].
            let from = self.reference_dir(ancestors);
            let Some(ref_path) = find::resolve_reference(&from, &ref_display_path) else {
                let mut outside = Diagnostic::warning(format!(
                    "Skipped recipe reference '{ref_display_path}': it points outside the \
                     recipe directory, so it is not a recipe in this collection. Anything \
                     it would have added is not on the list"
                ));
                if let Some(path) = ancestors.last() {
                    outside = outside.at_file(path.clone());
                }
                self.diagnostics.push(outside);
                continue;
            };
            let ref_path = ref_path.to_string();

            // If the caller specified which references to include, skip others.
            // Both sides are resolved first, so a stored `./sauce`, a stored
            // `sauce` and a reference written `@./sauce{}` are one thing —
            // which is what the old `./`-stripping did, and it goes on holding
            // for the `../sauces/tomato` a stored path now spells resolved.
            if let Some(included) = included {
                if !included.iter().any(|r| {
                    find::resolve_reference(&from, r).is_some_and(|p| p.as_str() == ref_path)
                }) {
                    tracing::debug!(
                        "Skipping reference '{}' — not in included_references",
                        ref_display_path
                    );
                    continue;
                }
            }

            // `ancestors` holds one recipe per level, so its length is how deep
            // this reference sits. Stop well above anything a real collection
            // reaches but well below where the recursion runs out of stack —
            // see `MAX_REFERENCE_DEPTH`.
            if ancestors.len() >= MAX_REFERENCE_DEPTH {
                let mut stopped = Diagnostic::warning(format!(
                    "Stopped at recipe reference '{ref_path}': references are nested more \
                     than {MAX_REFERENCE_DEPTH} deep here. Anything below it is not on the \
                     list"
                ));
                // Attributed to the recipe holding the reference, so a caller
                // can group or open it rather than reading the file name back
                // out of the message. Not the same choice `cycle_warning`
                // makes — that one names the recipe being referenced, which in
                // a cycle is the interesting end of it. Here the reference is
                // one of many that stop at the same place, and what a reader
                // wants is where the list stopped.
                //
                // `ancestors` cannot be empty here — it is long enough to have
                // hit the limit — but an unattributed warning is a better
                // answer to being wrong about that than a panic.
                if let Some(path) = ancestors.last() {
                    stopped = stopped.at_file(path.clone());
                }
                self.diagnostics.push(stopped);
                continue;
            }

            let ref_entry = find::get_recipe(self.base_path, &ref_path)?;

            if let Some(cycle) = cycle_warning(ancestors, &ref_entry, &ref_path) {
                self.diagnostics.push(cycle);
                continue;
            }
            // Expanding this one, so it is an ancestor of anything inside it.
            let mut ancestors = ancestors.to_vec();
            ancestors.extend(ref_entry.path().cloned());

            // Parse and scale the recipe based on the quantity specification.
            let (ref_recipe, ref_scale) = match ingredient.quantity.as_ref() {
                Some(quantity) => {
                    let target_value =
                        match quantity.value() {
                            Value::Number(num) => num.to_string().parse::<f64>().map_err(|_| {
                                CoreError::Reference {
                                    name: ref_path.clone(),
                                    message: format!("invalid numeric value: {num}"),
                                }
                            })?,
                            other => {
                                return Err(CoreError::Reference {
                                    name: ref_path.clone(),
                                    message: format!("quantity is not a number: {other}"),
                                });
                            }
                        };

                    let (mut ref_recipe, ref_diagnostics) =
                        parse_entry(&ref_entry, &ref_path, None)?;
                    self.diagnostics.extend(ref_diagnostics);

                    tracing::debug!(
                        "Scaling recipe '{}' to target {} {}",
                        ref_path,
                        target_value,
                        quantity.unit().unwrap_or("(no unit)")
                    );
                    // Read before scaling: the servings and yield this is worked
                    // out from are what scaling rewrites.
                    let reached = target_factor(&ref_recipe, target_value, quantity.unit());
                    ref_recipe
                        .scale_to_target(target_value, quantity.unit(), self.converter)
                        .map_err(|e| CoreError::Reference {
                            name: ref_path.clone(),
                            message: format!(
                                "cannot scale to {} {}: {e}",
                                target_value,
                                quantity.unit().unwrap_or("(no unit)")
                            ),
                        })?;

                    // The target is absolute, so the caller's factor does not
                    // multiply into it — but the factor it *reached* carries on
                    // down to whatever this recipe references in turn.
                    (ref_recipe, reached.unwrap_or(1.0))
                }
                None => {
                    // No quantity specified, so the caller's scaling applies.
                    let (ref_recipe, ref_diagnostics) =
                        parse_entry(&ref_entry, &ref_path, Some(scale))?;
                    self.diagnostics.extend(ref_diagnostics);
                    (ref_recipe, scale)
                }
            };

            // The referenced recipe's own ingredients, and then the recipes it
            // references in turn — `add_recipe` hands back the ones it skipped,
            // which is exactly what is left to follow.
            let nested = self.list.add_recipe(&ref_recipe, self.converter, false);
            tracing::debug!("Found {} nested references to process", nested.len());
            self.expand(&ref_recipe, &nested, ref_scale, None, &ancestors)?;
        }

        Ok(())
    }
}

/// The factor [`Recipe::scale_to_target`] works out for `target`, so that
/// references *inside* the scaled recipe that carry no quantity of their own
/// can inherit it.
///
/// `scale_to_target` applies the factor without reporting it, so this derives
/// it the same way — and must run first, while the recipe still holds the
/// servings and yield that scaling rewrites.
///
/// `None` when the recipe cannot answer: servings that are not a whole number
/// — `cooklang` holds them as a `u32`, so `1.5` is as unreadable to it as none
/// at all — no yield, or a yield measured in another unit. `scale_to_target`
/// raises the error for each of those, so nothing here has to, and the
/// caller's `unwrap_or(1.0)` is unreachable.
///
/// With one exception, which is why that fallback is a `1` and not an
/// `expect`: a **zero** base, `servings: 0` or `yield: 0%g`. `cooklang`
/// divides by it and scales the recipe by infinity; this returns `None` and
/// what the recipe references is left alone. Both readings of such a recipe
/// are nonsense, and confining the nonsense to the one recipe `cooklang`
/// itself scaled beats spreading infinity down the chain.
///
/// Unit names are matched exactly, including case, because that is how
/// `cooklang` matches them — mirroring it is the whole point, and matching
/// more loosely here would answer where `scale_to_target` refuses, which is
/// the one way the fallback could be reached in earnest. The menu feature's
/// `reference_scale_factor`, over in the `cookcli` crate's
/// `util::menu_scale`, does the same conversion case-insensitively and falls
/// back to the raw target rather than to one. The two cannot share an
/// implementation while they disagree about that: this one is a shadow of
/// `cooklang`'s arithmetic and has to match it exactly, and that one is a
/// policy about what a menu author probably meant.
fn target_factor(recipe: &Recipe, target: f64, unit: Option<&str>) -> Option<f64> {
    let base = match unit {
        // No unit at all is already a factor rather than an amount.
        None => return Some(target),
        // Whole servings only, rounded the way `scale_to_servings` rounds
        // them, so that half a serving asked for does not put the two out of
        // step.
        Some("servings") | Some("serving") => {
            return match f64::from(recipe.metadata.servings()?.as_number()?) {
                0.0 => None,
                base => Some(target.round() / base),
            }
        }
        Some(unit) => {
            // The one yield spelling `scale_to_yield` accepts: `1000%g`.
            let (value, yield_unit) = recipe.metadata.get("yield")?.as_str()?.split_once('%')?;
            if yield_unit != unit {
                return None;
            }
            value.parse::<f64>().ok()?
        }
    };

    (base != 0.0).then(|| target / base)
}

/// The warning to raise if expanding `entry` would lead back up its own chain,
/// or `None` when it is safe to expand.
///
/// `ancestors` is the chain of recipes currently being expanded, innermost
/// last, by resolved file path; `lookup` is how the reference was written, so
/// the message names what the author typed rather than an absolute path they
/// never wrote.
///
/// An entry with no resolved path cannot be compared to anything and is treated
/// as safe. That is not a hole in practice: every entry here came from
/// [`find::get_recipe`], which only finds files.
fn cycle_warning(
    ancestors: &[Utf8PathBuf],
    entry: &RecipeEntry,
    lookup: &str,
) -> Option<Diagnostic> {
    let path = entry.path()?;
    if !ancestors.iter().any(|ancestor| ancestor == path) {
        return None;
    }

    // Name the whole chain: with mutual references the pair is what is wrong,
    // and naming only the end of it leaves the reader to work out the rest.
    let chain = ancestors
        .iter()
        .map(|p| p.file_name().unwrap_or(p.as_str()).to_string())
        .collect::<Vec<_>>()
        .join(" → ");
    let repeated = path.file_name().unwrap_or(path.as_str());

    Some(
        Diagnostic::warning(format!(
            "Skipped circular recipe reference '{lookup}': {chain} → {repeated} leads back to a \
             recipe already being expanded. Its ingredients are counted once"
        ))
        .at_file(path.clone()),
    )
}

/// Parse the recipe a [`ScaledRecipe`] names, from disk or from memory.
///
/// The two arms differ only in where the text comes from. In-memory text is
/// attributed to its caller-supplied `name` — the same rule
/// [`recipe::read`](crate::recipe::read) follows — and carries no file, so
/// [`Diagnostic::location`] has a span but no path for callers to open.
/// Deliberately never touches the filesystem for a [`RecipeSource::Content`]:
/// the whole point is a buffer that has no file yet.
fn parse_source(
    base_path: &Utf8Path,
    source: &RecipeSource,
    scale: f64,
) -> Result<(Recipe, Vec<Diagnostic>), CoreError> {
    match source {
        RecipeSource::Path(path) => {
            let entry = find::get_recipe(base_path, path.as_str())?;
            parse_entry(&entry, path.as_str(), Some(scale))
        }
        RecipeSource::Content { text, name } => {
            let outcome = parse_recipe_at(text, name, scale, None)?;
            Ok((outcome.value, outcome.diagnostics))
        }
    }
}

/// Read and parse a resolved recipe file.
///
/// `scale` of `None` means "leave the quantities alone", which is not the same
/// as `Some(1.0)` — see [`parse_unscaled`].
fn parse_entry(
    entry: &RecipeEntry,
    lookup: &str,
    scale: Option<f64>,
) -> Result<(Recipe, Vec<Diagnostic>), CoreError> {
    let path = entry.path().cloned();
    // Name the file when it is known, so diagnostics point at something the
    // caller can open, and fall back to what was looked up otherwise.
    let display = path
        .clone()
        .unwrap_or_else(|| Utf8PathBuf::from(lookup))
        .to_string();

    let content = entry.content().map_err(|source| CoreError::Io {
        path: Utf8PathBuf::from(&display),
        source: find::entry_error(source),
    })?;

    let outcome = match scale {
        Some(scale) => parse_recipe_at(&content, &display, scale, path.as_deref())?,
        None => parse_unscaled(&content, &display, path.as_deref())?,
    };
    Ok((outcome.value, outcome.diagnostics))
}

#[cfg(test)]
mod tests;
