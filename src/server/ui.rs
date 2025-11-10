use crate::server::{templates::*, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use camino::Utf8PathBuf;
use serde::Deserialize;
use std::sync::Arc;

pub fn ui() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(recipes_page))
        .route("/directory/*path", get(recipes_directory))
        .route("/recipe/*path", get(recipe_page))
        .route("/shopping-list", get(shopping_list_page))
        .route("/pantry", get(pantry_page))
        .route("/preferences", get(preferences_page))
}

async fn recipes_page(
    State(state): State<Arc<AppState>>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    recipes_handler(state, None).await
}

async fn recipes_directory(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    recipes_handler(state, Some(path)).await
}

async fn recipes_handler(
    state: Arc<AppState>,
    path: Option<String>,
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
        let item_path = if let Some(p) = &path {
            format!("{p}/{name}")
        } else {
            name.to_string()
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
        "All Recipes".to_string()
    };

    let template = RecipesTemplate {
        active: "recipes".to_string(),
        current_name,
        breadcrumbs,
        items,
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
        let template = menu_page_handler(path, scale, entry, state).await?;
        return Ok(template.into_response());
    }

    let recipe = crate::util::parse_recipe_from_entry(&entry, scale).map_err(|e| {
        tracing::error!("Failed to parse recipe: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let tags = entry.tags();

    // Get the image path if available
    let image_path = entry.title_image().clone().and_then(|img_path| {
        tracing::debug!("Recipe image path from entry: {}", img_path);
        // If it's a URL, use it directly
        if img_path.starts_with("http://") || img_path.starts_with("https://") {
            Some(img_path)
        } else {
            // For file paths, we need to make them relative to the base path and accessible via /api/static
            let img_path = camino::Utf8Path::new(&img_path);

            // Try to strip the base_path prefix to get a relative path
            if let Ok(relative) = img_path.strip_prefix(&state.base_path) {
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
    });

    let mut ingredients = Vec::new();
    let mut cookware = Vec::new();
    let mut sections = Vec::new();

    for ingredient in &recipe.ingredients {
        let reference_path = ingredient.reference.as_ref().map(|r| {
            // For web URLs - always use forward slash
            if r.components.is_empty() {
                r.name.clone()
            } else {
                format!("{}/{}", r.components.join("/"), r.name)
            }
        });

        ingredients.push(IngredientData {
            name: ingredient.name.to_string(),
            quantity: ingredient
                .quantity
                .as_ref()
                .and_then(|q| crate::util::format::format_quantity(q.value())),
            unit: ingredient
                .quantity
                .as_ref()
                .and_then(|q| q.unit().as_ref().map(|u| u.to_string())),
            note: ingredient.note.clone(),
            reference_path,
        });
    }

    for item in &recipe.cookware {
        cookware.push(CookwareData {
            name: item.name.to_string(),
        });
    }

    let mut total_steps = 0;
    for section in &recipe.sections {
        let mut section_steps = Vec::new();
        let mut section_notes = Vec::new();
        let mut section_ingredient_indices = std::collections::HashSet::new();

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
                                step_items.push(StepItem::Text(value.to_string()));
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
                    section_steps.push(StepData {
                        items: step_items,
                        ingredients: step_ingredients,
                    });
                }
                Content::Text(text) => {
                    // Skip list bullet items
                    if text.trim() != "-" {
                        section_notes.push(text.trim().to_string());
                    }
                }
            }
        }

        // Only add sections that have steps or notes
        if !section_steps.is_empty() || !section_notes.is_empty() {
            use crate::server::templates::RecipeSection;

            // Collect ingredients used in this section
            let mut section_ingredients = Vec::new();
            for idx in section_ingredient_indices {
                if let Some(ingredient) = recipe.ingredients.get(idx) {
                    let reference_path = ingredient.reference.as_ref().map(|r| {
                        // For web URLs - always use forward slash
                        if r.components.is_empty() {
                            r.name.clone()
                        } else {
                            format!("{}/{}", r.components.join("/"), r.name)
                        }
                    });

                    section_ingredients.push(IngredientData {
                        name: ingredient.name.to_string(),
                        quantity: ingredient
                            .quantity
                            .as_ref()
                            .and_then(|q| crate::util::format::format_quantity(q.value())),
                        unit: ingredient
                            .quantity
                            .as_ref()
                            .and_then(|q| q.unit().as_ref().map(|u| u.to_string())),
                        note: ingredient.note.clone(),
                        reference_path,
                    });
                }
            }

            sections.push(RecipeSection {
                name: section.name.clone(),
                steps: section_steps.clone(),
                notes: section_notes.clone(),
                step_offset: total_steps,
                ingredients: section_ingredients,
            });
            total_steps += section_steps.len();
        }
    }

    let breadcrumbs: Vec<String> = path.split('/').map(|s| s.to_string()).collect();

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
                custom_metadata.push((key_str.to_string(), val_str.to_string()));
            }
        }
        custom_metadata.retain(|(k, _)| !k.starts_with("source.") && !k.starts_with("time."));

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
    };

    Ok(template.into_response())
}

async fn menu_page_handler(
    path: String,
    scale: f64,
    entry: cooklang_find::RecipeEntry,
    state: Arc<AppState>,
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
    };

    Ok(template)
}

async fn shopping_list_page() -> impl askama_axum::IntoResponse {
    ShoppingListTemplate {
        active: "shopping".to_string(),
    }
}

async fn pantry_page(
    State(state): State<Arc<AppState>>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    // Load pantry configuration
    let pantry_path = state.pantry_path.as_ref();

    let mut sections = Vec::new();

    if let Some(path) = pantry_path {
        if let Ok(content) = std::fs::read_to_string(path) {
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
        sections,
    })
}

async fn preferences_page(State(state): State<Arc<AppState>>) -> impl askama_axum::IntoResponse {
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
    }
}
