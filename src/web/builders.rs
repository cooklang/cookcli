//! Template builders shared between the dynamic web server and the static-site
//! renderer. Each function takes plain inputs and produces an Askama template
//! struct ready to render (or be turned into an `axum::Response` by a handler).
//!
//! The builders intentionally avoid any axum / tokio-async types so they can be
//! reused from a non-async context (e.g. `cook build web`).

use crate::util::recipe_info::{ref_info_or_default, RecipeInfo};
use crate::web::language::FeatureFlags;
use crate::web::templates::*;
use crate::web::viewer::Viewer;
use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use cooklang_find::MenuItem;
use fluent_templates::Loader;
use unic_langid::LanguageIdentifier;

/// Inputs for [`build_recipes_template`].
pub struct RecipesBuildInput<'a> {
    pub base_path: &'a Utf8Path,
    pub url_prefix: &'a str,
    pub sub_path: Option<&'a str>,
    pub lang: LanguageIdentifier,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    pub viewer: Viewer,
}

/// Build a [`RecipesTemplate`] for either the root or a subdirectory.
pub fn build_recipes_template(input: RecipesBuildInput<'_>) -> Result<RecipesTemplate> {
    let RecipesBuildInput {
        base_path,
        url_prefix,
        sub_path,
        lang,
        static_mode,
        repo_url,
        features,
        viewer,
    } = input;

    let search_path = if let Some(p) = sub_path {
        // On the server `p` comes straight from the `/directory/{*path}` URL.
        if !crate::util::is_safe_relative_path(p) {
            anyhow::bail!("Invalid path: {p}");
        }
        base_path.join(p)
    } else {
        base_path.to_path_buf()
    };

    let tree = cooklang_find::build_tree(&search_path)
        .map_err(|e| anyhow::anyhow!("Failed to build recipe tree: {e}"))?;

    let mut items = Vec::new();

    for (name, child) in &tree.children {
        let is_dir = !child.children.is_empty();
        let item_path = {
            let url_path = child
                .recipe
                .as_ref()
                .and_then(|recipe| recipe.file_name())
                .map(|f| {
                    f.trim_end_matches(".cook")
                        .trim_end_matches(".menu")
                        .to_string()
                })
                .unwrap_or_else(|| name.to_string());

            match sub_path {
                Some(p) => format!("{p}/{url_path}"),
                None => url_path.to_string(),
            }
        };

        // Extract tags, image, is_menu, and file timestamps if this is a recipe
        let (tags, image_path, is_menu, modified_at, created_at) =
            if let Some(ref recipe) = child.recipe {
                let img_path = recipe
                    .title_image()
                    .clone()
                    .and_then(|img| get_image_path(base_path, url_prefix, img));

                let (modified_at, created_at) = recipe
                    .path()
                    .map(|p| {
                        let meta = std::fs::metadata(p).ok();
                        let modified = meta
                            .as_ref()
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs());
                        let created = meta
                            .as_ref()
                            .and_then(|m| m.created().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs());
                        (modified, created)
                    })
                    .unwrap_or((None, None));

                (
                    recipe.tags(),
                    img_path,
                    recipe.is_menu(),
                    modified_at,
                    created_at,
                )
            } else {
                (Vec::new(), None, false, None, None)
            };

        items.push(RecipeItem {
            name: name.to_string(),
            path: item_path,
            is_directory: is_dir,
            count: if is_dir {
                count_recipes_tree(child)
            } else {
                None
            },
            description: None,
            tags,
            image_path,
            is_menu,
            modified_at,
            created_at,
        });
    }

    items.sort_by(|a, b| match (a.is_directory, b.is_directory) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => natural_cmp(&a.name, &b.name),
    });

    let todays_menu = if sub_path.is_none() {
        crate::web::menus::find_todays_menu(base_path)
    } else {
        None
    };

    let breadcrumbs = if let Some(p) = sub_path {
        p.split('/')
            .scan(String::new(), |acc, segment| {
                if !acc.is_empty() {
                    acc.push('/');
                }
                acc.push_str(segment);
                Some(Breadcrumb {
                    name: segment.to_string(),
                    path: acc.clone(),
                })
            })
            .collect()
    } else {
        vec![]
    };

    let current_name = if let Some(p) = sub_path {
        p.split('/').next_back().unwrap_or("Recipes").to_string()
    } else {
        crate::web::i18n::LOCALES.lookup(&lang, "recipes-title")
    };

    let new_recipe_url = match sub_path {
        Some(p) => format!("{url_prefix}/new?filename={}%2F", urlencoding::encode(p)),
        None => format!("{url_prefix}/new"),
    };
    let new_menu_url = match sub_path {
        Some(p) => format!(
            "{url_prefix}/new?kind=menu&filename={}%2F",
            urlencoding::encode(p)
        ),
        None => format!("{url_prefix}/new?kind=menu"),
    };

    // The pick happens on the server (`/random`), so a static site has no
    // button; neither does a folder with nothing but menus in it.
    let random_recipe_url =
        (!static_mode && !cook_recipe_paths(&tree).is_empty()).then(|| match sub_path {
            Some(p) => format!("{url_prefix}/random/{}", crate::util::encode_url_path(p)),
            None => format!("{url_prefix}/random"),
        });

    Ok(RecipesTemplate {
        active: "recipes".to_string(),
        current_name,
        breadcrumbs,
        items,
        todays_menu,
        new_recipe_url,
        new_menu_url,
        random_recipe_url,
        tr: Tr::new(lang),
        prefix: url_prefix.to_string(),
        static_mode,
        repo_url,
        features,
        viewer,
    })
}

/// One chunk of a name for natural sorting: a run of digits compared by
/// value, or a run of anything else compared as lowercase text.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NaturalChunk {
    Number(u128),
    Text(String),
}

/// Split a name into digit runs and non-digit runs, lowercased, so that
/// "Recipe 9" sorts before "Recipe 10" and case does not split otherwise
/// equal names. This folds case and digits like the client-side
/// `Intl.Collator` with `{ numeric: true, sensitivity: 'base' }` in
/// `templates/recipes.html`, but not accents: non-ASCII letters keep code
/// point order, so "Äpfel" sorts after "zucchini". The client therefore
/// leaves the served order alone on the default sort and only re-sorts
/// when the user picks another field or direction.
fn natural_key(name: &str) -> Vec<NaturalChunk> {
    let mut chunks = Vec::new();
    let mut text = String::new();
    let mut digits = String::new();

    let flush_text = |text: &mut String, chunks: &mut Vec<NaturalChunk>| {
        if !text.is_empty() {
            chunks.push(NaturalChunk::Text(std::mem::take(text)));
        }
    };
    let flush_digits = |digits: &mut String, chunks: &mut Vec<NaturalChunk>| {
        if !digits.is_empty() {
            // Cap absurdly long digit runs instead of panicking on overflow.
            let value = digits.parse::<u128>().unwrap_or(u128::MAX);
            digits.clear();
            chunks.push(NaturalChunk::Number(value));
        }
    };

    for c in name.chars() {
        if c.is_ascii_digit() {
            flush_text(&mut text, &mut chunks);
            digits.push(c);
        } else {
            flush_digits(&mut digits, &mut chunks);
            text.extend(c.to_lowercase());
        }
    }
    flush_text(&mut text, &mut chunks);
    flush_digits(&mut digits, &mut chunks);
    chunks
}

/// Case-insensitive natural ordering. Ties (names that differ only in case
/// or leading zeros) fall back to byte order so the sort stays deterministic.
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    natural_key(a).cmp(&natural_key(b)).then_with(|| a.cmp(b))
}

/// Inputs for [`build_recipe_template`].
pub struct RecipeBuildInput<'a> {
    pub base_path: &'a Utf8Path,
    pub url_prefix: &'a str,
    pub recipe_path: &'a str,
    pub aisle_path: Option<&'a Utf8PathBuf>,
    pub scale: f64,
    /// `?servings=`: wins over `scale` on a recipe that declares servings,
    /// ignored everywhere else (menus, recipes without servings).
    pub servings: Option<f64>,
    pub lang: LanguageIdentifier,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    pub viewer: Viewer,
}

/// Output of [`build_recipe_template`] — either a regular recipe or a menu.
pub enum RecipeBuildOutput {
    Recipe(Box<RecipeTemplate>),
    Menu(Box<MenuTemplate>),
}

/// Converts a timer's quantity to a whole number of seconds, if its unit is a
/// recognized time unit. Returns `None` for ranges, text values, or units this
/// simple lookup doesn't recognize.
fn timer_duration_seconds(quantity: &cooklang::quantity::Quantity) -> Option<i64> {
    let cooklang::Value::Number(n) = quantity.value() else {
        return None;
    };
    let multiplier = match quantity.unit()?.to_lowercase().as_str() {
        "s" | "sec" | "secs" | "second" | "seconds" => 1.0,
        "m" | "min" | "mins" | "minute" | "minutes" => 60.0,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3600.0,
        _ => return None,
    };
    Some((n.value() * multiplier).round() as i64)
}

/// The factor a recipe or menu page applies, and its servings stepper.
///
/// A recipe that declares a whole, non-zero number of `servings` gets the
/// stepper, which counts servings: `?servings=` sets them and wins over
/// `?scale=`; without it the multiplier from `?scale=` still applies, shown as
/// the servings it gives, so existing `?scale=` links keep their meaning.
/// Anything else — no `servings`, text such as `4-6`, a fraction (cooklang
/// only reads `u32`), or `0` — keeps the plain multiplier and returns no
/// stepper.
///
/// Any positive `?servings=` is taken as is, like `?scale=`: the stepper's
/// half-a-serving floor is for what people type, while a menu can link to 0.4
/// of a 4-serving recipe.
fn resolve_servings(
    base: Option<u32>,
    servings: Option<f64>,
    scale: f64,
) -> (f64, Option<ServingsScale>) {
    let Some(base) = base.filter(|&n| n > 0) else {
        return (scale, None);
    };
    let base_f = f64::from(base);
    let (scale, chosen) = match servings.filter(|s| s.is_finite() && *s > 0.0) {
        Some(s) => (s / base_f, s),
        None => (scale, base_f * scale),
    };
    // Rounded for display only (`0.1 * 6` is 0.6000000000000001); the factor
    // above is the exact one.
    let chosen = (chosen * 100.0).round() / 100.0;
    (scale, Some(ServingsScale { base, chosen }))
}

/// Build a [`RecipeTemplate`] or [`MenuTemplate`] for the given recipe path.
pub fn build_recipe_template(input: RecipeBuildInput<'_>) -> Result<RecipeBuildOutput> {
    let RecipeBuildInput {
        base_path,
        url_prefix,
        recipe_path,
        aisle_path,
        scale,
        servings,
        lang,
        static_mode,
        repo_url,
        features,
        viewer,
    } = input;

    // On the server `recipe_path` comes straight from the `/recipe/{*path}` URL.
    if !crate::util::is_safe_relative_path(recipe_path) {
        anyhow::bail!("Invalid path: {recipe_path}");
    }

    let recipe_path_buf = Utf8PathBuf::from(recipe_path);
    tracing::info!(
        "Looking for recipe at path: {}, extension: {:?}",
        recipe_path,
        recipe_path_buf.extension()
    );

    let entry = cooklang_find::get_recipe(vec![base_path], recipe_path_buf.as_path())
        .map_err(|e| anyhow::anyhow!("Recipe not found: {recipe_path}: {e}"))?;

    let actual_path = entry.path();
    tracing::info!(
        "Recipe path: {}, actual_path: {:?}, is_menu: {}",
        recipe_path,
        actual_path,
        entry.is_menu()
    );

    if entry.is_menu() {
        let template = build_menu_template_inner(
            recipe_path.to_string(),
            scale,
            servings,
            entry,
            base_path,
            url_prefix,
            lang,
            static_mode,
            repo_url,
            features,
            viewer,
        )?;
        return Ok(RecipeBuildOutput::Menu(Box::new(template)));
    }

    let mut recipe = crate::util::parse_unscaled_recipe_from_entry(&entry)
        .map_err(|e| anyhow::anyhow!("Failed to parse recipe: {e}"))?;
    let (scale, servings) = resolve_servings(
        recipe.metadata.servings().and_then(|s| s.as_number()),
        servings,
        scale,
    );
    recipe.scale(scale, crate::util::PARSER.converter());

    // Load aisle config for cooking mode ingredient sorting
    let aisle_content = if let Some(path) = aisle_path {
        match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!("Failed to read aisle file from {:?}: {}", path, e);
                String::new()
            }
        }
    } else {
        String::new()
    };
    let aisle = cooklang::aisle::parse_lenient(&aisle_content)
        .into_output()
        .unwrap_or_default();

    let tags = entry.tags();

    // Get the image path if available
    let image_path = entry
        .title_image()
        .clone()
        .and_then(|img_path| get_image_path(base_path, url_prefix, img_path));

    let mut ingredients = Vec::new();
    let mut cookware = Vec::new();
    let mut sections = Vec::new();

    // Group ingredients by display name and merge quantities
    let mut grouped_ingredients: std::collections::HashMap<
        String,
        (
            cooklang::quantity::GroupedQuantity,
            Vec<&cooklang::model::Ingredient>,
        ),
    > = std::collections::HashMap::new();

    for entry in recipe.group_ingredients(crate::util::PARSER.converter()) {
        let ingredient = entry.ingredient;
        let display_name = ingredient.display_name().to_string();

        grouped_ingredients
            .entry(display_name)
            .and_modify(|(merged_qty, igrs)| {
                merged_qty.merge(&entry.quantity, crate::util::PARSER.converter());
                igrs.push(ingredient);
            })
            .or_insert_with(|| (entry.quantity.clone(), vec![ingredient]));
    }

    // Sort by name for consistent display
    let mut sorted_ingredients: Vec<_> = grouped_ingredients.into_iter().collect();
    sorted_ingredients.sort_by(|a, b| a.0.cmp(&b.0));

    for (display_name, (quantity, ingredient_list)) in sorted_ingredients {
        // Use the first ingredient's data for reference path and notes
        let first_ingredient = ingredient_list[0];
        let reference_path = first_ingredient.reference.as_ref().map(|r| {
            // For web URLs - always use forward slash
            if r.components.is_empty() {
                r.name.clone()
            } else {
                format!("{}/{}", r.components.join("/"), r.name)
            }
        });

        // Combine notes from all ingredients
        let combined_note = if ingredient_list.len() > 1 {
            let notes: Vec<_> = ingredient_list
                .iter()
                .filter_map(|i| i.note.as_ref())
                .collect();
            if notes.is_empty() {
                None
            } else {
                Some(
                    notes
                        .iter()
                        .map(|n| n.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }
        } else {
            first_ingredient.note.clone()
        };

        // Format the merged quantity - show all quantities comma-separated
        let (formatted_quantity, formatted_unit) = if quantity.is_empty() {
            (None, None)
        } else {
            let quantities: Vec<_> = crate::util::format::quantity::ordered_components(&quantity)
                .into_iter()
                .map(|q| {
                    let qty_str =
                        crate::util::format::number::format_quantity(q.value()).unwrap_or_default();
                    let unit_str = q.unit().as_ref().map(|u| u.to_string()).unwrap_or_default();
                    if unit_str.is_empty() {
                        qty_str
                    } else {
                        format!("{} {}", qty_str, unit_str)
                    }
                })
                .collect();
            (Some(quantities.join(", ")), None)
        };

        ingredients.push(IngredientData {
            name: display_name,
            quantity: formatted_quantity,
            unit: formatted_unit,
            note: combined_note,
            reference_path,
        });
    }

    for item in &recipe.group_cookware(crate::util::PARSER.converter()) {
        cookware.push(CookwareData {
            name: item.cookware.name.to_string(),
        });
    }

    let mut total_steps = 0;
    for (section_index, section) in recipe.sections.iter().enumerate() {
        let mut section_items = Vec::new();
        let mut section_ingredient_indices = std::collections::HashSet::new();
        let mut cooking_mode_ingredient_indices: Vec<usize> = Vec::new();
        let mut step_count = 0;

        for content in &section.content {
            use cooklang::Content;
            match content {
                Content::Step(step) => {
                    let mut step_items = Vec::new();
                    let mut step_ingredients = Vec::new();

                    for item in &step.items {
                        use crate::web::templates::{StepIngredient, StepItem};
                        use cooklang::Item;

                        match item {
                            Item::Text { value } => {
                                let parts: Vec<&str> = value.split('\n').collect();
                                for (i, part) in parts.iter().enumerate() {
                                    if i > 0 {
                                        step_items.push(StepItem::LineBreak);
                                    }
                                    if !part.is_empty() {
                                        step_items.push(StepItem::Text(part.to_string()));
                                    }
                                }
                            }
                            Item::Ingredient { index } => {
                                section_ingredient_indices.insert(*index);
                                cooking_mode_ingredient_indices.push(*index);
                                if let Some(ing) = recipe.ingredients.get(*index) {
                                    let reference_path = ing.reference.as_ref().map(|r| {
                                        // For web URLs - always use forward slash
                                        if r.components.is_empty() {
                                            r.name.clone()
                                        } else {
                                            format!("{}/{}", r.components.join("/"), r.name)
                                        }
                                    });

                                    step_items.push(StepItem::Ingredient {
                                        name: ing.name.to_string(),
                                        reference_path,
                                    });

                                    // Also add to step ingredients list
                                    step_ingredients.push(StepIngredient {
                                        name: ing.name.to_string(),
                                        quantity: ing.quantity.as_ref().and_then(|q| {
                                            crate::util::format::number::format_quantity(q.value())
                                        }),
                                        unit: ing
                                            .quantity
                                            .as_ref()
                                            .and_then(|q| q.unit().as_ref().map(|u| u.to_string())),
                                        note: ing.note.clone(),
                                    });
                                }
                            }
                            Item::Cookware { index } => {
                                if let Some(cw) = recipe.cookware.get(*index) {
                                    step_items.push(StepItem::Cookware(cw.name.to_string()));
                                }
                            }
                            Item::Timer { index } => {
                                if let Some(timer) = recipe.timers.get(*index) {
                                    let mut timer_text = String::new();
                                    let mut seconds = None;

                                    // Add timer quantity and unit
                                    if let Some(quantity) = &timer.quantity {
                                        if let Some(formatted) =
                                            crate::util::format::number::format_quantity(
                                                quantity.value(),
                                            )
                                        {
                                            timer_text.push_str(&formatted);
                                        }
                                        if let Some(unit) = quantity.unit() {
                                            if !timer_text.is_empty() {
                                                timer_text.push(' ');
                                            }
                                            timer_text.push_str(unit);
                                        }
                                        seconds = timer_duration_seconds(quantity);
                                    }

                                    // If no duration info, just show "timer"
                                    if timer_text.is_empty() {
                                        timer_text = "timer".to_string();
                                    }

                                    step_items.push(StepItem::Timer {
                                        display: timer_text,
                                        seconds,
                                    });
                                }
                            }
                            Item::InlineQuantity { index } => {
                                if let Some(q) = recipe.inline_quantities.get(*index) {
                                    let mut qty =
                                        crate::util::format::number::format_quantity(q.value())
                                            .unwrap_or_default();
                                    if let Some(unit) = q.unit() {
                                        if !qty.is_empty() {
                                            qty.push_str(&format!(" {unit}"));
                                        } else {
                                            qty = unit.to_string();
                                        }
                                    }
                                    step_items.push(StepItem::Quantity(qty));
                                }
                            }
                        }
                    }

                    let section_image_path =
                        step_image(&entry, section_index + 1, step_count + 1, total_steps)
                            .and_then(|img_path| {
                                get_image_path(base_path, url_prefix, img_path.to_string())
                            });

                    section_items.push(RecipeSectionItem::Step(StepData {
                        number: step_count + 1,
                        items: step_items,
                        ingredients: step_ingredients,
                        image_path: section_image_path,
                    }));
                    step_count += 1;
                }
                Content::Text(text) => {
                    // Skip list bullet items
                    if text.trim() != "-" {
                        section_items.push(RecipeSectionItem::Note(text.trim().to_string()));
                    }
                }
            }
        }

        // Only add sections that have items (steps or notes)
        if !section_items.is_empty() {
            use crate::web::templates::RecipeSection;

            // Collect and group ingredients used in this section
            let mut section_grouped_ingredients: std::collections::HashMap<
                String,
                (
                    cooklang::quantity::GroupedQuantity,
                    Vec<&cooklang::model::Ingredient>,
                ),
            > = std::collections::HashMap::new();

            for idx in section_ingredient_indices {
                if let Some(ingredient) = recipe.ingredients.get(idx) {
                    let display_name = ingredient.display_name().to_string();
                    let qty = if let Some(q) = &ingredient.quantity {
                        let mut grouped_qty = cooklang::quantity::GroupedQuantity::empty();
                        grouped_qty.add(q, crate::util::PARSER.converter());
                        grouped_qty
                    } else {
                        cooklang::quantity::GroupedQuantity::empty()
                    };

                    section_grouped_ingredients
                        .entry(display_name)
                        .and_modify(|(merged_qty, igrs)| {
                            if let Some(q) = &ingredient.quantity {
                                merged_qty.add(q, crate::util::PARSER.converter());
                            }
                            igrs.push(ingredient);
                        })
                        .or_insert_with(|| (qty, vec![ingredient]));
                }
            }

            // Sort section ingredients by name
            let mut sorted_section_ingredients: Vec<_> =
                section_grouped_ingredients.into_iter().collect();
            sorted_section_ingredients.sort_by(|a, b| a.0.cmp(&b.0));

            let mut section_ingredients = Vec::new();
            for (display_name, (quantity, ingredient_list)) in sorted_section_ingredients {
                let first_ingredient = ingredient_list[0];
                let reference_path = first_ingredient.reference.as_ref().map(|r| {
                    // For web URLs - always use forward slash
                    if r.components.is_empty() {
                        r.name.clone()
                    } else {
                        format!("{}/{}", r.components.join("/"), r.name)
                    }
                });

                // Combine notes from all ingredients in the section
                let combined_note = if ingredient_list.len() > 1 {
                    let notes: Vec<_> = ingredient_list
                        .iter()
                        .filter_map(|i| i.note.as_ref())
                        .collect();
                    if notes.is_empty() {
                        None
                    } else {
                        Some(
                            notes
                                .iter()
                                .map(|n| n.as_str())
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                    }
                } else {
                    first_ingredient.note.clone()
                };

                // Format the merged quantity
                let (formatted_quantity, formatted_unit) = if quantity.is_empty() {
                    (None, None)
                } else {
                    let quantities: Vec<_> =
                        crate::util::format::quantity::ordered_components(&quantity)
                            .into_iter()
                            .map(|q| {
                                let qty_str =
                                    crate::util::format::number::format_quantity(q.value())
                                        .unwrap_or_default();
                                let unit_str =
                                    q.unit().as_ref().map(|u| u.to_string()).unwrap_or_default();
                                if unit_str.is_empty() {
                                    qty_str
                                } else {
                                    format!("{} {}", qty_str, unit_str)
                                }
                            })
                            .collect();
                    (Some(quantities.join(", ")), None)
                };

                section_ingredients.push(IngredientData {
                    name: display_name,
                    quantity: formatted_quantity,
                    unit: formatted_unit,
                    note: combined_note,
                    reference_path,
                });
            }

            // Build uncombined ingredients for cooking mode, sorted by aisle order
            let mut cooking_mode_ingredients_with_key: Vec<(
                Option<(usize, usize)>,
                IngredientData,
            )> = Vec::new();
            for idx in &cooking_mode_ingredient_indices {
                if let Some(ingredient) = recipe.ingredients.get(*idx) {
                    if !ingredient.modifiers().should_be_listed() {
                        continue;
                    }
                    let sort_key = aisle.ingredient_sort_key(&ingredient.name);

                    let (formatted_quantity, formatted_unit) = if let Some(q) = &ingredient.quantity
                    {
                        let qty_str = crate::util::format::number::format_quantity(q.value());
                        let unit_str = q.unit().as_ref().map(|u| u.to_string());
                        (qty_str, unit_str)
                    } else {
                        (None, None)
                    };

                    cooking_mode_ingredients_with_key.push((
                        sort_key,
                        IngredientData {
                            name: ingredient.name.to_string(),
                            quantity: formatted_quantity,
                            unit: formatted_unit,
                            note: ingredient.note.clone(),
                            reference_path: None,
                        },
                    ));
                }
            }

            // Sort: aisle items first (by category_index, ingredient_index), then uncategorized at end
            cooking_mode_ingredients_with_key.sort_by(|a, b| match (&a.0, &b.0) {
                (Some(ka), Some(kb)) => ka.cmp(kb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.1.name.cmp(&b.1.name),
            });

            let cooking_mode_ingredients: Vec<IngredientData> = cooking_mode_ingredients_with_key
                .into_iter()
                .map(|(_, data)| data)
                .collect();

            sections.push(RecipeSection {
                number: section_index + 1,
                name: section.name.clone(),
                items: section_items.clone(),
                step_offset: total_steps,
                ingredients: section_ingredients,
                cooking_mode_ingredients,
            });
            total_steps += step_count;
        }
    }

    let breadcrumbs: Vec<String> = recipe_path
        .split('/')
        .map(|s| s.trim_end_matches(".cook").to_string())
        .collect();

    let metadata = if recipe.metadata.map.is_empty() {
        None
    } else {
        // Get standard metadata fields (handle both string and number types)
        let get_field = |key: &str| -> Option<String> {
            recipe.metadata.get(key).and_then(|v| {
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else if let Some(n) = v.as_i64() {
                    Some(n.to_string())
                } else {
                    v.as_f64().map(crate::util::format::number::format_number)
                }
            })
        };

        let mut custom_metadata = Vec::new();
        for (key, value) in recipe.metadata.map_filtered() {
            if let (Some(key_str), Some(val_str)) = (key.as_str(), value.as_str()) {
                if key_str.starts_with("source.") || key_str.starts_with("time.") {
                    continue;
                }

                custom_metadata.push((key_str.to_string(), val_str.to_string()));
            }
        }

        Some(RecipeMetadata {
            // The stepper's value rather than the metadata: `scale` rounds
            // `servings` to a whole number, so half of a 4-serving recipe
            // would read 1.
            servings: servings
                .as_ref()
                .map(|s| s.chosen.to_string())
                .or_else(|| get_field("servings")),
            time: get_field("time"),
            difficulty: get_field("difficulty"),
            course: get_field("course"),
            prep_time: get_field("prep time")
                .or_else(|| get_field("prep_time"))
                .or_else(|| get_field("preptime"))
                .or_else(|| get_field("time.prep")),
            cook_time: get_field("cook time")
                .or_else(|| get_field("cook_time"))
                .or_else(|| get_field("cooktime"))
                .or_else(|| get_field("time.cook")),
            cuisine: get_field("cuisine"),
            diet: get_field("diet"),
            author: get_field("author").or_else(|| get_field("source.author")),
            description: get_field("description"),
            source: get_field("source").or_else(|| get_field("source.name")),
            source_url: get_field("source.url"),
            custom: custom_metadata,
        })
    };

    // Use title from metadata if available, otherwise use filename
    let recipe_name = recipe
        .metadata
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            recipe_path
                .split('/')
                .next_back()
                .unwrap_or(recipe_path)
                .replace(".cook", "")
        });

    let template = RecipeTemplate {
        active: "recipes".to_string(),
        recipe: RecipeData {
            name: recipe_name,
            metadata,
        },
        recipe_path: recipe_path.to_string(),
        breadcrumbs,
        scale,
        servings,
        tags,
        ingredients,
        cookware,
        sections,
        image_path,
        tr: Tr::new(lang),
        prefix: url_prefix.to_string(),
        static_mode,
        repo_url,
        features,
        viewer,
    };

    Ok(RecipeBuildOutput::Recipe(Box::new(template)))
}

#[allow(clippy::too_many_arguments)]
fn build_menu_template_inner(
    path: String,
    scale: f64,
    servings: Option<f64>,
    entry: cooklang_find::RecipeEntry,
    base_path: &Utf8Path,
    url_prefix: &str,
    lang: LanguageIdentifier,
    static_mode: bool,
    repo_url: Option<String>,
    features: FeatureFlags,
    viewer: Viewer,
) -> Result<MenuTemplate> {
    let menu_path = Utf8Path::new(&path);
    // The menu's own `servings` decide the scale when the stepper counts
    // servings; the references' factors are then resolved at that scale.
    let menu_servings = |menu: &cooklang_find::Menu| {
        menu.metadata.get("servings").and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
                .and_then(|n| u32::try_from(n).ok())
        })
    };
    let mut menu = crate::util::menu::parse_menu(&entry, menu_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse menu: {e}"))?;
    let (scale, servings) = resolve_servings(menu_servings(&menu), servings, scale);
    menu.resolve_scales(&[base_path], scale);

    // Referenced recipes are read from disk for their default `servings`.
    // Menus repeat references, so memoise per build.
    let mut ref_info_cache: std::collections::HashMap<String, RecipeInfo> =
        std::collections::HashMap::new();

    // Get the image path if available
    let image_path = entry
        .title_image()
        .clone()
        .and_then(|img_path| get_image_path(base_path, url_prefix, img_path));

    let breadcrumbs: Vec<String> = path.split('/').map(|s| s.to_string()).collect();

    // Parse sections and content
    let mut sections = Vec::new();

    for section in &menu.sections {
        let mut lines: Vec<Vec<MenuSectionItem>> = Vec::new();

        for meal in &section.meals {
            // The page shows a meal header as the line it was written as.
            if let Some(meal_type) = &meal.meal_type {
                let header = match &meal.time {
                    Some(time) => format!("{meal_type} ({time}):"),
                    None => format!("{meal_type}:"),
                };
                lines.push(vec![MenuSectionItem::Text(header)]);
            }

            let mut line = Vec::new();
            for item in &meal.items {
                match item {
                    MenuItem::RecipeReference {
                        path,
                        scale: factor,
                        ..
                    } => {
                        let factor = factor.unwrap_or(1.0);
                        let info = ref_info_cache
                            .entry(path.clone())
                            .or_insert_with(|| ref_info_or_default(base_path, path, path));

                        // Display-only: a x1 badge on every unscaled reference
                        // would be pure noise, so suppress it. The number
                        // shown, when shown, is the same one the API reports.
                        // A recipe with servings is linked by its servings, as
                        // its page counts them.
                        let scaled = factor != 1.0;
                        line.push(MenuSectionItem::RecipeReference {
                            name: path.clone(),
                            scale: scaled.then_some(factor),
                            servings: info
                                .default_servings
                                .filter(|&n| n > 0 && scaled)
                                .map(|n| f64::from(n) * factor),
                        });
                    }
                    MenuItem::Ingredient {
                        name,
                        quantity,
                        unit,
                    } => {
                        let (quantity, unit) = crate::util::menu::scaled_quantity(
                            quantity.as_deref(),
                            unit.as_deref(),
                            scale,
                        );
                        line.push(MenuSectionItem::Ingredient {
                            name: name.clone(),
                            quantity,
                            unit,
                        });
                    }
                    MenuItem::Text { text } => line.push(MenuSectionItem::Text(text.clone())),
                    MenuItem::LineBreak => lines.push(std::mem::take(&mut line)),
                    // Notes are for the author, not the page.
                    _ => {}
                }
            }
            if !line.is_empty() {
                lines.push(line);
            }
        }

        // Filter out empty lines (lines that are only whitespace text)
        lines.retain(|line| {
            !line
                .iter()
                .all(|item| matches!(item, MenuSectionItem::Text(t) if t.trim().is_empty()))
        });

        if !lines.is_empty() {
            sections.push(MenuSection {
                name: section.name.clone(),
                lines,
            });
        }
    }

    // Get metadata
    let metadata = if menu.metadata == cooklang_find::Metadata::default() {
        None
    } else {
        let get_field = |key: &str| -> Option<String> {
            menu.metadata.get(key).and_then(|v| {
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else if let Some(n) = v.as_i64() {
                    Some(n.to_string())
                } else {
                    v.as_f64().map(crate::util::format::number::format_number)
                }
            })
        };

        // `Metadata` keeps its entries in a `HashMap` and offers no iterator,
        // so custom fields come from its serialised form, sorted by key to
        // keep the page stable from one load to the next.
        let raw = serde_json::to_value(&menu.metadata).unwrap_or_default();
        let mut custom_metadata: Vec<(String, String)> = raw
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(key, _)| key.parse::<cooklang::metadata::StdKey>().is_err())
            .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_string())))
            .collect();
        custom_metadata.sort();

        Some(RecipeMetadata {
            // As on the recipe page: `scale` rounds `servings` to a whole number.
            servings: servings
                .as_ref()
                .map(|s| s.chosen.to_string())
                .or_else(|| get_field("servings")),
            time: get_field("time"),
            difficulty: get_field("difficulty"),
            course: get_field("course"),
            prep_time: get_field("prep time")
                .or_else(|| get_field("prep_time"))
                .or_else(|| get_field("preptime")),
            cook_time: get_field("cook time")
                .or_else(|| get_field("cook_time"))
                .or_else(|| get_field("cooktime")),
            cuisine: get_field("cuisine"),
            diet: get_field("diet"),
            author: get_field("author").or_else(|| get_field("source.author")),
            description: get_field("description"),
            source: get_field("source").or_else(|| get_field("source.name")),
            source_url: get_field("source.url"),
            custom: custom_metadata,
        })
    };

    let menu_name = menu.name.clone();

    Ok(MenuTemplate {
        active: "recipes".to_string(),
        name: menu_name,
        recipe_path: path,
        breadcrumbs,
        scale,
        servings,
        metadata,
        sections,
        image_path,
        tr: Tr::new(lang),
        prefix: url_prefix.to_string(),
        static_mode,
        repo_url,
        features,
        viewer,
    })
}

/// Paths of every `.cook` recipe in `tree` and below it, menus left out.
pub fn cook_recipe_paths(tree: &cooklang_find::RecipeTree) -> Vec<&Utf8Path> {
    let mut paths = Vec::new();
    let mut stack = vec![tree];
    while let Some(node) = stack.pop() {
        if let Some(path) = node
            .recipe
            .as_ref()
            .filter(|recipe| !recipe.is_menu())
            .and_then(|recipe| recipe.path())
        {
            paths.push(path.as_path());
        }
        stack.extend(node.children.values());
    }
    paths
}

fn count_recipes_tree(tree: &cooklang_find::RecipeTree) -> Option<usize> {
    let mut count = 0;

    for child in tree.children.values() {
        if !child.children.is_empty() {
            count += count_recipes_tree(child).unwrap_or(0);
        } else {
            count += 1;
        }
    }

    Some(count)
}

/// The URL a recipe's title or step picture is served at.
///
/// An `http(s)` address from the metadata is passed through. A file becomes
/// `{prefix}/api/static/` followed by its path from the recipe directory,
/// **each segment percent-encoded**. The path is built from folder and file
/// names someone else may have chosen — a shared or synced collection, a
/// cloned repository — so it can hold anything a name can: `"` and `'`, which
/// end an attribute; `#` and `?`, which cut the URL short; `%`, which the
/// server would decode. Encoded, it is inert wherever it lands, and those
/// pictures load at all (#548).
pub(crate) fn get_image_path(
    base_path: &Utf8Path,
    prefix: &str,
    img_path: String,
) -> Option<String> {
    tracing::debug!("Recipe image path from entry: {}", img_path);
    // If it's a URL, use it directly
    if img_path.starts_with("http://") || img_path.starts_with("https://") {
        Some(img_path)
    } else {
        // For file paths, we need to make them relative to the base path and accessible via /api/static
        let img_path = camino::Utf8Path::new(&img_path);

        // Try to strip the base_path prefix to get a relative path
        if let Ok(relative) = img_path.strip_prefix(base_path) {
            let result = static_url(prefix, relative);
            tracing::debug!("Image path relative to base: {}", result);
            Some(result)
        } else if !img_path.is_absolute() {
            Some(static_url(prefix, img_path))
        } else {
            img_path
                .file_name()
                .map(|name| static_url(prefix, Utf8Path::new(name)))
        }
    }
}

/// The picture file shown for step `step` of section `section`, both
/// one-based, with `steps_before` steps in the sections ahead of it.
///
/// Step pictures come in two naming conventions (#374): `Recipe.S.N.ext`
/// (section S, step N within it — what the iOS app writes) and `Recipe.G.ext`
/// (step G counted across every section). The section-specific one wins.
/// `section` counts every section of the parsed recipe, empty ones included.
pub(crate) fn step_image(
    entry: &cooklang_find::RecipeEntry,
    section: usize,
    step: usize,
    steps_before: usize,
) -> Option<&String> {
    let images = entry.step_images();
    images
        .get(section, step)
        .or_else(|| images.get(0, steps_before + step))
}

/// `{prefix}/api/static/` and `relative`, one percent-encoded segment per
/// path component. Joined with `/` whatever the platform's separator is, so a
/// Windows path makes a working URL too.
fn static_url(prefix: &str, relative: &Utf8Path) -> String {
    let segments: Vec<_> = relative
        .components()
        .map(|component| urlencoding::encode(component.as_str()))
        .collect();
    format!("{prefix}/api/static/{}", segments.join("/"))
}

#[cfg(test)]
mod image_path_tests {
    use super::get_image_path;
    use camino::Utf8Path;

    fn url(base: &str, prefix: &str, image: &str) -> Option<String> {
        get_image_path(Utf8Path::new(base), prefix, image.to_string())
    }

    /// The folder name from #548: unencoded, its `"` ended the `src`
    /// attribute cooking mode wrote and the rest became an event handler.
    #[test]
    fn a_quote_in_a_folder_name_cannot_leave_the_url() {
        let image = "/recipes/x\" onerror=\"alert(1)\" y=\"/Pancakes.1.jpg";
        let url = url("/recipes", "", image).unwrap();
        assert_eq!(
            url,
            "/api/static/x%22%20onerror%3D%22alert%281%29%22%20y%3D%22/Pancakes.1.jpg"
        );
        assert!(!url.contains(['"', '\'', '<', '>', ' ']), "{url}");
    }

    #[test]
    fn every_segment_is_encoded_and_slashes_are_kept() {
        assert_eq!(
            url("/recipes", "", "/recipes/Breakfast/Easy Pancakes.jpg").as_deref(),
            Some("/api/static/Breakfast/Easy%20Pancakes.jpg")
        );
        // `#`, `?` and `%` used to cut the URL short or be decoded by the
        // server, so these pictures never loaded.
        assert_eq!(
            url("/recipes", "/cook", "/recipes/50% #1?/Tart's.png").as_deref(),
            Some("/cook/api/static/50%25%20%231%3F/Tart%27s.png")
        );
    }

    #[test]
    fn relative_paths_are_encoded_too() {
        assert_eq!(
            url("/recipes", "", "Sub Dir/a b.jpg").as_deref(),
            Some("/api/static/Sub%20Dir/a%20b.jpg")
        );
    }

    /// Unix only: without a drive letter the path is not absolute on Windows.
    #[cfg(unix)]
    #[test]
    fn a_picture_outside_the_directory_is_encoded_by_name() {
        assert_eq!(
            url("/recipes", "", "/elsewhere/a \"b\".jpg").as_deref(),
            Some("/api/static/a%20%22b%22.jpg")
        );
    }

    #[test]
    fn web_addresses_are_passed_through() {
        let address = "https://example.com/a b.jpg?x=1#y";
        assert_eq!(url("/recipes", "", address).as_deref(), Some(address));
    }
}

#[cfg(test)]
mod natural_sort_tests {
    use super::{natural_cmp, natural_key};
    use std::cmp::Ordering;

    #[test]
    fn non_ascii_letters_keep_code_point_order() {
        // Accents are not folded: this pins the documented divergence from
        // the browser collator so a change here is deliberate.
        assert_eq!(natural_cmp("zucchini", "Äpfel"), Ordering::Less);
        // Case still folds for non-ASCII letters.
        assert_eq!(natural_key("äpfel"), natural_key("Äpfel"));
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(natural_cmp("apple pie", "Banana bread"), Ordering::Less);
        assert_eq!(natural_cmp("Banana bread", "apple pie"), Ordering::Greater);
    }

    #[test]
    fn digit_runs_compare_numerically() {
        assert_eq!(natural_cmp("Recipe 9", "Recipe 10"), Ordering::Less);
        assert_eq!(natural_cmp("Recipe 10", "Recipe 9"), Ordering::Greater);
        assert_eq!(natural_cmp("Recipe 10", "Recipe 10"), Ordering::Equal);
    }

    #[test]
    fn case_only_differences_are_stable_ties() {
        // Equal under the natural key; byte order breaks the tie so the
        // result is deterministic across runs.
        assert_eq!(natural_cmp("a", "A"), Ordering::Greater);
        assert_eq!(natural_cmp("A", "a"), Ordering::Less);
        assert_eq!(natural_cmp("a", "a"), Ordering::Equal);
    }

    #[test]
    fn sorts_like_the_client_collator() {
        let mut names = vec![
            "Recipe 10",
            "banana bread",
            "Recipe 9",
            "Apple pie",
            "recipe 2",
        ];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            names,
            vec![
                "Apple pie",
                "banana bread",
                "recipe 2",
                "Recipe 9",
                "Recipe 10"
            ]
        );
    }

    #[test]
    fn directories_come_before_recipes() {
        struct Item {
            name: &'static str,
            is_directory: bool,
        }
        let mut items = [
            Item {
                name: "zucchini",
                is_directory: false,
            },
            Item {
                name: "Soups",
                is_directory: true,
            },
            Item {
                name: "Apple pie",
                is_directory: false,
            },
            Item {
                name: "breakfast",
                is_directory: true,
            },
        ];
        items.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => natural_cmp(a.name, b.name),
        });
        let order: Vec<_> = items.iter().map(|i| i.name).collect();
        assert_eq!(order, vec!["breakfast", "Soups", "Apple pie", "zucchini"]);
    }
}

#[cfg(test)]
mod servings_tests {
    use super::*;
    use askama::Template;

    const SERVES_FOUR: &str = "---\nservings: 4\n---\n\nAdd @flour{200%g}.\n";
    const NO_SERVINGS: &str = "Add @flour{200%g}.\n";

    /// The recipe page for `recipe`, as `/recipe/Soup?scale=..&servings=..`.
    fn page(recipe: &str, scale: f64, servings: Option<f64>) -> Box<RecipeTemplate> {
        let dir = tempfile::TempDir::new().unwrap();
        let base = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(base.join("Soup.cook"), recipe).unwrap();
        let output = build_recipe_template(RecipeBuildInput {
            base_path: base,
            url_prefix: "",
            recipe_path: "Soup",
            aisle_path: None,
            scale,
            servings,
            lang: "en-US".parse().unwrap(),
            static_mode: false,
            repo_url: None,
            features: FeatureFlags::default(),
            viewer: Viewer::default(),
        })
        .unwrap();
        match output {
            RecipeBuildOutput::Recipe(template) => template,
            RecipeBuildOutput::Menu(_) => panic!("Soup.cook is not a menu"),
        }
    }

    fn stepper(template: &RecipeTemplate) -> Option<(u32, f64)> {
        template.servings.as_ref().map(|s| (s.base, s.chosen))
    }

    #[test]
    fn a_recipe_with_servings_starts_at_its_own_servings() {
        let template = page(SERVES_FOUR, 1.0, None);
        assert_eq!(stepper(&template), Some((4, 4.0)));
        assert_eq!(template.scale, 1.0);

        let html = template.render().unwrap();
        assert!(html.contains(r#"id="servings""#), "{html}");
        assert!(html.contains(r#"value="4""#));
        assert!(html.contains(r#"data-base-servings="4""#));
        assert!(!html.contains(r#"id="scale""#));
        assert!(html.contains("👥 4 servings"));
    }

    #[test]
    fn servings_set_the_factor() {
        let template = page(SERVES_FOUR, 1.0, Some(6.0));
        assert_eq!(template.scale, 1.5);
        assert_eq!(stepper(&template), Some((4, 6.0)));

        let html = template.render().unwrap();
        assert!(html.contains("300"), "200 g x 1.5: {html}");
        assert!(html.contains("👥 6 servings"));
        assert!(html.contains("6 servings</div>"), "print header");
    }

    #[test]
    fn half_a_serving_and_no_ceiling() {
        let half = page(SERVES_FOUR, 1.0, Some(0.5));
        assert_eq!(half.scale, 0.125);
        // `scale` rounds the metadata to a whole serving; the pill must not.
        assert!(half.render().unwrap().contains("👥 0.5 servings"));

        assert_eq!(page(SERVES_FOUR, 1.0, Some(1000.0)).scale, 250.0);
    }

    /// Scaled away from its own servings, the page says what the recipe was
    /// written for, since times and pan sizes do not always scale, and links
    /// back to it.
    #[test]
    fn a_scaled_recipe_says_what_it_was_written_for() {
        let html = page(SERVES_FOUR, 1.0, None).render().unwrap();
        assert!(!html.contains("metadata-original-servings"), "unscaled");

        let html = page(SERVES_FOUR, 1.0, Some(6.0)).render().unwrap();
        assert!(html.contains("📖 Written for 4 servings"), "{html}");
        assert!(html.contains(r#"<a href="/recipe/Soup""#));

        let one = "---\nservings: 1\n---\n\nAdd @flour{200%g}.\n";
        let html = page(one, 2.0, None).render().unwrap();
        assert!(html.contains("📖 Written for 1 serving<"));

        let html = page(NO_SERVINGS, 2.0, None).render().unwrap();
        assert!(!html.contains("metadata-original-servings"), "no servings");
    }

    /// Without servings to count, a scaled recipe gives the same reminder as
    /// a factor of the original, whether or not it has any metadata.
    #[test]
    fn a_recipe_scaled_by_the_multiplier_says_so() {
        let html = page(NO_SERVINGS, 1.0, None).render().unwrap();
        assert!(!html.contains("metadata-original"), "unscaled");

        // No metadata at all: the badge gets a row of its own.
        let html = page(NO_SERVINGS, 1.5, None).render().unwrap();
        assert!(html.contains("📖 ×1.5 of the original recipe"), "{html}");
        assert!(html.contains(r#"<a href="/recipe/Soup""#));

        let titled = "---\ntitle: Soup\n---\n\nAdd @flour{200%g}.\n";
        let html = page(titled, 0.333333, None).render().unwrap();
        assert_eq!(html.matches("📖 ×0.333 of the original recipe").count(), 1);

        // A recipe with servings says what it was written for instead.
        let html = page(SERVES_FOUR, 1.5, None).render().unwrap();
        assert!(!html.contains("metadata-original-scale"));
        assert!(html.contains("metadata-original-servings"));
    }

    /// The half-a-serving floor is the stepper's; a menu can link below it.
    #[test]
    fn servings_below_the_stepper_floor_are_taken_as_is() {
        let template = page(SERVES_FOUR, 1.0, Some(0.4));
        assert_eq!(template.scale, 0.1);
        assert_eq!(stepper(&template), Some((4, 0.4)));
        assert!(template.render().unwrap().contains(r#"min="0.4""#));
    }

    #[test]
    fn servings_win_over_scale() {
        assert_eq!(page(SERVES_FOUR, 3.0, Some(2.0)).scale, 0.5);
    }

    /// Menu links and bookmarks carry `?scale=`: same factor, shown as servings.
    #[test]
    fn scale_links_keep_their_factor() {
        let template = page(SERVES_FOUR, 1.5, None);
        assert_eq!(template.scale, 1.5);
        assert_eq!(stepper(&template), Some((4, 6.0)));

        let odd = page("---\nservings: 6\n---\n\nAdd @flour{200%g}.\n", 0.1, None);
        assert_eq!(odd.scale, 0.1);
        assert_eq!(stepper(&odd), Some((6, 0.6)), "rounded for display");
    }

    #[test]
    fn unusable_servings_fall_back_to_scale() {
        for servings in [0.0, -2.0, f64::NAN, f64::INFINITY] {
            let template = page(SERVES_FOUR, 2.0, Some(servings));
            assert_eq!(template.scale, 2.0, "?servings={servings}");
            assert_eq!(stepper(&template), Some((4, 8.0)));
        }
    }

    #[test]
    fn a_recipe_without_servings_keeps_the_multiplier() {
        let template = page(NO_SERVINGS, 2.0, Some(6.0));
        assert_eq!(template.scale, 2.0, "?servings= is ignored");
        assert!(template.servings.is_none());

        let html = template.render().unwrap();
        assert!(html.contains(r#"id="scale""#));
        assert!(html.contains(r#"max="200""#));
        assert!(!html.contains(r#"id="servings""#));
    }

    #[test]
    fn servings_that_are_not_a_whole_number_keep_the_multiplier() {
        for servings in ["4-6", "1.5", "0"] {
            let recipe = format!("---\nservings: {servings}\n---\n\nAdd @flour{{200%g}}.\n");
            let template = page(&recipe, 2.0, Some(6.0));
            assert_eq!(template.scale, 2.0, "servings: {servings}");
            assert!(template.servings.is_none(), "servings: {servings}");
        }
    }

    /// `Week.menu` serves 2 and references `Soup` (serves 4) and `Toast` (no
    /// servings), rendered as `/recipe/Week.menu?scale=..&servings=..`.
    fn menu_html(menu_servings: &str, scale: f64, servings: Option<f64>) -> (MenuTemplate, String) {
        let dir = tempfile::TempDir::new().unwrap();
        let base = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(base.join("Soup.cook"), SERVES_FOUR).unwrap();
        std::fs::write(base.join("Toast.cook"), NO_SERVINGS).unwrap();
        std::fs::write(
            base.join("Week.menu"),
            format!(
                "---\n{menu_servings}---\n\n== Monday ==\n\nLunch: \\\n\
                 - @./Soup{{2%servings}} \\\n- @./Toast{{3}} \\\n- @./Soup{{}}\n"
            ),
        )
        .unwrap();
        let output = build_recipe_template(RecipeBuildInput {
            base_path: base,
            url_prefix: "",
            recipe_path: "Week.menu",
            aisle_path: None,
            scale,
            servings,
            lang: "en-US".parse().unwrap(),
            static_mode: false,
            repo_url: None,
            features: FeatureFlags::default(),
            viewer: Viewer::default(),
        })
        .unwrap();
        let RecipeBuildOutput::Menu(template) = output else {
            panic!("Week.menu is a menu");
        };
        let html = template.render().unwrap();
        (*template, html)
    }

    /// The query string of every link to `Soup` or `Toast` in `html`, in
    /// order. The page links to the menu itself too.
    fn links(html: &str) -> Vec<&str> {
        html.split(r#"href="/recipe/"#)
            .skip(1)
            .map(|rest| &rest[..rest.find('"').unwrap()])
            .filter(|href| href.starts_with("Soup") || href.starts_with("Toast"))
            .map(|href| href.split_once('?').map_or("", |(_, query)| query))
            .collect()
    }

    #[test]
    fn a_menu_with_servings_counts_servings() {
        let (template, html) = menu_html("servings: 2\n", 1.0, None);
        assert_eq!(
            template.servings.as_ref().map(|s| (s.base, s.chosen)),
            Some((2, 2.0))
        );
        assert!(html.contains(r#"id="servings""#));
        assert!(!html.contains(r#"id="scale""#));

        assert!(!html.contains("metadata-original-servings"));

        let (template, html) = menu_html("servings: 2\n", 1.0, Some(4.0));
        assert_eq!(template.scale, 2.0);
        assert!(html.contains("👥 4 servings"));
        assert!(html.contains("📖 Written for 2 servings"));
    }

    #[test]
    fn a_menu_without_servings_keeps_the_multiplier() {
        let (template, html) = menu_html("title: Week\n", 2.0, Some(4.0));
        assert_eq!(template.scale, 2.0, "?servings= is ignored");
        assert!(template.servings.is_none());
        assert!(html.contains(r#"id="scale""#));
        assert!(html.contains("📖 ×2 of the original recipe"));
    }

    /// A recipe that declares servings is linked by its servings, as its page
    /// counts them; any other by its factor. A x1 reference has a plain link.
    #[test]
    fn menu_links_carry_servings_when_the_recipe_has_them() {
        // Soup{2%servings} is x0.5 of 4 servings; Toast{3} x3; Soup{} x1.
        let (_, html) = menu_html("servings: 2\n", 1.0, None);
        assert_eq!(links(&html), vec!["servings=2", "scale=3", ""]);

        // The whole menu at 4 servings doubles every factor, which brings
        // Soup{2%servings} to x1, Soup's own 4 servings: a plain link.
        let (_, html) = menu_html("servings: 2\n", 1.0, Some(4.0));
        assert_eq!(links(&html), vec!["", "scale=6", "servings=8"]);

        // Below the stepper's half a serving: 0.1 of 4 servings.
        let (_, html) = menu_html("title: Week\n", 0.1, None);
        assert_eq!(
            links(&html),
            vec!["servings=0.2", "scale=0.3", "servings=0.4"]
        );
    }
}
