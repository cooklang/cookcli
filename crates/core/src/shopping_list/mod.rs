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
    format::shopping_list::quantity_fmt,
    parser::{parse_recipe_at, parse_unscaled, PARSER},
    ConfigSource, Context, CoreError, Diagnostic, Outcome, RecipeSource,
};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::{
    aisle::AisleConf, ingredient_list::IngredientList, pantry::PantryConf,
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
#[derive(Debug, Clone)]
pub struct GenerateRequest {
    /// The recipes to include, each with its own scaling factor.
    pub recipes: Vec<ScaledRecipe>,
    /// Leave recipes referenced from within a recipe unexpanded.
    ///
    /// This does not drop the reference: it stays on the list as an item named
    /// after the referenced recipe, with no quantity.
    pub ignore_references: bool,
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
    /// A leading `./` is ignored on both sides of the comparison, so a path
    /// stored without one still matches a reference written with one.
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
    /// empty when no recipe gave a quantity at all.
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
            quantities: quantity.iter().map(quantity_fmt).collect(),
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

    let mut list = list.use_common_names(&aisle, PARSER.converter());
    if let Some(pantry) = &pantry {
        list = list.subtract_pantry(pantry, PARSER.converter());
    }

    Ok(Outcome::with_diagnostics(
        AggregatedList::build(list, &aisle),
        diagnostics,
    ))
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
/// Expansion is iterative and stops three files deep: the named recipe, the
/// recipes it references, and the recipes *those* reference — whose own
/// references are not followed.
///
/// **Every reference is resolved from disk**, under [`Context::base_path`],
/// including the references of a [`RecipeSource::Content`] recipe. Only the
/// starting recipe can come from memory; see [`ScaledRecipe`].
///
/// # Cycles are not detected
///
/// Because expansion is bounded rather than recursive, two recipes that
/// reference each other neither loop nor fail. They silently double-count: the
/// ingredients of the starting recipe are added once directly and once more
/// when the cycle leads back to it. That is
/// <https://github.com/cooklang/cookcli/issues/424>, pinned by
/// `mutually_referencing_recipes_terminate`. Fixing it means making expansion
/// recursive, tracking the recipes on the current path, and adding a
/// `CoreError` variant for the cycle — deliberately absent today rather than
/// present and unreachable.
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

    tracing::debug!(
        "ignore_references = {}, ref_indices.len() = {}",
        options.ignore_references,
        ref_indices.len()
    );

    if !options.ignore_references {
        for ref_index in ref_indices {
            let ingredient = &parsed.ingredients[ref_index];
            let Some(reference) = ingredient.reference.as_ref() else {
                continue;
            };

            // The display-style path, matching what the web UI shows.
            let ref_display_path = if reference.components.is_empty() {
                reference.name.clone()
            } else {
                format!("{}/{}", reference.components.join("/"), reference.name)
            };

            // If the caller specified which references to include, skip others.
            // Normalize by stripping "./" prefix so paths stored without one
            // still match display paths, which may carry one.
            if let Some(included) = options.included_references {
                fn strip_dot_slash(s: &str) -> &str {
                    s.strip_prefix("./").unwrap_or(s)
                }
                if !included
                    .iter()
                    .any(|r| strip_dot_slash(r) == strip_dot_slash(&ref_display_path))
                {
                    tracing::debug!(
                        "Skipping reference '{}' — not in included_references",
                        ref_display_path
                    );
                    continue;
                }
            }

            let ref_path = reference.path(std::path::MAIN_SEPARATOR_STR);
            let ref_entry = find::get_recipe(base_path, &ref_path)?;

            // Parse and scale the recipe based on the quantity specification.
            let ref_recipe = match ingredient.quantity.as_ref() {
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
                    diagnostics.extend(ref_diagnostics);

                    tracing::debug!(
                        "Scaling recipe '{}' to target {} {}",
                        ref_path,
                        target_value,
                        quantity.unit().unwrap_or("(no unit)")
                    );
                    ref_recipe
                        .scale_to_target(target_value, quantity.unit(), converter)
                        .map_err(|e| CoreError::Reference {
                            name: ref_path.clone(),
                            message: format!(
                                "cannot scale to {} {}: {e}",
                                target_value,
                                quantity.unit().unwrap_or("(no unit)")
                            ),
                        })?;

                    // No further scaling: the target already accounts for it.
                    ref_recipe
                }
                None => {
                    // No quantity specified, so the caller's scaling applies.
                    let (ref_recipe, ref_diagnostics) =
                        parse_entry(&ref_entry, &ref_path, Some(recipe.scale))?;
                    diagnostics.extend(ref_diagnostics);
                    ref_recipe
                }
            };

            // References inside the referenced recipe, one level further down.
            let nested_refs: Vec<usize> = ref_recipe
                .ingredients
                .iter()
                .enumerate()
                .filter(|(_, ingredient)| ingredient.reference.is_some())
                .map(|(index, _)| index)
                .collect();

            tracing::debug!("Found {} nested references to process", nested_refs.len());
            for nested_index in nested_refs {
                let nested_ingredient = &ref_recipe.ingredients[nested_index];
                tracing::debug!("Processing nested ingredient: {:?}", nested_ingredient.name);
                let Some(nested_ref) = &nested_ingredient.reference else {
                    continue;
                };

                let nested_path = if nested_ref.components.is_empty() {
                    nested_ref.name.clone()
                } else {
                    let sep = std::path::MAIN_SEPARATOR.to_string();
                    format!(
                        ".{}{}{}{}",
                        sep,
                        nested_ref.components.join(&sep),
                        sep,
                        nested_ref.name
                    )
                };

                let nested_entry = find::get_recipe(base_path, &nested_path)?;
                let (mut nested_recipe, nested_diagnostics) =
                    parse_entry(&nested_entry, &nested_path, None)?;
                diagnostics.extend(nested_diagnostics);

                // Scaling a nested reference depends on the unit it names.
                match &nested_ingredient.quantity {
                    Some(quantity) if quantity.unit() == Some("servings") => {
                        // The quantity is the number of servings wanted.
                        if let Value::Number(target_servings) = quantity.value() {
                            let target = target_servings.to_string().parse().unwrap_or(1.0);
                            tracing::debug!("Scaling nested recipe to {} servings", target);
                            nested_recipe
                                .scale_to_target(target, Some("servings"), converter)
                                .map_err(|e| CoreError::Reference {
                                    name: nested_path.clone(),
                                    message: format!("cannot scale to {target} servings: {e}"),
                                })?;
                            // References are expanded above, so exclude them here.
                            list.add_recipe(&nested_recipe, converter, false);
                        }
                    }
                    Some(quantity) => {
                        // Any other unit is treated as a plain scaling factor,
                        // covering cases like "2 cups" of something.
                        if let Value::Number(num) = quantity.value() {
                            let scaling = num.to_string().parse().unwrap_or(1.0);
                            nested_recipe.scale(scaling, converter);
                            list.add_recipe(&nested_recipe, converter, false);
                        }
                    }
                    None => {
                        list.add_recipe(&nested_recipe, converter, false);
                    }
                }
            }

            // The referenced recipe's own ingredients go in last, after its
            // nested references, so they are not counted twice.
            list.add_recipe(&ref_recipe, converter, false);
        }
    }

    Ok(diagnostics)
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
