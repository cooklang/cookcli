use crate::server::handlers::common::{check_path, json_error, ApiError};
use crate::server::{activity, private_paths, AppState};
use crate::util::inside_collection;
use crate::util::recipe_info::{
    combined_quantity, ref_info_or_default, reference_factor, resolve_recipe_info, RecipeInfo,
};
use crate::util::PARSER;
use crate::web::language::get_preferred_language;
use crate::web::templates::Tr;
use crate::web::viewer::Viewer;
use anyhow::Context as _;
use axum::http::HeaderMap;
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use camino::{Utf8Path, Utf8PathBuf};
use cookcli_core::shopping_list::{
    add_extra_item, add_optional_selections, extract_ingredients, merge_case_variants,
    optional_selections, recipe_display_name, subtract_pantry_from_optional, ExtractOptions,
    FreeHandItem, OptionalSelection, ScaledRecipe, ShoppingIngredients, ShoppingListStore,
    StoredEntry,
};
use cookcli_core::{Diagnostic, DiagnosticKind, UnscalableCause};
use cooklang::ingredient_list::IngredientList;
use cooklang::model::Ingredient;
use cooklang::pantry::PantryConf;
use cooklang::quantity::GroupedQuantity;
use cooklang::Recipe;
use serde::Deserialize;
use serde_json;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct RecipeRequest {
    recipe: String,
    scale: Option<f64>,
    /// Which sub-recipe references to include. `None` = all.
    included_references: Option<Vec<String>>,
    /// The optional ingredients chosen for this recipe, with the amounts to
    /// buy, as stored in `.shopping-list`.
    #[serde(default)]
    optional_ingredients: Vec<OptionalSelection>,
}

/// One thing for [`shopping_list`] to put on the list: a recipe, or a
/// free-hand item in the shape `/api/shopping_list/extra_items` returns.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ListRequest {
    Recipe(RecipeRequest),
    Item(FreeHandItem),
}

pub async fn shopping_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<Vec<ListRequest>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Warnings are shown as they are, so they are worded in the page's
    // language here.
    let tr = Tr::new(get_preferred_language(&headers));
    let mut ingredients = ShoppingIngredients::default();
    let core_ctx = cookcli_core::Context::new(state.base_path.clone());
    // Why the list is not what the recipes on it say — a reference listed as
    // written because its recipe could not be scaled. Sent along so the page
    // can show it: the server log is no place for a shopper to find out why
    // an ingredient is missing.
    let mut warnings: Vec<String> = Vec::new();

    for entry in payload {
        let entry = match entry {
            ListRequest::Recipe(entry) => entry,
            ListRequest::Item(item) => {
                // Merged into the list like the CLI's `--extra` items: same
                // name, same aisle, same pantry subtraction.
                add_extra_item(&item.spec(), &mut ingredients.required)
                    .map_err(|e| (StatusCode::BAD_REQUEST, json_error(e)))?;
                continue;
            }
        };
        let name = entry.recipe;
        // Straight to `cooklang_find`, which joins it to the recipe directory
        // and asks the filesystem. Checked before that happens, not after:
        // on Windows, merely looking up `\\host\share` hands that host the
        // user's NTLM hash.
        check_path(&name)?;
        // A name that reaches a file through a link out of the collection is
        // refused before it is read — this route is guest-reachable (#698).
        if !inside_collection(&state.base_path, &state.base_path.join(&name)) {
            // The refusal says what core says, so a link out and a dangling
            // name answer identically.
            return Err((
                StatusCode::BAD_REQUEST,
                json_error(format!("recipe not found: {name}")),
            ));
        }
        let recipe = ScaledRecipe::scaled(
            cookcli_core::RecipeSource::Path(name.as_str().into()),
            entry.scale.unwrap_or(1.0),
        );

        let diagnostics = extract_ingredients(
            &core_ctx,
            &recipe,
            &ExtractOptions {
                ignore_references: false,
                included_references: entry.included_references.as_deref(),
                // Only what was chosen, below.
                include_optional: false,
                confined: true,
            },
            &mut ingredients,
        )
        .map_err(|e| {
            tracing::error!("Error processing recipe: {}", e);
            (StatusCode::BAD_REQUEST, json_error(e))
        })?;
        let diagnostics = diagnostics.into_iter().chain(add_optional_selections(
            &mut ingredients,
            &entry.optional_ingredients,
        ));

        for diagnostic in diagnostics {
            tracing::warn!("Recipe '{}': {}", name, diagnostic.message);
            // Named after the recipe it is about — the one that raised it,
            // which for a reference further down is not the one requested.
            let file = diagnostic
                .location
                .as_ref()
                .and_then(|l| l.file.as_deref())
                // With `/` on Windows too, like every recipe path the API
                // sends — and like the reference named in the same line.
                .map(|file| {
                    file.strip_prefix(&state.base_path)
                        .unwrap_or(file)
                        .as_str()
                        .replace('\\', "/")
                })
                .unwrap_or_else(|| name.clone());
            // The rest — a parse warning in one of the recipes, say — is for
            // the recipe page and `cook doctor`, and stays in the log.
            let Some(warning) = warning_text(&tr, &file, &diagnostic) else {
                continue;
            };
            let warning = private_paths::hide(&warning);
            // Two recipes on the list can share a sub-recipe, and so the
            // warning about it.
            if !warnings.contains(&warning) {
                warnings.push(warning);
            }
        }
    }

    // Merge case variants so `sugar`/`Sugar` buy and deplete the pantry as
    // one item (#694) — each list on its own, required and optional stay
    // separate.
    let ShoppingIngredients { required, optional } = ingredients;
    let required = merge_case_variants(required);
    let optional = merge_case_variants(optional);

    // Parse aisle with lenient parsing
    let aisle_content = read_aisle_file(&state);
    let aisle_result = cooklang::aisle::parse_lenient(&aisle_content);

    if aisle_result.report().has_warnings() {
        for warning in aisle_result.report().warnings() {
            tracing::warn!("Aisle configuration warning: {}", warning);
        }
    }

    let aisle = aisle_result.output().cloned().unwrap_or_default();

    let (pantry_conf, pantry_error) = read_pantry(&state);

    // Use common names from aisle configuration
    let required = required.use_common_names(&aisle, PARSER.converter());
    let optional = optional.use_common_names(&aisle, PARSER.converter());

    // Track pantry items that were found and subtracted (excluding zero quantities)
    let mut pantry_items: Vec<String> = Vec::new();
    if let Some(ref pantry) = pantry_conf {
        // Check which items from the original list are in the pantry with non-zero quantity
        for (ingredient_name, _) in required.iter().chain(optional.iter()) {
            if pantry_items.contains(ingredient_name) {
                continue;
            }
            if let Some((_, pantry_item)) = pantry.find_ingredient(ingredient_name) {
                // Check if the pantry item has a non-zero quantity
                if let Some(qty_str) = pantry_item.quantity() {
                    // Special case for unlimited
                    if qty_str == "unlim" || qty_str == "unlimited" {
                        pantry_items.push(ingredient_name.clone());
                    } else if let Some((value, _)) = pantry_item.parsed_quantity() {
                        // Only include if quantity is greater than 0
                        if value > 0.0 {
                            pantry_items.push(ingredient_name.clone());
                        }
                    }
                } else {
                    // No quantity specified means we have it (backward compatibility)
                    pantry_items.push(ingredient_name.clone());
                }
            }
        }
    }

    // Apply pantry subtraction if pantry is available. Stock counts against
    // the required amount first, so the optional list is worked out from the
    // required one before that is reduced.
    let (required, optional) = if let Some(ref pantry) = pantry_conf {
        let optional = subtract_pantry_from_optional(optional, &required, pantry);
        (
            required.subtract_pantry(pantry, PARSER.converter()),
            optional,
        )
    } else {
        (required, optional)
    };

    // Categorise both lists into one, optional items after the required ones
    // within each category, and categories in the aisle configuration's order.
    type Item = (String, cooklang::quantity::GroupedQuantity, bool);
    let mut categories: Vec<(String, Vec<Item>)> = Vec::new();
    for (list, is_optional) in [(required, false), (optional, true)] {
        for (category, items) in list.categorize(&aisle) {
            let items = items
                .iter()
                .map(|(name, qty)| (name.clone(), qty.clone(), is_optional));
            match categories.iter_mut().find(|(c, _)| *c == category) {
                Some((_, existing)) => existing.extend(items),
                None => categories.push((category, items.collect())),
            }
        }
    }
    categories.sort_by_key(|(category, _)| {
        aisle
            .categories
            .iter()
            .position(|c| c.name == category)
            .unwrap_or(usize::MAX)
    });

    // Build the response
    let mut shopping_categories = Vec::new();

    for (category, mut entries) in categories {
        // The "other" bucket holds ingredients with no aisle category. They
        // arrive in recipe insertion order, which is unhelpful when scanning
        // a long list — sort alphabetically (case-insensitive) so shoppers
        // can find items predictably. The sort is stable, so a required
        // item stays ahead of the optional one with the same name.
        if category == "other" {
            entries.sort_by_key(|(a, _, _)| a.to_lowercase());
        }

        let mut shopping_items = Vec::new();
        for (name, qty, is_optional) in entries {
            let item_json = serde_json::json!({
                "name": name,
                // Not `into_vec()`: that yields the components in the group's
                // own random order, so an ingredient measured two ways came
                // back differently on every request.
                "quantities": crate::util::format::quantity::ordered_components(&qty),
                "optional": is_optional,
            });
            shopping_items.push(item_json);
        }

        if !shopping_items.is_empty() {
            shopping_categories.push(serde_json::json!({
                "category": category,
                "items": shopping_items
            }));
        }
    }

    // Load checked state
    let store = ShoppingListStore::new(&state.base_path);
    let checked = store.checked_set().unwrap_or_default();

    let json_value = serde_json::json!({
        "categories": shopping_categories,
        "pantry_items": pantry_items,
        "pantry_error": pantry_error,
        "checked": checked.into_iter().collect::<Vec<_>>(),
        "warnings": warnings
    });
    Ok(Json(json_value))
}

/// The pantry to take off the list, and, when there is a pantry file but it
/// could not be used, why.
///
/// The list is made without it then, so the reason goes to the page: a list
/// asking for everything already at home is otherwise all a shopper sees. It is
/// worded as the pantry page words it, where the file can be fixed.
fn read_pantry(state: &AppState) -> (Option<PantryConf>, Option<String>) {
    let Some(path) = &state.pantry_path else {
        tracing::debug!("No pantry file configured");
        return (None, None);
    };

    match std::fs::read_to_string(path) {
        Ok(content) => {
            tracing::debug!("Loaded pantry file from: {:?}", path);
            let result = cooklang::pantry::parse_lenient(&content);
            for warning in result.report().warnings() {
                tracing::warn!("Pantry configuration warning: {}", warning);
            }
            if let Some(pantry) = result.output() {
                return (Some(pantry.clone()), None);
            }
        }
        Err(e) => tracing::debug!("Failed to read pantry file from {:?}: {}", path, e),
    }

    // Read again through core for the error the pantry page shows, which
    // quotes the parser and the line; reached only when the file is broken.
    let ctx = cookcli_core::Context::new(state.base_path.clone())
        .with_pantry(cookcli_core::ConfigSource::Path(path.clone()));
    let error = match cookcli_core::pantry::load(&ctx) {
        Err(error) => error.to_string(),
        // Fixed in between.
        Ok(_) => "the pantry configuration could not be parsed".to_string(),
    };
    tracing::warn!("Pantry not used for the shopping list: {error}");
    (None, Some(private_paths::hide(&error)))
}

/// `diagnostic` as one line for the shopping list page, in the page's language
/// and after the recipe it is about — or `None` when it is not one the page
/// shows.
///
/// The page shows what core describes in parts ([`Diagnostic::kind`]): those
/// are about the list, and can be worded in any language. That leaves out the
/// parse warnings of the recipes on it, which the list is not where to fix —
/// and one of which, `cooklang` 0.19 reading `yield` as a synonym of
/// `servings`, is raised by the very `yield: 600%g` this warning asks for.
fn warning_text(tr: &Tr, recipe: &str, diagnostic: &Diagnostic) -> Option<String> {
    let Some(DiagnosticKind::UnscalableReference {
        reference,
        amount,
        cause,
    }) = &diagnostic.kind
    else {
        return None;
    };
    let (key, unit, declared) = match cause {
        UnscalableCause::NotANumber => ("shopping-warning-not-a-number", "", ""),
        UnscalableCause::NoServings => ("shopping-warning-no-servings", "", ""),
        UnscalableCause::NoYield { unit } => ("shopping-warning-no-yield", unit.as_str(), ""),
        UnscalableCause::YieldInOtherUnit { unit } => {
            ("shopping-warning-yield-unit", unit.as_str(), "")
        }
        UnscalableCause::NonPositiveBase { declared } => {
            ("shopping-warning-non-positive-base", "", declared.as_str())
        }
        _ => return None,
    };
    Some(tr.ta(
        key,
        &[
            ("recipe", recipe),
            ("reference", reference),
            ("amount", amount),
            ("unit", unit),
            ("declared", declared),
        ],
    ))
}

pub async fn get_shopping_list_items(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<StoredEntry>>, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    let mut items = store.load().map_err(|e| {
        tracing::error!("Failed to load shopping list: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    let core_ctx = cookcli_core::Context::new(state.base_path.clone());
    name_by_title(&core_ctx, &mut items);
    Ok(Json(items))
}

/// Name every entry — menus, the recipes in them, and the sub-recipes each
/// pulls in — by the title its file declares rather than by its file name.
///
/// Done on the way out rather than stored, so a list written before this, or
/// a recipe retitled since, still shows the current title. The sub-recipes'
/// sizes are worked out on the way too, so their links open them at it.
fn name_by_title(ctx: &cookcli_core::Context, entries: &mut [StoredEntry]) {
    for entry in entries {
        // `None` only when the stored path is not one to look up; keep the
        // file name then, and read sub-references from the root.
        let path = cookcli_core::resolve_reference(Utf8Path::new(""), &entry.path);
        if let Some(title) = path.as_deref().and_then(|path| recipe_title(ctx, path)) {
            entry.name = title;
        }

        if let Some(refs) = &entry.included_references {
            // A reference that steps up does so from the directory of the
            // recipe writing it — see `resolve_reference`.
            let dir = path
                .as_deref()
                .and_then(Utf8Path::parent)
                .unwrap_or(Utf8Path::new(""));
            entry.included_reference_names = Some(
                refs.iter()
                    .map(|reference| {
                        cookcli_core::resolve_reference(dir, reference)
                            .and_then(|path| recipe_title(ctx, &path))
                            .unwrap_or_else(|| recipe_display_name(reference))
                    })
                    .collect(),
            );
            entry.included_reference_scales = Some(reference_scales(
                ctx,
                path.as_deref(),
                entry.scale,
                dir,
                refs,
            ));
        }

        if let Some(recipes) = &mut entry.recipes {
            name_by_title(ctx, recipes);
        }
    }
}

/// How many times its own size each of `refs` is made when the recipe at
/// `path` is made `scale` times — what that recipe's page opens them at, see
/// [`reference_factor`]. `None` for any that cannot be worked out, and for all
/// of them when the recipe cannot be read.
fn reference_scales(
    ctx: &cookcli_core::Context,
    path: Option<&Utf8Path>,
    scale: f64,
    dir: &Utf8Path,
    refs: &[String],
) -> Vec<Option<f64>> {
    let recipe = path.and_then(|path| {
        let entry = crate::util::get_recipe(ctx.base_path(), path.as_str()).ok()?;
        // A reference through a link out of the collection is not looked up
        // either — its servings would come from a file outside it (#698).
        if entry
            .path()
            .is_some_and(|file| !inside_collection(ctx.base_path(), file))
        {
            return None;
        }
        crate::util::parse_recipe_from_entry(&entry, scale).ok()
    });
    let Some(recipe) = recipe else {
        return vec![None; refs.len()];
    };
    refs.iter()
        .map(|reference| {
            // Both sides are resolved first, as core does when it expands the
            // list: the recipe page stores `Sauces/Pesto` resolved, while older
            // lists and other clients hold it as written (`./Sauces/Pesto`,
            // `../Sauces/Pesto`), sometimes with `.cook`.
            let resolved = |path: &str| {
                let path = cookcli_core::resolve_reference(dir, path)?;
                let path = path.as_str();
                Some(path.strip_suffix(".cook").unwrap_or(path).to_string())
            };
            let wanted = resolved(reference)?;
            let uses: Vec<&Ingredient> = recipe
                .ingredients
                .iter()
                .filter(|ingredient| {
                    ingredient.reference.as_ref().is_some_and(|r| {
                        let mut parts = r.components.clone();
                        parts.push(r.name.clone());
                        resolved(&parts.join("/")).as_ref() == Some(&wanted)
                    })
                })
                .collect();
            if uses.is_empty() {
                return None;
            }
            let target = cookcli_core::resolve_reference(dir, reference)?;
            // The target's servings come from a file too: one outside the
            // collection is not read for them either (#698).
            let info = if cookcli_core::find::get_recipe_inside(ctx.base_path(), target.as_str())
                .is_ok()
            {
                ref_info_or_default(ctx.base_path(), target.as_str(), reference)
            } else {
                RecipeInfo::default()
            };
            reference_factor(combined_quantity(&uses).as_ref(), scale, &info)
        })
        .collect()
}

/// The recipe's metadata `title`, or its file stem when it declares none.
/// `None` when it cannot be read, so the caller keeps the name it has.
///
/// `path` must already be a safe relative path: `cooklang-find` joins it to
/// the recipe directory and asks the filesystem.
fn recipe_title(ctx: &cookcli_core::Context, path: &Utf8Path) -> Option<String> {
    // A stored reference that leaves the collection once resolved gets no
    // title from it — its file is not this server's to name (#698).
    cookcli_core::find::get_recipe_inside(ctx.base_path(), path.as_str()).ok()?;
    let request = cookcli_core::recipe::ReadRequest {
        source: cookcli_core::RecipeSource::Path(path.to_owned()),
        scale: 1.0,
    };
    match cookcli_core::recipe::read(ctx, request) {
        Ok(outcome) => Some(outcome.value.title).filter(|title| !title.is_empty()),
        Err(e) => {
            tracing::debug!("No title for shopping list entry '{path}': {e}");
            None
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AddItemRequest {
    pub path: String,
    pub scale: f64,
    /// Which sub-recipe references to include. `None` = all (menus, backward compat).
    pub included_references: Option<Vec<String>>,
    /// Names of the recipe's optional ingredients to buy too. The amounts are
    /// worked out from the recipe at `scale` and stored with them.
    #[serde(default)]
    pub optional_ingredients: Vec<String>,
}

pub async fn add_to_shopping_list(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(payload): Json<AddItemRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // Nothing is read here, but the path is persisted and resolved later —
    // by `/api/shopping_list` when the page posts it back, and by
    // `aggregate_current_ingredient_names` on the way to a compact. Refuse it
    // at the door rather than storing something those have to defend against.
    check_path(&payload.path)?;
    if !inside_collection(&state.base_path, &state.base_path.join(&payload.path)) {
        tracing::error!("{:?} resolves outside the recipe directory", payload.path);
        return Err((
            StatusCode::BAD_REQUEST,
            json_error(format!(
                "{} is not inside the recipe directory once links are followed",
                payload.path
            )),
        ));
    }
    // A bare name without an extension reaches its `.cook`/`.menu` through
    // cooklang-find — `Leak` finds `Leak.cook`, link or not — so it is
    // resolved here rather than on the joined path above, which sees nothing
    // until the extension is tried. A name that resolves to nothing is still
    // stored, as before; one that resolves outside is refused.
    if let Ok(entry) = cookcli_core::find::get_recipe(&state.base_path, &payload.path) {
        if entry
            .path()
            .is_some_and(|file| !inside_collection(&state.base_path, file))
        {
            tracing::error!("{:?} resolves outside the recipe directory", payload.path);
            return Err((
                StatusCode::BAD_REQUEST,
                json_error(format!(
                    "{} is not inside the recipe directory once links are followed",
                    payload.path
                )),
            ));
        }
    }

    let store = ShoppingListStore::new(&state.base_path);
    let added = format!(
        "added {}{} to the shopping list",
        activity::quoted(&payload.path),
        scaled(payload.scale)
    );
    let optional_ingredients = if payload.optional_ingredients.is_empty() {
        Vec::new()
    } else {
        let core_ctx = cookcli_core::Context::new(state.base_path.clone());
        let recipe = ScaledRecipe::scaled(
            cookcli_core::RecipeSource::Path(payload.path.as_str().into()),
            payload.scale,
        );
        optional_selections(&core_ctx, &recipe, &payload.optional_ingredients, true)
            .map_err(|e| {
                tracing::error!("Failed to read optional ingredients: {e}");
                (StatusCode::BAD_REQUEST, json_error(e))
            })?
            .value
    };

    // `name` is derived from `path` on load — any client-supplied display
    // name would be silently discarded, so it's not accepted here.
    let item = StoredEntry {
        name: recipe_display_name(&payload.path),
        path: payload.path,
        scale: payload.scale,
        included_references: payload.included_references,
        included_reference_names: None,
        included_reference_scales: None,
        recipes: None,
        optional_ingredients,
    };

    store.add(item).map_err(|e| {
        tracing::error!("Failed to add to shopping list: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    activity::record(&viewer, added);

    Ok(StatusCode::OK)
}

/// ` ×2` after what was added at another scale than 1, nothing otherwise.
fn scaled(scale: f64) -> String {
    if scale == 1.0 {
        String::new()
    } else {
        format!(" ×{scale}")
    }
}

#[derive(Debug, Deserialize)]
pub struct RemoveItemRequest {
    pub path: String,
    /// The entry's position in `GET /api/shopping_list/items`. The same
    /// recipe can be on the list twice, at another scale or with other
    /// sub-recipes, and the path alone cannot say which one to take. `None`
    /// takes the first entry with `path`.
    #[serde(default)]
    pub index: Option<usize>,
}

pub async fn remove_from_shopping_list(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(payload): Json<RemoveItemRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    let removed = match payload.index {
        Some(index) => store.remove_at(index, &payload.path),
        None => store.remove(&payload.path).map(|()| true),
    }
    .map_err(|e| {
        tracing::error!("Failed to remove from shopping list: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    if !removed {
        // The list changed since the page read it. Taking another entry with
        // the same path is what removed the wrong one, so refuse instead.
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "The shopping list changed; reload it and try again."
            })),
        ));
    }
    activity::record(
        &viewer,
        format_args!(
            "removed {} from the shopping list",
            activity::quoted(&payload.path)
        ),
    );

    compact_after_remove(&state, &store).await;

    Ok(StatusCode::OK)
}

/// Compact the checked log once something has left the list: stale checks
/// (ingredients nothing remaining asks for) can drop. Best-effort — a failure
/// here must not break the remove itself. Serialized against concurrent
/// check/uncheck/compact.
async fn compact_after_remove(state: &AppState, store: &ShoppingListStore) {
    let _guard = state.checked_log_lock.lock().await;
    match aggregate_current_ingredient_names(state) {
        Ok(names) => {
            if let Err(e) = store.compact(names) {
                tracing::warn!("Failed to compact checked log after remove: {:?}", e);
            }
        }
        Err(e) => tracing::warn!(
            "Skipping compact after remove — aggregation failed: {:?}",
            e
        ),
    }
}

/// The free-hand items on the list: lines no recipe put there, such as the
/// ingredients written straight into a menu whose days were added.
pub async fn get_extra_items(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<FreeHandItem>>, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    let items = store.load_free_hand().map_err(|e| {
        tracing::error!("Failed to load shopping list: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(items))
}

pub async fn remove_extra_item(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(payload): Json<FreeHandItem>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    store.remove_free_hand(&payload).map_err(|e| {
        tracing::error!("Failed to remove from shopping list: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    activity::record(
        &viewer,
        format_args!(
            "removed {} from the shopping list",
            activity::quoted(&payload.spec())
        ),
    );

    compact_after_remove(&state, &store).await;

    Ok(StatusCode::OK)
}

pub async fn clear_shopping_list(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // Acquire the checked-log lock so a concurrent check/uncheck can't
    // add a tick to `.shopping-checked` between our emptying it and the caller's
    // view of a cleared list.
    let _guard = state.checked_log_lock.lock().await;
    let store = ShoppingListStore::new(&state.base_path);
    store.clear().map_err(|e| {
        tracing::error!("Failed to clear shopping list: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    activity::record(&viewer, "cleared the shopping list");

    Ok(StatusCode::OK)
}

// -- Check/uncheck endpoints --

#[derive(Debug, Deserialize)]
pub struct CheckItemRequest {
    pub name: String,
}

impl CheckItemRequest {
    /// The ingredient's name, trimmed, as the checked log reads it back.
    ///
    /// A blank name only wrote a `+ ` line matching nothing, and a line break
    /// would turn one request into several log entries, so both are refused.
    fn ingredient(&self) -> Result<&str, (StatusCode, Json<serde_json::Value>)> {
        let name = self.name.trim();
        if name.is_empty() || name.contains(['\n', '\r']) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid ingredient name" })),
            ));
        }
        Ok(name)
    }
}

pub async fn check_shopping_item(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(payload): Json<CheckItemRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let name = payload.ingredient()?;
    let _guard = state.checked_log_lock.lock().await;
    let store = ShoppingListStore::new(&state.base_path);
    store.check(name).map_err(|e| {
        tracing::error!("Failed to check item: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    activity::record(
        &viewer,
        format_args!(
            "checked off {} on the shopping list",
            activity::quoted(name)
        ),
    );
    Ok(StatusCode::OK)
}

pub async fn uncheck_shopping_item(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(payload): Json<CheckItemRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let name = payload.ingredient()?;
    let _guard = state.checked_log_lock.lock().await;
    let store = ShoppingListStore::new(&state.base_path);
    store.uncheck(name).map_err(|e| {
        tracing::error!("Failed to uncheck item: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    activity::record(
        &viewer,
        format_args!("unchecked {} on the shopping list", activity::quoted(name)),
    );
    Ok(StatusCode::OK)
}

pub async fn get_checked_items(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    let checked = store.checked_set().map_err(|e| {
        tracing::error!("Failed to get checked items: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    Ok(Json(checked.into_iter().collect()))
}

pub async fn compact_checked(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.checked_log_lock.lock().await;
    let store = ShoppingListStore::new(&state.base_path);
    let names = aggregate_current_ingredient_names(&state).map_err(|e| {
        tracing::error!("Failed to aggregate ingredients for compact: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    store.compact(names).map_err(|e| {
        tracing::error!("Failed to compact checked list: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    activity::record(
        &viewer,
        "dropped the shopping list's checks for items no longer on it",
    );
    Ok(StatusCode::OK)
}

/// Aggregate the ingredient names a user would see for the currently-stored
/// shopping list. Walks every recipe reference persisted in `.shopping-list`
/// and expands it through `extract_ingredients`, honoring any
/// `included_references` and recipe scale factors.
///
/// Returns each name both as the recipes write it and as the aisle file's
/// common name, which is the one the page shows and ticks: comparing only the
/// raw names dropped the tick on `egg` when the recipes said `eggs`.
/// `compact_checked` compares case-insensitively.
///
/// Returns `Err(..)` if any recipe fails to parse. The caller should refuse
/// to compact in that case — a partial ingredient set would mark otherwise-
/// valid checks as stale and wipe them, which is how the original bug this
/// module was fixing manifested.
fn aggregate_current_ingredient_names(state: &AppState) -> anyhow::Result<Vec<String>> {
    let store = ShoppingListStore::new(&state.base_path);
    let items = store.load()?;
    let mut list = ShoppingIngredients::default();
    let core_ctx = cookcli_core::Context::new(state.base_path.clone());

    // Each entry is aggregated independently: the shopping list may
    // legitimately contain the same recipe more than once (e.g. duplicate
    // entries from the legacy format), and that is not a cycle.
    let mut add = |path: &str,
                   scale: f64,
                   included: Option<&[String]>,
                   selections: &[OptionalSelection]|
     -> anyhow::Result<()> {
        // `.shopping-list` is a file, and one written before these endpoints
        // checked what they stored can still hold a path that leaves the
        // recipe directory — lexically, or through a link once resolved.
        // Skipping it costs the compact the ingredients of an entry that was
        // never a recipe in this collection; following it would read
        // whatever it names (#698).
        if !crate::util::is_safe_relative_path(path)
            || !inside_collection(&state.base_path, &state.base_path.join(path))
        {
            tracing::warn!(
                "Ignoring stored shopping list path outside the recipe directory: {path}"
            );
            return Ok(());
        }
        extract_ingredients(
            &core_ctx,
            &ScaledRecipe::scaled(cookcli_core::RecipeSource::Path(path.into()), scale),
            &ExtractOptions {
                ignore_references: false,
                included_references: included,
                include_optional: false,
                confined: true,
            },
            &mut list,
        )
        .with_context(|| format!("aggregating ingredients for {path} at scale {scale}"))?;
        add_optional_selections(&mut list, selections);
        Ok(())
    };

    for item in &items {
        if let Some(recipes) = &item.recipes {
            // Menu/plan entry — expand each nested recipe.
            for recipe in recipes {
                add(
                    &recipe.path,
                    recipe.scale,
                    recipe.included_references.as_deref(),
                    &recipe.optional_ingredients,
                )?;
            }
        } else {
            add(
                &item.path,
                item.scale,
                item.included_references.as_deref(),
                &item.optional_ingredients,
            )?;
        }
    }

    // Free-hand items are ticked like any other ingredient, so a tick on
    // them survives the compaction too.
    for item in store.load_free_hand()? {
        add_extra_item(&item.spec(), &mut list.required)
            .with_context(|| format!("aggregating free-hand item {}", item.spec()))?;
    }

    // The page lists, and so ticks, each ingredient under the name the aisle
    // file gives it (`egg` for a recipe's `eggs`). Keep the recipes' own names
    // as well: a tick made under one, before the aisle file grouped it, still
    // stands for an ingredient on the list. Chosen optional ingredients are
    // items of their own, ticked like the rest.
    let aisle_content = read_aisle_file(state);
    let aisle = cooklang::aisle::parse_lenient(&aisle_content)
        .output()
        .cloned()
        .unwrap_or_default();
    let mut names = Vec::new();
    // Merge case variants first, so `sugar`/`Sugar` tick as one name (#694).
    for part in [
        merge_case_variants(list.required),
        merge_case_variants(list.optional),
    ] {
        names.extend(part.iter().map(|(name, _)| name.clone()));
        names.extend(
            part.use_common_names(&aisle, PARSER.converter())
                .iter()
                .map(|(name, _)| name.clone()),
        );
    }
    Ok(names)
}

/// The aisle file's text; empty when there is none or it can't be read, which
/// the lenient aisle parser reads as no aisles at all.
fn read_aisle_file(state: &AppState) -> String {
    let Some(path) = state.aisle_file() else {
        tracing::debug!("No aisle file configured");
        return String::new();
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            tracing::debug!("Loaded aisle file from: {:?}", path);
            content
        }
        Err(e) => {
            tracing::warn!("Failed to read aisle file from {:?}: {}", path, e);
            String::new()
        }
    }
}

// -- Add menu (bulk) endpoint --

#[derive(Debug, Deserialize)]
pub struct AddMenuRequest {
    pub path: String,
    pub scale: f64,
    /// Only the days with these dates (`YYYY-MM-DD`), as the menu's dated
    /// sections name them. Without it the whole menu is added.
    #[serde(default)]
    pub dates: Option<Vec<String>>,
}

/// The most days one request may ask for: a year and then some.
const MAX_DAYS: usize = 400;

/// Add a menu to the shopping list.
///
/// The whole menu goes on as a single entry with its recipes nested inside,
/// its own free-hand ingredients read from the menu when the list is made.
///
/// Some of its days (`dates`) cannot be stored that way: the format reads a
/// menu entry as the whole menu plus the listed recipes, so every day's
/// free-hand ingredients would come along. Those days' recipes go on as
/// recipe entries instead, as if added one by one, and their free-hand
/// ingredients as free-hand lines — both native to the format, so the Cooklang
/// apps read them too.
pub async fn add_menu_to_shopping_list(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(payload): Json<AddMenuRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    check_path(&payload.path)?;
    let dates = payload.dates.as_deref().map(chosen_dates).transpose()?;

    let store = ShoppingListStore::new(&state.base_path);
    let menu_scale = payload.scale;

    let recipe_path = Utf8PathBuf::from(&payload.path);
    let entry = cooklang_find::get_recipe(vec![&state.base_path], &recipe_path).map_err(|e| {
        tracing::error!("Menu not found: {}", payload.path);
        (
            StatusCode::NOT_FOUND,
            json_error(format!("Menu not found: {}: {}", payload.path, e)),
        )
    })?;

    // `get_recipe` follows links, so `Link/Week.menu` through a
    // `Link -> /outside` symlink is a hit (#698).
    if entry
        .path()
        .is_some_and(|file| !inside_collection(&state.base_path, file))
    {
        tracing::error!("{:?} resolves outside the recipe directory", payload.path);
        return Err((
            StatusCode::BAD_REQUEST,
            json_error(format!(
                "{} is not inside the recipe directory once links are followed",
                payload.path
            )),
        ));
    }

    // References come back with their factor already resolved: `{target%unit}`
    // against the referenced recipe's servings/yield, times the menu scale.
    let menu = crate::util::menu::load_menu(&entry, &recipe_path, &state.base_path, menu_scale)
        .map_err(|e| {
            tracing::error!("Failed to parse menu: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!("Failed to parse menu: {e}")),
            )
        })?;

    let mut recipes = Vec::new();

    for section in &menu.sections {
        // With a choice of days, only the sections dated one of them.
        if dates
            .as_ref()
            .is_some_and(|dates| !on_a_chosen_day(section.name.as_deref(), dates))
        {
            continue;
        }
        for item in section.meals.iter().flat_map(|meal| &meal.items) {
            let cooklang_find::MenuItem::RecipeReference {
                path,
                scale,
                optional,
                ..
            } = item
            else {
                continue;
            };
            // An optional recipe in a menu (`@?./Desserts/Cake{}`) adds nothing
            // to the list unless someone accepts it, and a menu is added whole.
            if *optional {
                continue;
            }
            if crate::util::menu::is_outside_reference(path) {
                tracing::warn!(
                    "Skipping recipe reference '{}' in menu '{}': it points outside the \
                     recipe directory",
                    path,
                    payload.path
                );
                continue;
            }
            // Lexically inside is not containment either — `Linked/Secret`
            // is a link out. Resolving it here keeps that file's own
            // references from being stored as list sub-entries (#698).
            let lands_outside = cookcli_core::find::get_recipe(&state.base_path, path)
                .ok()
                .and_then(|entry| entry.path().cloned())
                .is_some_and(|file| !inside_collection(&state.base_path, &file));
            if lands_outside {
                tracing::warn!(
                    "Skipping recipe reference '{}' in menu '{}': it resolves outside the \
                     recipe directory",
                    path,
                    payload.path
                );
                continue;
            }

            // Resolve this recipe's sub-recipe references
            let info = match resolve_recipe_info(&state.base_path, path) {
                Ok(info) => info,
                Err(e) => {
                    tracing::warn!("Could not resolve referenced recipe '{}': {}", path, e);
                    RecipeInfo::default()
                }
            };
            let mut sub_refs = info.sub_refs;

            // A menu used as a meal of this one (`@./Brunches/Sunday.menu{}`)
            // is listed like a recipe, its references followed in a request
            // of its own. A reference of it back to this menu would count this
            // menu's contents a second time there, where nothing knows it is
            // already on the list, so it is dropped here instead.
            if info.is_menu {
                let this_menu = menu_stem(&payload.path);
                let nested_dir = Utf8Path::new(path)
                    .parent()
                    .map(Utf8Path::to_owned)
                    .unwrap_or_default();
                sub_refs.retain(|sub_ref| {
                    let back = cookcli_core::resolve_reference(&nested_dir, sub_ref)
                        .is_some_and(|p| menu_stem(p.as_str()) == this_menu);
                    if back {
                        tracing::warn!(
                            "Skipping reference '{}' in menu '{}': it leads back to '{}'",
                            sub_ref,
                            path,
                            payload.path
                        );
                    }
                    !back
                });
            }

            recipes.push(StoredEntry {
                name: recipe_display_name(path),
                path: path.clone(),
                scale: scale.unwrap_or(menu_scale),
                included_references: Some(sub_refs),
                included_reference_names: None,
                included_reference_scales: None,
                recipes: None,
                optional_ingredients: Vec::new(),
            });
        }
    }

    let failed = |e: cookcli_core::CoreError| {
        tracing::error!("Failed to add menu to shopping list: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    };

    let Some(dates) = dates else {
        let added = format!(
            "added menu {}{} to the shopping list",
            activity::quoted(&payload.path),
            scaled(menu_scale)
        );
        store
            .add_menu(payload.path, menu_scale, recipes)
            .map_err(failed)?;
        activity::record(&viewer, added);
        return Ok(StatusCode::OK);
    };

    // The chosen days' free-hand ingredients, at the menu's scale. They are
    // read from the Cooklang parse of the menu, whose quantities the scaling
    // has already multiplied and can be added up by unit.
    let scaled_menu = crate::util::parse_recipe_from_entry(&entry, menu_scale).map_err(|e| {
        tracing::error!("Failed to parse menu: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error(format!("Failed to parse menu: {e}")),
        )
    })?;
    let items = free_hand_items(&scaled_menu, &ingredients_on_days(&scaled_menu, &dates));

    let added = format!(
        "added {} of menu {}{} to the shopping list",
        match dates.len() {
            1 => "1 day".to_string(),
            n => format!("{n} days"),
        },
        activity::quoted(&payload.path),
        scaled(menu_scale)
    );
    store.add_all(recipes, items).map_err(failed)?;
    activity::record(&viewer, added);

    Ok(StatusCode::OK)
}

/// A menu's path as `add_menu` compares them: without a leading `./` or the
/// `.menu` extension, either of which a reference may be written with.
fn menu_stem(path: &str) -> &str {
    let path = path.strip_prefix("./").unwrap_or(path);
    path.strip_suffix(".menu").unwrap_or(path)
}

/// The dates a request chose, checked: each a `YYYY-MM-DD` date, at least one,
/// at most [`MAX_DAYS`].
fn chosen_dates(dates: &[String]) -> Result<BTreeSet<String>, ApiError> {
    let refuse = |message: String| (StatusCode::BAD_REQUEST, json_error(message));
    if dates.is_empty() {
        return Err(refuse("No days to add".to_string()));
    }
    if dates.len() > MAX_DAYS {
        return Err(refuse(format!("At most {MAX_DAYS} days at a time")));
    }
    dates
        .iter()
        .map(|date| {
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map(|day| day.format("%Y-%m-%d").to_string())
                .map_err(|_| refuse(format!("Not a date (YYYY-MM-DD): {date}")))
        })
        .collect()
}

/// Whether a menu section with this name is one of the chosen days, dated as
/// the calendar dates it: by a `YYYY-MM-DD` anywhere in the name.
fn on_a_chosen_day(section_name: Option<&str>, dates: &BTreeSet<String>) -> bool {
    section_name
        .and_then(crate::web::plan::section_date)
        .is_some_and(|(date, _)| dates.contains(&date.format("%Y-%m-%d").to_string()))
}

/// The indices of the ingredients written in the menu's sections dated one of
/// `dates`, as the calendar dates them — recipe references and free-hand
/// ingredients alike.
fn ingredients_on_days(menu: &Recipe, dates: &BTreeSet<String>) -> BTreeSet<usize> {
    let mut chosen = BTreeSet::new();
    for section in &menu.sections {
        if !on_a_chosen_day(section.name.as_deref(), dates) {
            continue;
        }
        for content in &section.content {
            if let cooklang::Content::Step(step) = content {
                for item in &step.items {
                    if let cooklang::Item::Ingredient { index } = item {
                        chosen.insert(*index);
                    }
                }
            }
        }
    }
    chosen
}

/// The free-hand ingredients among `chosen`, as shopping list lines: the ones
/// a list made from the whole menu would show (not recipe references, not
/// hidden), merged by name, in the order the menu first names them.
fn free_hand_items(menu: &Recipe, chosen: &BTreeSet<usize>) -> Vec<FreeHandItem> {
    let converter = PARSER.converter();
    let mut list = IngredientList::new();
    for &index in chosen {
        let Some(ingredient) = menu.ingredients.get(index) else {
            continue;
        };
        if ingredient.reference.is_some() || !ingredient.modifiers().should_be_listed() {
            continue;
        }
        let mut quantity = GroupedQuantity::empty();
        if let Some(q) = &ingredient.quantity {
            quantity.add(q, converter);
        }
        list.add_ingredient(ingredient.display_name().into_owned(), &quantity, converter);
    }

    let mut items = Vec::new();
    for (name, quantity) in list.iter() {
        let components = crate::util::format::quantity::ordered_components(quantity);
        if components.is_empty() {
            items.push(FreeHandItem {
                name: name.clone(),
                quantity: None,
            });
        }
        for component in components {
            items.push(FreeHandItem {
                name: name.clone(),
                quantity: Some(quantity_spec(component)),
            });
        }
    }
    items
}

/// A quantity as the shopping list format writes one: `50%g`, `1/2%cup`,
/// `2-3`, `2`.
fn quantity_spec(quantity: &cooklang::Quantity) -> String {
    use crate::util::format::number::format_number;
    let value = match quantity.value() {
        cooklang::Value::Number(n) => format_number(n.value()),
        cooklang::Value::Range { start, end } => format!(
            "{}-{}",
            format_number(start.value()),
            format_number(end.value())
        ),
        cooklang::Value::Text(text) => text.clone(),
    };
    match quantity.unit() {
        Some(unit) => format!("{value}%{unit}"),
        None => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingredient(name: &str) -> Option<String> {
        CheckItemRequest {
            name: name.to_string(),
        }
        .ingredient()
        .ok()
        .map(str::to_string)
    }

    #[test]
    fn a_checked_name_is_trimmed() {
        assert_eq!(ingredient("  olive oil "), Some("olive oil".to_string()));
    }

    #[test]
    fn a_blank_or_multi_line_name_is_refused() {
        for name in ["", "   ", "milk\n- eggs", "milk\r+ eggs"] {
            assert_eq!(ingredient(name), None, "{name:?}");
        }
    }

    fn menu(text: &str) -> Recipe {
        PARSER.parse(text).into_output().expect("the menu parses")
    }

    fn dates(dates: &[&str]) -> BTreeSet<String> {
        dates.iter().map(|d| d.to_string()).collect()
    }

    const MENU: &str = "== Mon (2026-10-05) ==\n\nLunch: \\\n- @./Soup{} \\\n- @almonds{50%g}\n\n\
        == Tue (2026-10-06) ==\n\n- @bread{1%loaf}\n\n\
        == Wed (2026-10-07) ==\n\n- @almonds{1/2%cup} \\\n- @almonds{25%g} \\\n- @pepper\n\n\
        = 2026-10-08 Dinner\n\n- @rice{200%g}\n\n\
        == Later ==\n\n- @eggs{6}\n";

    fn names(menu: &Recipe, chosen: &BTreeSet<usize>) -> Vec<String> {
        chosen
            .iter()
            .map(|&i| menu.ingredients[i].name.to_string())
            .collect()
    }

    #[test]
    fn only_the_sections_of_the_chosen_dates_count() {
        let menu = menu(MENU);
        let chosen = ingredients_on_days(&menu, &dates(&["2026-10-05", "2027-01-01"]));
        assert_eq!(names(&menu, &chosen), ["Soup", "almonds"]);

        // A date counts anywhere in the section's name, as on the calendar.
        let chosen = ingredients_on_days(&menu, &dates(&["2026-10-08"]));
        assert_eq!(names(&menu, &chosen), ["rice"]);

        assert!(ingredients_on_days(&menu, &dates(&["2026-10-09"])).is_empty());
    }

    #[test]
    fn free_hand_items_merge_by_name_and_keep_units_apart() {
        let menu = menu(MENU);
        let chosen = ingredients_on_days(&menu, &dates(&["2026-10-05", "2026-10-07"]));
        let items: Vec<String> = free_hand_items(&menu, &chosen)
            .iter()
            .map(FreeHandItem::spec)
            .collect();
        // The recipe reference is not on the list; units that cannot be
        // added up stay apart.
        assert_eq!(items, ["almonds{1/2%cup}", "almonds{75%g}", "pepper{}"]);
    }

    #[test]
    fn chosen_dates_are_checked() {
        assert_eq!(
            chosen_dates(&["2026-10-07".into(), "2026-10-05".into()]).unwrap(),
            dates(&["2026-10-05", "2026-10-07"])
        );
        assert!(chosen_dates(&[]).is_err());
        assert!(chosen_dates(&["2026-13-01".into()]).is_err());
        assert!(chosen_dates(&vec!["2026-10-05".to_string(); MAX_DAYS + 1]).is_err());
    }
}
