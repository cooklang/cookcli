//! Format a recipe as [schema.org/Recipe](https://schema.org/Recipe) JSON-LD.
//!
//! This is the shape search engines read, so the field names are fixed by the
//! vocabulary rather than chosen here.

use crate::number::format_quantity_with_unit;
use crate::quantity::grouped_quantity_fmt;
use cooklang::{convert::Converter, model::Item, Recipe};
use serde_json::{json, Value};
use std::io;

/// Write `recipe` as a schema.org `Recipe` JSON-LD object.
///
/// `pretty` indents the JSON. Nothing here can fail except the write itself,
/// which `serde_json` reports as the underlying [`io::Error`].
pub fn print_schema(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
    writer: impl io::Write,
    pretty: bool,
) -> io::Result<()> {
    let schema = create_schema_object(recipe, name, scale, converter);

    if pretty {
        serde_json::to_writer_pretty(writer, &schema)?;
    } else {
        serde_json::to_writer(writer, &schema)?;
    }

    Ok(())
}

/// Build the JSON-LD object. Infallible: every field is optional, and a
/// missing or unparseable one is simply left out.
fn create_schema_object(recipe: &Recipe, name: &str, scale: f64, converter: &Converter) -> Value {
    let mut schema = json!({
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": if scale != 1.0 {
            format!("{name} (scaled {scale}x)")
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
    add_time_fields(&mut schema, recipe, converter);

    // Add nutrition information if present
    add_nutrition_info(&mut schema, recipe);

    // Add ingredients
    let ingredients = create_ingredients_list(recipe, converter);
    if !ingredients.is_empty() {
        schema["recipeIngredient"] = json!(ingredients);
    }

    // Add cookware as tools
    let tools = create_tools_list(recipe, converter);
    if !tools.is_empty() {
        schema["tool"] = json!(tools);
    }

    // Add instructions
    let instructions = create_instructions_list(recipe);
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

    schema
}

fn add_time_fields(schema: &mut Value, recipe: &Recipe, converter: &Converter) {
    // `metadata.time` is cooklang's own parser — the same one `human` output
    // uses — so schema.org durations accept what the rest of the CLI accepts
    // (a `time:` total, `prep time`/`cook time`, YAML integers) and reject
    // what it rejects (ranges, unknown units) instead of guessing (#705). It
    // reports whole minutes, so anything under a minute is dropped.
    match recipe.metadata.time(converter) {
        Some(cooklang::metadata::RecipeTime::Total(minutes)) => {
            schema["totalTime"] = json!(minutes_iso(minutes));
        }
        Some(cooklang::metadata::RecipeTime::Composed {
            prep_time,
            cook_time,
        }) => {
            if let Some(prep) = prep_time {
                schema["prepTime"] = json!(minutes_iso(prep));
            }
            if let Some(cook) = cook_time {
                schema["cookTime"] = json!(minutes_iso(cook));
            }
            if let (Some(prep), Some(cook)) = (prep_time, cook_time) {
                schema["totalTime"] = json!(minutes_iso(prep.saturating_add(cook)));
            }
        }
        None => {}
    }
}

/// A whole number of minutes as an ISO 8601 duration, decomposed into days,
/// hours and minutes (`P2D`, `PT1H30M`).
fn minutes_iso(minutes: u32) -> String {
    let days = minutes / 1_440;
    let hours = minutes % 1_440 / 60;
    let minutes = minutes % 60;

    let mut out = String::from("P");
    if days != 0 {
        out += &format!("{days}D");
    }
    let mut time = String::new();
    if hours != 0 {
        time += &format!("{hours}H");
    }
    if minutes != 0 {
        time += &format!("{minutes}M");
    }
    if !time.is_empty() {
        out.push('T');
        out += &time;
    }
    if out == "P" {
        return "PT0M".to_string();
    }
    out
}

fn add_nutrition_info(schema: &mut Value, recipe: &Recipe) {
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
}

fn create_ingredients_list(recipe: &Recipe, converter: &Converter) -> Vec<String> {
    let mut ingredients = Vec::new();

    for entry in recipe.group_ingredients(converter) {
        let ingredient = entry.ingredient;

        if !ingredient.modifiers().should_be_listed() {
            continue;
        }

        let mut ingredient_text = String::new();

        if !entry.quantity.is_empty() {
            ingredient_text.push_str(&grouped_quantity_fmt(&entry.quantity));
            ingredient_text.push(' ');
        }

        ingredient_text.push_str(&ingredient.display_name());

        if ingredient.modifiers().is_optional() {
            ingredient_text.push_str(" (optional)");
        }

        if let Some(note) = &ingredient.note {
            ingredient_text.push_str(&format!(", {note}"));
        }

        ingredients.push(ingredient_text);
    }

    ingredients
}

fn create_tools_list(recipe: &Recipe, converter: &Converter) -> Vec<String> {
    let mut tools = Vec::new();

    for item in recipe.group_cookware(converter) {
        let cw = item.cookware;

        let mut tool_text = String::new();

        if !item.quantity.is_empty() {
            tool_text.push_str(&grouped_quantity_fmt(&item.quantity));
            tool_text.push(' ');
        }

        tool_text.push_str(cw.display_name());

        if cw.modifiers().is_optional() {
            tool_text.push_str(" (optional)");
        }

        if let Some(note) = &cw.note {
            tool_text.push_str(&format!(", {note}"));
        }

        tools.push(tool_text);
    }

    tools
}

fn build_step(recipe: &Recipe, step: &cooklang::model::Step, step_number: usize) -> Value {
    let mut step_text = String::new();
    let mut total_seconds: f64 = 0.0;
    let mut has_timers = false;

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
                    step_text.push_str(&format!("{name} for "));
                }
                if let Some(quantity) = &t.quantity {
                    step_text.push_str(&format_quantity_with_unit(quantity));

                    // Accumulate timer duration for timeRequired.
                    // Range values are averaged (e.g. 15-30 min → 22.5 min).
                    // Note: the parser produces Range only from scaling operations,
                    // not from syntax like ~{15-30%min} (which parses as Text).
                    let seconds = match quantity.value() {
                        cooklang::quantity::Value::Number(n) => Some(n.value()),
                        cooklang::quantity::Value::Range { start, end } => {
                            Some((start.value() + end.value()) / 2.0)
                        }
                        cooklang::quantity::Value::Text(_) => None,
                    };
                    if let Some(val) = seconds {
                        let multiplier = match quantity.unit().map(|u| u.to_lowercase()).as_deref()
                        {
                            Some("s" | "sec" | "second" | "seconds") => Some(1.0),
                            Some("h" | "hr" | "hour" | "hours") => Some(3600.0),
                            // Cooklang timers without an explicit unit default to minutes
                            Some("m" | "min" | "minute" | "minutes") | None => Some(60.0),
                            Some(_) => None, // unknown unit — skip rather than guess
                        };
                        if let Some(m) = multiplier {
                            total_seconds += val * m;
                            has_timers = true;
                        }
                    }
                }
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                step_text.push_str(&format_quantity_with_unit(q));
            }
        }
    }

    let mut instruction = json!({
        "@type": "HowToStep",
        "position": step_number,
        "text": step_text.trim()
    });

    // Add timeRequired if the step has timers with a positive total duration.
    // The total_secs > 0 guard also prevents emitting a bare "PT" string
    // when all timers round to zero (e.g. ~{0%minutes}).
    let total_secs = total_seconds.round() as i64;
    if has_timers && total_secs > 0 {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        let secs = total_secs % 60;

        let mut dur = "PT".to_string();
        if hours > 0 {
            dur.push_str(&format!("{hours}H"));
        }
        if mins > 0 {
            dur.push_str(&format!("{mins}M"));
        }
        if secs > 0 {
            dur.push_str(&format!("{secs}S"));
        }
        instruction["timeRequired"] = json!(dur);
    }

    instruction
}

fn create_instructions_list(recipe: &Recipe) -> Vec<Value> {
    let mut instructions = Vec::new();
    // Step position is global across all sections, consistent with Google's
    // structured data examples for Recipe.
    let mut step_number = 0;

    for section in &recipe.sections {
        let mut section_steps = Vec::new();
        for content in &section.content {
            if let cooklang::Content::Step(step) = content {
                step_number += 1;
                section_steps.push(build_step(recipe, step, step_number));
            }
        }

        if section_steps.is_empty() {
            continue;
        }

        if let Some(name) = &section.name {
            // Wrap in HowToSection only for named sections
            let section_obj = json!({
                "@type": "HowToSection",
                "name": name,
                "itemListElement": section_steps
            });
            instructions.push(section_obj);
        } else {
            // Unnamed sections emit steps flat
            instructions.extend(section_steps);
        }
    }

    instructions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minutes_decompose_into_iso_8601() {
        assert_eq!(minutes_iso(90), "PT1H30M");
        assert_eq!(minutes_iso(120), "PT2H");
        assert_eq!(minutes_iso(30), "PT30M");
        assert_eq!(minutes_iso(45), "PT45M");
        assert_eq!(minutes_iso(1_440), "P1D");
        assert_eq!(minutes_iso(2_880), "P2D");
        assert_eq!(minutes_iso(0), "PT0M");
    }

    /// `metadata.time` is cooklang's own parser: it understands the
    /// `prep time`/`cook time` pair, YAML integers and the `time:` total,
    /// and refuses what it cannot read rather than guessing (#705).
    #[test]
    fn schema_times_come_from_the_shared_parser() {
        let recipe = crate::test_support::parse_recipe(
            "---\nprep time: 10 min\ncook time: 30\n---\n\nBoil @water{}.\n",
            "r",
            1.0,
        )
        .expect("parses")
        .value;
        let mut schema = json!({});
        add_time_fields(&mut schema, &recipe, &Converter::default());
        assert_eq!(schema["prepTime"], json!("PT10M"), "{schema}");
        assert_eq!(schema["cookTime"], json!("PT30M"), "{schema}");
        assert_eq!(schema["totalTime"], json!("PT40M"), "{schema}");

        let recipe = crate::test_support::parse_recipe(
            "---\ntime: 1h30m\n---\n\nBoil @water{}.\n",
            "r",
            1.0,
        )
        .expect("parses")
        .value;
        let mut schema = json!({});
        add_time_fields(&mut schema, &recipe, &Converter::default());
        assert_eq!(schema["totalTime"], json!("PT1H30M"), "{schema}");

        // An unreadable duration is omitted, not emitted as a wrong `PT0M`.
        let recipe = crate::test_support::parse_recipe(
            "---\nprep time: about an hour\n---\n\nBoil @water{}.\n",
            "r",
            1.0,
        )
        .expect("parses")
        .value;
        let mut schema = json!({});
        add_time_fields(&mut schema, &recipe, &Converter::default());
        assert_eq!(schema["prepTime"], Value::Null, "{schema}");
        assert_eq!(schema["totalTime"], Value::Null, "{schema}");
    }
}
