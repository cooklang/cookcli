use crate::server::{templates::*, AppState};
use axum::{
    extract::{Extension, Host, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Form, Router,
};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use fluent_templates::Loader;
use serde::Deserialize;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

pub fn ui() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(recipes_page))
        .route("/directory/*path", get(recipes_directory))
        .route("/recipe/*path", get(recipe_page))
        .route("/edit/*path", get(edit_page))
        .route("/new", get(new_page).post(create_recipe))
        .route("/shopping-list", get(shopping_list_page))
        .route("/pantry", get(pantry_page))
        .route("/preferences", get(preferences_page))
}

async fn recipes_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    recipes_handler(state, None, lang).await
}

async fn recipes_directory(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    recipes_handler(state, Some(path), lang).await
}

async fn recipes_handler(
    state: Arc<AppState>,
    path: Option<String>,
    lang: LanguageIdentifier,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    let base = &state.base_path;
    let search_path = if let Some(p) = &path {
        base.join(p)
    } else {
        base.clone()
    };

    let tree = cooklang_find::build_tree(&search_path).map_err(|e| {
        tracing::error!("Failed to build recipe tree: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut items = Vec::new();

    for (name, child) in &tree.children {
        let is_dir = !child.children.is_empty();
        let item_path = {
            let url_path = child
                .recipe
                .as_ref()
                .and_then(|recipe| recipe.file_name())
                .unwrap_or_else(|| name.to_string());

            match &path {
                Some(p) => format!("{p}/{url_path}"),
                None => url_path.to_string(),
            }
        };

        // Extract tags, image, and is_menu if this is a recipe
        let (tags, image_path, is_menu) = if let Some(ref recipe) = child.recipe {
            // Get image path similar to how we do it in recipe_page
            let img_path = recipe.title_image().clone().and_then(|img| {
                if img.starts_with("http://") || img.starts_with("https://") {
                    Some(img)
                } else {
                    // Make path relative to base and accessible via /api/static
                    let img_path = camino::Utf8Path::new(&img);
                    if let Ok(relative) = img_path.strip_prefix(base) {
                        Some(format!("/api/static/{relative}"))
                    } else if !img_path.is_absolute() {
                        Some(format!("/api/static/{img_path}"))
                    } else {
                        img_path
                            .file_name()
                            .map(|name| format!("/api/static/{name}"))
                    }
                }
            });
            (recipe.tags(), img_path, recipe.is_menu())
        } else {
            (Vec::new(), None, false)
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
        });
    }

    items.sort_by(|a, b| match (a.is_directory, b.is_directory) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    let breadcrumbs = if let Some(p) = &path {
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

    let current_name = if let Some(ref p) = path {
        p.split('/').next_back().unwrap_or("Recipes").to_string()
    } else {
        crate::server::i18n::LOCALES.lookup(&lang, "recipes-title")
    };

    let template = RecipesTemplate {
        active: "recipes".to_string(),
        current_name,
        breadcrumbs,
        items,
        tr: Tr::new(lang),
    };

    Ok(template)
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

#[derive(Deserialize)]
struct RecipeQuery {
    scale: Option<f64>,
}

async fn recipe_page(
    Path(path): Path<String>,
    Query(query): Query<RecipeQuery>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> Result<axum::response::Response, StatusCode> {
    let scale = query.scale.unwrap_or(1.0);

    let recipe_path = Utf8PathBuf::from(&path);
    tracing::info!(
        "Looking for recipe at path: {}, extension: {:?}",
        path,
        recipe_path.extension()
    );

    let entry = cooklang_find::get_recipe(vec![&state.base_path], &recipe_path).map_err(|_| {
        tracing::error!("Recipe not found: {path}");
        StatusCode::NOT_FOUND
    })?;

    // Check if this is a menu file
    let actual_path = entry.path();
    tracing::info!(
        "Recipe path: {}, actual_path: {:?}, is_menu: {}",
        path,
        actual_path,
        entry.is_menu()
    );
    if entry.is_menu() {
        let template = menu_page_handler(path, scale, entry, state, lang).await?;
        return Ok(template.into_response());
    }

    let recipe = crate::util::parse_recipe_from_entry(&entry, scale).map_err(|e| {
        tracing::error!("Failed to parse recipe: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let tags = entry.tags();

    // Get the image path if available
    let image_path = entry
        .title_image()
        .clone()
        .and_then(|img_path| get_image_path(&state.base_path, img_path));

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
            let quantities: Vec<_> = quantity
                .iter()
                .map(|q| {
                    let qty_str =
                        crate::util::format::format_quantity(q.value()).unwrap_or_default();
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
    for section in &recipe.sections {
        let mut section_items = Vec::new();
        let mut section_ingredient_indices = std::collections::HashSet::new();
        let mut step_count = 0;

        for content in &section.content {
            use cooklang::Content;
            match content {
                Content::Step(step) => {
                    let mut step_items = Vec::new();
                    let mut step_ingredients = Vec::new();

                    for item in &step.items {
                        use crate::server::templates::{StepIngredient, StepItem};
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
                                            crate::util::format::format_quantity(q.value())
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

                                    // Add timer quantity and unit
                                    if let Some(quantity) = &timer.quantity {
                                        if let Some(formatted) =
                                            crate::util::format::format_quantity(quantity.value())
                                        {
                                            timer_text.push_str(&formatted);
                                        }
                                        if let Some(unit) = quantity.unit() {
                                            if !timer_text.is_empty() {
                                                timer_text.push(' ');
                                            }
                                            timer_text.push_str(unit);
                                        }
                                    }

                                    // If no duration info, just show "timer"
                                    if timer_text.is_empty() {
                                        timer_text = "timer".to_string();
                                    }

                                    step_items.push(StepItem::Timer(timer_text));
                                }
                            }
                            Item::InlineQuantity { index } => {
                                if let Some(q) = recipe.inline_quantities.get(*index) {
                                    let mut qty = crate::util::format::format_quantity(q.value())
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

                    let section_image_path = entry
                        .step_images()
                        .get(0, total_steps + step_count + 1)
                        .and_then(|img_path| {
                            get_image_path(&state.base_path, img_path.to_string())
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
            use crate::server::templates::RecipeSection;

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
                    let quantities: Vec<_> = quantity
                        .iter()
                        .map(|q| {
                            let qty_str =
                                crate::util::format::format_quantity(q.value()).unwrap_or_default();
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

            sections.push(RecipeSection {
                name: section.name.clone(),
                items: section_items.clone(),
                step_offset: total_steps,
                ingredients: section_ingredients,
            });
            total_steps += step_count;
        }
    }

    let breadcrumbs: Vec<String> = path
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
                    v.as_f64().map(crate::util::format::format_number)
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
            servings: get_field("servings"),
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
            path.split('/')
                .next_back()
                .unwrap_or(&path)
                .replace(".cook", "")
        });

    let template = RecipeTemplate {
        active: "recipes".to_string(),
        recipe: RecipeData {
            name: recipe_name,
            metadata,
        },
        recipe_path: path,
        breadcrumbs,
        scale,
        tags,
        ingredients,
        cookware,
        sections,
        image_path,
        tr: Tr::new(lang),
    };

    Ok(template.into_response())
}

async fn edit_page(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    tracing::info!("Edit page requested for path: {}", path);

    // Validate path to prevent directory traversal
    let path_check = Utf8Path::new(&path);
    if !path_check
        .components()
        .all(|c| matches!(c, Utf8Component::Normal(_)))
    {
        tracing::error!("Invalid path: {path}");
        return Err(StatusCode::BAD_REQUEST);
    }

    let recipe_path = Utf8PathBuf::from(&path);

    // Find the actual file
    let entry = cooklang_find::get_recipe(vec![&state.base_path], &recipe_path).map_err(|_| {
        tracing::error!("Recipe not found: {path}");
        StatusCode::NOT_FOUND
    })?;

    let file_path = entry.path().ok_or_else(|| {
        tracing::error!("Recipe has no file path: {path}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Read raw content
    let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
        tracing::error!("Failed to read recipe file: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get recipe name from path
    let recipe_name = path
        .split('/')
        .next_back()
        .unwrap_or(&path)
        .replace(".cook", "")
        .replace(".menu", "");

    let template = crate::server::templates::EditTemplate {
        active: "recipes".to_string(),
        recipe_name,
        recipe_path: path,
        content,
        base_path: state.base_path.to_string(),
        tr: crate::server::templates::Tr::new(lang),
    };

    Ok(template)
}

#[derive(Deserialize, Default)]
struct NewPageQuery {
    error: Option<String>,
    filename: Option<String>,
}

async fn new_page(
    Extension(lang): Extension<LanguageIdentifier>,
    Query(query): Query<NewPageQuery>,
) -> impl askama_axum::IntoResponse {
    crate::server::templates::NewTemplate {
        active: "recipes".to_string(),
        tr: Tr::new(lang),
        error: query.error,
        filename: query.filename,
    }
}

#[derive(Deserialize)]
struct NewRecipeForm {
    filename: String,
}

/// Helper to build redirect URL with error message
fn new_page_error(error: &str, filename: &str) -> axum::response::Response {
    let encoded_error = urlencoding::encode(error);
    let encoded_filename = urlencoding::encode(filename);
    axum::response::Redirect::to(&format!(
        "/new?error={}&filename={}",
        encoded_error, encoded_filename
    ))
    .into_response()
}

/// Validates that the request originated from the same host (CSRF protection)
fn validate_same_origin(headers: &HeaderMap, host: &str) -> bool {
    // Check Origin header first (preferred for CSRF protection)
    if let Some(origin) = headers.get(header::ORIGIN) {
        if let Ok(origin_str) = origin.to_str() {
            // Origin format is scheme://host[:port]
            if let Ok(origin_url) = url::Url::parse(origin_str) {
                if let Some(origin_host) = origin_url.host_str() {
                    let origin_with_port = if let Some(port) = origin_url.port() {
                        format!("{}:{}", origin_host, port)
                    } else {
                        origin_host.to_string()
                    };
                    return origin_with_port == host || origin_host == host;
                }
            }
        }
        return false;
    }

    // Fallback to Referer header (less reliable but better than nothing)
    if let Some(referer) = headers.get(header::REFERER) {
        if let Ok(referer_str) = referer.to_str() {
            if let Ok(referer_url) = url::Url::parse(referer_str) {
                if let Some(referer_host) = referer_url.host_str() {
                    let referer_with_port = if let Some(port) = referer_url.port() {
                        format!("{}:{}", referer_host, port)
                    } else {
                        referer_host.to_string()
                    };
                    return referer_with_port == host || referer_host == host;
                }
            }
        }
        return false;
    }

    // No Origin or Referer header - reject for safety
    // (though browsers should always send one for form submissions)
    false
}

async fn create_recipe(
    State(state): State<Arc<AppState>>,
    Host(host): Host,
    headers: HeaderMap,
    Form(form): Form<NewRecipeForm>,
) -> impl IntoResponse {
    // CSRF protection: verify request came from same origin
    if !validate_same_origin(&headers, &host) {
        tracing::warn!("CSRF validation failed for create_recipe request");
        return (StatusCode::FORBIDDEN, "Invalid request origin").into_response();
    }

    let original_filename = form.filename.clone();

    // Validate input before sanitization
    if form.filename.trim().is_empty() {
        return new_page_error("Recipe name cannot be empty", &original_filename);
    }

    // Sanitize path - allow alphanumeric, space, dash, underscore, and forward slash
    let recipe_path: String = form
        .filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_' || *c == '/')
        .collect();

    // Clean up path: remove leading/trailing slashes, collapse multiple slashes
    let recipe_path = recipe_path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");

    if recipe_path.is_empty() {
        return new_page_error("Recipe name cannot be empty", &original_filename);
    }

    let file_path = state.base_path.join(format!("{}.cook", recipe_path));

    // Security: Validate path structure before any filesystem operations
    // Check that the constructed path, when normalized, stays within base_path
    let base_path_clone = state.base_path.clone();
    let base_canonical =
        match tokio::task::spawn_blocking(move || base_path_clone.canonicalize_utf8()).await {
            Ok(Ok(p)) => p,
            _ => {
                return new_page_error("Internal error: invalid base path", &original_filename);
            }
        };

    // Validate parent path components don't escape base_path
    // We do this by checking the joined path doesn't contain .. after normalization
    let normalized_path = file_path.as_str().replace("\\", "/");
    if normalized_path.contains("/../") || normalized_path.ends_with("/..") {
        tracing::warn!("Path traversal attempt detected in: {}", recipe_path);
        return new_page_error("Invalid recipe path", &original_filename);
    }

    // For the file path, we check the parent directory
    if let Some(parent) = file_path.parent() {
        // Create parent directories if they don't exist
        if !parent.exists() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                tracing::error!("Failed to create directories: {}", e);
                return new_page_error("Failed to create directory", &original_filename);
            }
        }

        // Now verify the created parent is under base_path
        let parent_owned = parent.to_owned();
        match tokio::task::spawn_blocking(move || parent_owned.canonicalize_utf8()).await {
            Ok(Ok(parent_canonical)) => {
                if !parent_canonical.starts_with(&base_canonical) {
                    tracing::warn!(
                        "Path traversal attempt: {} not under {}",
                        parent_canonical,
                        base_canonical
                    );
                    // Clean up the created directory if it's outside base_path
                    let _ = tokio::fs::remove_dir_all(parent).await;
                    return new_page_error("Invalid recipe path", &original_filename);
                }
            }
            _ => {
                return new_page_error("Invalid recipe path", &original_filename);
            }
        }
    }

    // Get the recipe name (last component of path) for the title
    let recipe_name = recipe_path
        .split('/')
        .next_back()
        .unwrap_or(&recipe_path)
        .replace(['-', '_'], " ");

    // Create recipe with YAML frontmatter
    let template = format!("---\ntitle: {}\n---\n\n", recipe_name);

    // Use OpenOptions with create_new to atomically check existence and create
    // This prevents TOCTOU race conditions
    use tokio::io::AsyncWriteExt;
    let file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true) // Fails if file exists - atomic check + create
        .open(&file_path)
        .await;

    match file {
        Ok(mut f) => {
            if let Err(e) = f.write_all(template.as_bytes()).await {
                tracing::error!("Failed to write recipe: {}", e);
                return new_page_error("Failed to write recipe file", &original_filename);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return new_page_error("A recipe with this name already exists", &original_filename);
        }
        Err(e) => {
            tracing::error!("Failed to create recipe file: {}", e);
            return new_page_error("Failed to create recipe file", &original_filename);
        }
    }

    // Redirect to editor
    axum::response::Redirect::to(&format!("/edit/{}.cook", recipe_path)).into_response()
}

fn get_image_path(base_path: &Utf8PathBuf, img_path: String) -> Option<String> {
    tracing::debug!("Recipe image path from entry: {}", img_path);
    // If it's a URL, use it directly
    if img_path.starts_with("http://") || img_path.starts_with("https://") {
        Some(img_path)
    } else {
        // For file paths, we need to make them relative to the base path and accessible via /api/static
        let img_path = camino::Utf8Path::new(&img_path);

        // Try to strip the base_path prefix to get a relative path
        if let Ok(relative) = img_path.strip_prefix(base_path) {
            let result = format!("/api/static/{relative}");
            tracing::debug!("Image path relative to base: {}", result);
            Some(result)
        } else {
            // If the path doesn't start with base_path, it might already be relative
            // or it might be an absolute path to a file within base_path
            // Let's check if it's a file name or relative path
            if !img_path.is_absolute() {
                Some(format!("/api/static/{img_path}"))
            } else {
                // Last resort: try to get just the filename
                img_path
                    .file_name()
                    .map(|name| format!("/api/static/{name}"))
            }
        }
    }
}

async fn menu_page_handler(
    path: String,
    scale: f64,
    entry: cooklang_find::RecipeEntry,
    state: Arc<AppState>,
    lang: LanguageIdentifier,
) -> Result<MenuTemplate, StatusCode> {
    let recipe = crate::util::parse_recipe_from_entry(&entry, scale).map_err(|e| {
        tracing::error!("Failed to parse menu: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get the image path if available
    let image_path = entry.title_image().clone().and_then(|img_path| {
        if img_path.starts_with("http://") || img_path.starts_with("https://") {
            Some(img_path)
        } else {
            let img_path = camino::Utf8Path::new(&img_path);
            if let Ok(relative) = img_path.strip_prefix(&state.base_path) {
                Some(format!("/api/static/{relative}"))
            } else if !img_path.is_absolute() {
                Some(format!("/api/static/{img_path}"))
            } else {
                img_path
                    .file_name()
                    .map(|name| format!("/api/static/{name}"))
            }
        }
    });

    let breadcrumbs: Vec<String> = path.split('/').map(|s| s.to_string()).collect();

    // Parse sections and content
    let mut sections = Vec::new();

    for section in &recipe.sections {
        let section_name = section.name.clone();
        let mut lines = Vec::new();

        for content in &section.content {
            use cooklang::Content;
            if let Content::Step(step) = content {
                // Build the full step content first
                let mut step_items = Vec::new();
                let mut current_text = String::new();

                for item in &step.items {
                    use crate::server::templates::MenuSectionItem;
                    use cooklang::Item;

                    match item {
                        Item::Text { value } => {
                            // Check if this is an isolated dash (bullet marker)
                            if value == "-" {
                                // Bullet marker - complete current line and start new one
                                if !current_text.is_empty() {
                                    step_items.push(MenuSectionItem::Text(current_text.clone()));
                                    current_text.clear();
                                }
                                if !step_items.is_empty() {
                                    lines.push(step_items.clone());
                                    step_items.clear();
                                }
                            } else {
                                current_text.push_str(value);
                            }
                        }
                        Item::Ingredient { index } => {
                            // First, add any pending text
                            if !current_text.is_empty() {
                                step_items.push(MenuSectionItem::Text(current_text.clone()));
                                current_text.clear();
                            }

                            if let Some(ing) = recipe.ingredients.get(*index) {
                                // Check if this is a recipe reference using the reference field
                                if let Some(ref recipe_ref) = ing.reference {
                                    // This is a recipe reference
                                    let recipe_scale =
                                        ing.quantity.as_ref().and_then(|q| match q.value() {
                                            cooklang::Value::Number(n) => Some(n.value()),
                                            _ => None,
                                        });

                                    // Apply menu scaling to the recipe reference
                                    let final_scale = recipe_scale.map(|s| s * scale);

                                    // Build the full path from components
                                    // For web URLs - always use forward slash
                                    let name = if recipe_ref.components.is_empty() {
                                        recipe_ref.name.clone()
                                    } else {
                                        format!(
                                            "{}/{}",
                                            recipe_ref.components.join("/"),
                                            recipe_ref.name
                                        )
                                    };

                                    step_items.push(MenuSectionItem::RecipeReference {
                                        name,
                                        scale: final_scale,
                                    });
                                } else {
                                    // Regular ingredient
                                    let quantity = ing.quantity.as_ref().and_then(|q| {
                                        crate::util::format::format_quantity(q.value())
                                    });
                                    let unit = ing
                                        .quantity
                                        .as_ref()
                                        .and_then(|q| q.unit().as_ref().map(|u| u.to_string()));

                                    step_items.push(MenuSectionItem::Ingredient {
                                        name: ing.name.to_string(),
                                        quantity,
                                        unit,
                                    });
                                }
                            }
                        }
                        _ => {} // Ignore other items in menu files
                    }
                }

                // Add any remaining content as a line
                if !current_text.is_empty() {
                    step_items.push(MenuSectionItem::Text(current_text));
                }
                if !step_items.is_empty() {
                    lines.push(step_items);
                }
            }
        }

        if !lines.is_empty() {
            sections.push(MenuSection {
                name: section_name,
                lines,
            });
        }
    }

    // Get metadata
    let metadata = if recipe.metadata.map.is_empty() {
        None
    } else {
        let get_field = |key: &str| -> Option<String> {
            recipe.metadata.get(key).and_then(|v| {
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else if let Some(n) = v.as_i64() {
                    Some(n.to_string())
                } else {
                    v.as_f64().map(crate::util::format::format_number)
                }
            })
        };

        let mut custom_metadata = Vec::new();
        for (key, value) in recipe.metadata.map_filtered() {
            if let (Some(key_str), Some(val_str)) = (key.as_str(), value.as_str()) {
                custom_metadata.push((key_str.to_string(), val_str.to_string()));
            }
        }

        Some(RecipeMetadata {
            servings: get_field("servings"),
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

    let menu_name = recipe
        .metadata
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            path.split('/')
                .next_back()
                .unwrap_or(&path)
                .replace(".menu", "")
        });

    let template = MenuTemplate {
        active: "recipes".to_string(),
        name: menu_name,
        recipe_path: path,
        breadcrumbs,
        scale,
        metadata,
        sections,
        image_path,
        tr: Tr::new(lang),
    };

    Ok(template)
}

async fn shopping_list_page(
    Extension(lang): Extension<LanguageIdentifier>,
) -> impl askama_axum::IntoResponse {
    ShoppingListTemplate {
        active: "shopping".to_string(),
        tr: Tr::new(lang),
    }
}

async fn pantry_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    // Load pantry configuration
    let pantry_path = state.pantry_path.as_ref();

    let mut sections = Vec::new();

    if let Some(path) = pantry_path {
        if let Ok(content) = tokio::fs::read_to_string(path).await {
            let result = cooklang::pantry::parse_lenient(&content);

            if let Some(pantry_conf) = result.output() {
                // Convert pantry data to template format
                for (section_name, items) in &pantry_conf.sections {
                    let mut pantry_items = Vec::new();

                    for item in items {
                        pantry_items.push(crate::server::templates::PantryItem {
                            name: item.name().to_string(),
                            quantity: item.quantity().map(|q| q.to_string()),
                            bought: item.bought().map(|b| b.to_string()),
                            expire: item.expire().map(|e| e.to_string()),
                            low: item.low().map(|l| l.to_string()),
                        });
                    }

                    sections.push(crate::server::templates::PantrySection {
                        name: section_name.clone(),
                        items: pantry_items,
                    });
                }
            }
        }
    }

    Ok(PantryTemplate {
        active: "pantry".to_string(),
        configured: pantry_path.is_some(),
        sections,
        tr: Tr::new(lang),
    })
}

async fn preferences_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> impl askama_axum::IntoResponse {
    PreferencesTemplate {
        active: "preferences".to_string(),
        aisle_path: state
            .aisle_path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "Not configured".to_string()),
        pantry_path: state
            .pantry_path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "Not configured".to_string()),
        base_path: state.base_path.to_string(),
        version: format!("{} - in food we trust", env!("CARGO_PKG_VERSION")),
        tr: Tr::new(lang),
    }
}
