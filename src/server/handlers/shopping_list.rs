use crate::server::handlers::common::{check_path, json_error};
use crate::server::{activity, AppState};
use crate::util::recipe_info::{resolve_recipe_info, RecipeInfo};
use crate::util::PARSER;
use crate::web::viewer::Viewer;
use anyhow::Context as _;
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use camino::{Utf8Path, Utf8PathBuf};
use cookcli_core::shopping_list::{
    extract_ingredients, recipe_display_name, ExtractOptions, ScaledRecipe, ShoppingIngredients,
    ShoppingListStore, StoredEntry,
};
use cooklang::ingredient_list::IngredientList;
use serde::Deserialize;
use serde_json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct RecipeRequest {
    recipe: String,
    scale: Option<f64>,
    /// Which sub-recipe references to include. `None` = all.
    included_references: Option<Vec<String>>,
}

pub async fn shopping_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(payload): axum::extract::Json<Vec<RecipeRequest>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut ingredients = ShoppingIngredients::default();
    let core_ctx = cookcli_core::Context::new(state.base_path.clone());

    for entry in payload {
        let name = entry.recipe;
        // Straight to `cooklang_find`, which joins it to the recipe directory
        // and asks the filesystem. Checked before that happens, not after:
        // on Windows, merely looking up `\\host\share` hands that host the
        // user's NTLM hash.
        check_path(&name)?;
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
                // The web list has no way to choose optional ingredients yet,
                // so it keeps listing them alongside the required ones.
                include_optional: true,
            },
            &mut ingredients,
        )
        .map_err(|e| {
            tracing::error!("Error processing recipe: {}", e);
            (StatusCode::BAD_REQUEST, json_error(e))
        })?;

        for diagnostic in diagnostics {
            tracing::warn!("Recipe '{}': {}", name, diagnostic.message);
        }
    }

    let mut list = merged(ingredients);

    // Parse aisle with lenient parsing
    let aisle_content = read_aisle_file(&state);
    let aisle_result = cooklang::aisle::parse_lenient(&aisle_content);

    if aisle_result.report().has_warnings() {
        for warning in aisle_result.report().warnings() {
            tracing::warn!("Aisle configuration warning: {}", warning);
        }
    }

    let aisle = aisle_result.output().cloned().unwrap_or_default();

    // Load pantry configuration
    let pantry_conf = if let Some(path) = &state.pantry_path {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                tracing::debug!("Loaded pantry file from: {:?}", path);
                // Parse pantry file using cooklang pantry parser
                let result = cooklang::pantry::parse_lenient(&content);

                if result.report().has_warnings() {
                    for warning in result.report().warnings() {
                        tracing::warn!("Pantry configuration warning: {}", warning);
                    }
                }

                result.output().cloned()
            }
            Err(e) => {
                tracing::warn!("Failed to read pantry file from {:?}: {}", path, e);
                None
            }
        }
    } else {
        tracing::debug!("No pantry file configured");
        None
    };

    // Use common names from aisle configuration
    list = list.use_common_names(&aisle, PARSER.converter());

    // Track pantry items that were found and subtracted (excluding zero quantities)
    let mut pantry_items = Vec::new();
    if let Some(ref pantry) = pantry_conf {
        // Check which items from the original list are in the pantry with non-zero quantity
        for (ingredient_name, _) in list.iter() {
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

    // Apply pantry subtraction if pantry is available
    let final_list = if let Some(ref pantry) = pantry_conf {
        list.subtract_pantry(pantry, PARSER.converter())
    } else {
        list
    };

    let categories = final_list.categorize(&aisle);

    // Build the response
    let mut shopping_categories = Vec::new();

    for (category, items) in categories {
        let mut entries: Vec<(String, _)> = items.into_iter().collect();

        // The "other" bucket holds ingredients with no aisle category. They
        // arrive in recipe insertion order, which is unhelpful when scanning
        // a long list — sort alphabetically (case-insensitive) so shoppers
        // can find items predictably.
        if category == "other" {
            entries.sort_by_key(|(a, _)| a.to_lowercase());
        }

        let mut shopping_items = Vec::new();
        for (name, qty) in entries {
            let item_json = serde_json::json!({
                "name": name,
                // Not `into_vec()`: that yields the components in the group's
                // own random order, so an ingredient measured two ways came
                // back differently on every request.
                "quantities": crate::util::format::quantity::ordered_components(&qty)
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
        "checked": checked.into_iter().collect::<Vec<_>>()
    });
    Ok(Json(json_value))
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
/// a recipe retitled since, still shows the current title.
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
        }

        if let Some(recipes) = &mut entry.recipes {
            name_by_title(ctx, recipes);
        }
    }
}

/// The recipe's metadata `title`, or its file stem when it declares none.
/// `None` when it cannot be read, so the caller keeps the name it has.
///
/// `path` must already be a safe relative path: `cooklang-find` joins it to
/// the recipe directory and asks the filesystem.
fn recipe_title(ctx: &cookcli_core::Context, path: &Utf8Path) -> Option<String> {
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

    let store = ShoppingListStore::new(&state.base_path);
    let added = format!(
        "added {}{} to the shopping list",
        activity::quoted(&payload.path),
        scaled(payload.scale)
    );
    // `name` is derived from `path` on load — any client-supplied display
    // name would be silently discarded, so it's not accepted here.
    let item = StoredEntry {
        name: recipe_display_name(&payload.path),
        path: payload.path,
        scale: payload.scale,
        included_references: payload.included_references,
        included_reference_names: None,
        recipes: None,
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

    // Compact the checked log now that one recipe is gone: stale checks
    // (ingredients no longer referenced by any remaining recipe) can drop.
    // Best-effort — a failure here must not break the remove itself.
    // Serialize against concurrent check/uncheck/compact.
    let _guard = state.checked_log_lock.lock().await;
    match aggregate_current_ingredient_names(&state) {
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

    Ok(StatusCode::OK)
}

pub async fn clear_shopping_list(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // Acquire the checked-log lock so a concurrent check/uncheck can't
    // recreate `.shopping-checked` between our remove_file and the caller's
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
    let mut add = |path: &str, scale: f64, included: Option<&[String]>| -> anyhow::Result<()> {
        // `.shopping-list` is a file, and one written before these endpoints
        // checked what they stored can still hold a path that leaves the
        // recipe directory. Skipping it costs the compact the ingredients of
        // an entry that was never a recipe in this collection; following it
        // would read whatever it names.
        if !crate::util::is_safe_relative_path(path) {
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
                include_optional: true,
            },
            &mut list,
        )
        .with_context(|| format!("aggregating ingredients for {path} at scale {scale}"))?;
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
                )?;
            }
        } else {
            add(&item.path, item.scale, item.included_references.as_deref())?;
        }
    }

    // The page lists, and so ticks, each ingredient under the name the aisle
    // file gives it (`egg` for a recipe's `eggs`). Keep the recipes' own names
    // as well: a tick made under one, before the aisle file grouped it, still
    // stands for an ingredient on the list.
    let list = merged(list);
    let mut names: Vec<String> = list.iter().map(|(name, _)| name.clone()).collect();
    let aisle_content = read_aisle_file(state);
    let aisle = cooklang::aisle::parse_lenient(&aisle_content)
        .output()
        .cloned()
        .unwrap_or_default();
    names.extend(
        list.use_common_names(&aisle, PARSER.converter())
            .iter()
            .map(|(name, _)| name.clone()),
    );
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

/// Fold the optional ingredients into the required ones, the way the web list
/// has always shown them.
fn merged(ingredients: ShoppingIngredients) -> IngredientList {
    let ShoppingIngredients {
        mut required,
        optional,
    } = ingredients;
    for (name, quantity) in optional.iter() {
        required.add_ingredient(name.clone(), quantity, PARSER.converter());
    }
    required
}

// -- Add menu (bulk) endpoint --

#[derive(Debug, Deserialize)]
pub struct AddMenuRequest {
    pub path: String,
    pub scale: f64,
}

/// Add all recipe references from a menu to the shopping list as a single
/// plan entry with recipes nested inside.
pub async fn add_menu_to_shopping_list(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(payload): Json<AddMenuRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    check_path(&payload.path)?;

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
        for item in section.meals.iter().flat_map(|meal| &meal.items) {
            let cooklang_find::MenuItem::RecipeReference { path, scale, .. } = item else {
                continue;
            };
            if crate::util::menu::is_outside_reference(path) {
                tracing::warn!(
                    "Skipping recipe reference '{}' in menu '{}': it points outside the \
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
                recipes: None,
            });
        }
    }

    let added = format!(
        "added menu {}{} to the shopping list",
        activity::quoted(&payload.path),
        scaled(menu_scale)
    );
    store
        .add_menu(payload.path, menu_scale, recipes)
        .map_err(|e| {
            tracing::error!("Failed to add menu to shopping list: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
        })?;
    activity::record(&viewer, added);

    Ok(StatusCode::OK)
}

/// A menu's path as `add_menu` compares them: without a leading `./` or the
/// `.menu` extension, either of which a reference may be written with.
fn menu_stem(path: &str) -> &str {
    let path = path.strip_prefix("./").unwrap_or(path);
    path.strip_suffix(".menu").unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::CheckItemRequest;

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
}
