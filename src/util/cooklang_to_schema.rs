use anyhow::{Context, Result};
use cooklang::{convert::Converter, model::Item, Recipe};
use serde_json::{json, Value};
use std::io;

pub fn print_schema(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
    writer: impl io::Write,
    pretty: bool,
) -> Result<()> {
    let schema = create_schema_object(recipe, name, scale, converter)?;

    if pretty {
        serde_json::to_writer_pretty(writer, &schema)
            .context("Failed to write Schema.org JSON-LD")?;
    } else {
        serde_json::to_writer(writer, &schema).context("Failed to write Schema.org JSON-LD")?;
    }

    Ok(())
}

fn create_schema_object(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
) -> Result<Value> {
    let mut schema = json!({
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": if scale != 1.0 {
            format!("{} (scaled {}x)", name, scale)
        } else {
            name.to_string()
        }
    });

    // Add description if present
    if let Some(desc) = recipe.metadata.description() {
        schema["description"] = json!(desc);
    }

    // Add tags as keywords
    if let Some(tags) = recipe.metadata.tags() {
        schema["keywords"] = json!(tags.join(", "));
    }

    // Add author information if present
    if let Some(author) = recipe.metadata.author() {
        let author_name = author.name().unwrap_or(author.url().unwrap_or(""));
        schema["author"] = json!({
            "@type": "Person",
            "name": author_name
        });
    }

    // Add source URL if present
    if let Some(source) = recipe.metadata.source() {
        if let Some(url) = source.url() {
            schema["url"] = json!(url);
        } else if let Some(name) = source.name() {
            schema["url"] = json!(name);
        }
    }

    // Add servings/yield
    if let Some(servings) = recipe.metadata.servings() {
        use cooklang::metadata::Servings;
        match servings {
            Servings::Number(n) => {
                // Apply scaling to servings
                let scaled_servings = if scale != 1.0 {
                    (n as f64 * scale).round() as u32
                } else {
                    n
                };
                schema["recipeYield"] = json!(format!("{} servings", scaled_servings));
            }
            Servings::Text(text) => {
                schema["recipeYield"] = json!(text);
            }
        }
    }

    // Add timing information
    add_time_fields(&mut schema, recipe)?;

    // Add nutrition information if present
    add_nutrition_info(&mut schema, recipe)?;

    // Add ingredients
    let ingredients = create_ingredients_list(recipe, converter)?;
    if !ingredients.is_empty() {
        schema["recipeIngredient"] = json!(ingredients);
    }

    // Add cookware as tools
    let tools = create_tools_list(recipe, converter)?;
    if !tools.is_empty() {
        schema["tool"] = json!(tools);
    }

    // Add instructions
    let instructions = create_instructions_list(recipe)?;
    if !instructions.is_empty() {
        schema["recipeInstructions"] = json!(instructions);
    }

    // Add recipe category and cuisine if available
    if let Some(category) = recipe.metadata.map.get("category") {
        if let Some(cat_str) = category.as_str() {
            schema["recipeCategory"] = json!(cat_str);
        }
    }

    if let Some(cuisine) = recipe.metadata.map.get("cuisine") {
        if let Some(cuisine_str) = cuisine.as_str() {
            schema["recipeCuisine"] = json!(cuisine_str);
        }
    }

    // Add image if present
    if let Some(image) = recipe.metadata.map.get("image") {
        if let Some(image_str) = image.as_str() {
            schema["image"] = json!(image_str);
        }
    }

    Ok(schema)
}

fn add_time_fields(schema: &mut Value, recipe: &Recipe) -> Result<()> {
    // Get prep time from metadata
    if let Some(prep_time_val) = recipe.metadata.get("prep time") {
        if let Some(prep_time_str) = prep_time_val.as_str() {
            schema["prepTime"] = json!(format_iso_duration(prep_time_str)?);
        }
    }

    // Get cook time from metadata
    if let Some(cook_time_val) = recipe.metadata.get("cook time") {
        if let Some(cook_time_str) = cook_time_val.as_str() {
            schema["cookTime"] = json!(format_iso_duration(cook_time_str)?);
        }
    }

    // Calculate total time if both prep and cook times are available
    let has_prep = recipe.metadata.get("prep time").is_some();
    let has_cook = recipe.metadata.get("cook time").is_some();
    if has_prep && has_cook {
        // Simplified - just sum the minutes from both times
        let mut total_minutes = 0;
        if let Some(prep_val) = recipe.metadata.get("prep time").and_then(|v| v.as_str()) {
            total_minutes += extract_number(prep_val).unwrap_or(0);
        }
        if let Some(cook_val) = recipe.metadata.get("cook time").and_then(|v| v.as_str()) {
            total_minutes += extract_number(cook_val).unwrap_or(0);
        }
        schema["totalTime"] = json!(format!("PT{}M", total_minutes));
    }

    Ok(())
}

fn format_iso_duration(time_str: &str) -> Result<String> {
    // Convert time strings like "30 minutes" or "1 hour" to ISO 8601 duration format
    // This is a simplified implementation
    let lower = time_str.to_lowercase();

    if lower.contains("hour") {
        if let Some(hours) = extract_number(&lower) {
            return Ok(format!("PT{}H", hours));
        }
    } else if lower.contains("min") {
        if let Some(minutes) = extract_number(&lower) {
            return Ok(format!("PT{}M", minutes));
        }
    }

    // Fallback: assume minutes if just a number
    if let Some(minutes) = extract_number(&lower) {
        return Ok(format!("PT{}M", minutes));
    }

    Ok("PT0M".to_string())
}

fn extract_number(s: &str) -> Option<i32> {
    s.chars()
        .filter(|c| c.is_numeric())
        .collect::<String>()
        .parse::<i32>()
        .ok()
}

fn add_nutrition_info(schema: &mut Value, recipe: &Recipe) -> Result<()> {
    let mut nutrition = json!({
        "@type": "NutritionInformation"
    });

    let mut has_nutrition = false;

    // Check for various nutrition fields in metadata
    if let Some(calories) = recipe.metadata.map.get("calories") {
        if let Some(cal_str) = calories.as_str() {
            nutrition["calories"] = json!(format!("{} calories", cal_str));
            has_nutrition = true;
        } else if let Some(cal_num) = calories.as_u64() {
            nutrition["calories"] = json!(format!("{} calories", cal_num));
            has_nutrition = true;
        }
    }

    if let Some(protein) = recipe.metadata.map.get("protein") {
        if let Some(prot_str) = protein.as_str() {
            nutrition["proteinContent"] = json!(prot_str);
            has_nutrition = true;
        }
    }

    if let Some(fat) = recipe.metadata.map.get("fat") {
        if let Some(fat_str) = fat.as_str() {
            nutrition["fatContent"] = json!(fat_str);
            has_nutrition = true;
        }
    }

    if let Some(carbs) = recipe.metadata.map.get("carbohydrates") {
        if let Some(carbs_str) = carbs.as_str() {
            nutrition["carbohydrateContent"] = json!(carbs_str);
            has_nutrition = true;
        }
    }

    if let Some(fiber) = recipe.metadata.map.get("fiber") {
        if let Some(fiber_str) = fiber.as_str() {
            nutrition["fiberContent"] = json!(fiber_str);
            has_nutrition = true;
        }
    }

    if let Some(sugar) = recipe.metadata.map.get("sugar") {
        if let Some(sugar_str) = sugar.as_str() {
            nutrition["sugarContent"] = json!(sugar_str);
            has_nutrition = true;
        }
    }

    if let Some(sodium) = recipe.metadata.map.get("sodium") {
        if let Some(sodium_str) = sodium.as_str() {
            nutrition["sodiumContent"] = json!(sodium_str);
            has_nutrition = true;
        }
    }

    if has_nutrition {
        schema["nutrition"] = nutrition;
    }

    Ok(())
}

fn create_ingredients_list(recipe: &Recipe, converter: &Converter) -> Result<Vec<String>> {
    let mut ingredients = Vec::new();

    for entry in recipe.group_ingredients(converter) {
        let ingredient = entry.ingredient;

        if !ingredient.modifiers().should_be_listed() {
            continue;
        }

        let mut ingredient_text = String::new();

        if !entry.quantity.is_empty() {
            ingredient_text.push_str(&entry.quantity.to_string());
            ingredient_text.push(' ');
        }

        ingredient_text.push_str(&ingredient.display_name());

        if ingredient.modifiers().is_optional() {
            ingredient_text.push_str(" (optional)");
        }

        if let Some(note) = &ingredient.note {
            ingredient_text.push_str(&format!(", {}", note));
        }

        ingredients.push(ingredient_text);
    }

    Ok(ingredients)
}

fn create_tools_list(recipe: &Recipe, converter: &Converter) -> Result<Vec<String>> {
    let mut tools = Vec::new();

    for item in recipe.group_cookware(converter) {
        let cw = item.cookware;

        let mut tool_text = String::new();

        if !item.quantity.is_empty() {
            tool_text.push_str(&item.quantity.to_string());
            tool_text.push(' ');
        }

        tool_text.push_str(cw.display_name());

        if cw.modifiers().is_optional() {
            tool_text.push_str(" (optional)");
        }

        if let Some(note) = &cw.note {
            tool_text.push_str(&format!(", {}", note));
        }

        tools.push(tool_text);
    }

    Ok(tools)
}

fn create_instructions_list(recipe: &Recipe) -> Result<Vec<Value>> {
    let mut instructions = Vec::new();
    let mut step_number = 0;

    for section in &recipe.sections {
        for content in &section.content {
            match content {
                cooklang::Content::Step(step) => {
                    step_number += 1;

                    let mut step_text = String::new();

                    for item in &step.items {
                        match item {
                            Item::Text { value } => {
                                step_text.push_str(value);
                            }
                            &Item::Ingredient { index } => {
                                let igr = &recipe.ingredients[index];
                                step_text.push_str(&igr.display_name());
                            }
                            &Item::Cookware { index } => {
                                let cw = &recipe.cookware[index];
                                step_text.push_str(&cw.name);
                            }
                            &Item::Timer { index } => {
                                let t = &recipe.timers[index];
                                if let Some(name) = &t.name {
                                    step_text.push_str(&format!("{} for ", name));
                                }
                                if let Some(quantity) = &t.quantity {
                                    step_text.push_str(&quantity.to_string());
                                }
                            }
                            &Item::InlineQuantity { index } => {
                                let q = &recipe.inline_quantities[index];
                                step_text.push_str(&q.to_string());
                            }
                        }
                    }

                    let instruction = json!({
                        "@type": "HowToStep",
                        "name": format!("Step {}", step_number),
                        "text": step_text.trim()
                    });

                    instructions.push(instruction);
                }
                cooklang::Content::Text(_) => {
                    // Skip text content between steps
                }
            }
        }
    }

    Ok(instructions)
}
