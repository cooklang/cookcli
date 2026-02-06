use anyhow::Result;
use cooklang::{
    convert::Converter,
    model::{Item, Section, Step},
    Recipe,
};
use std::io;

pub fn print_typst(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
    mut writer: impl io::Write,
) -> Result<()> {
    write_document_header(&mut writer)?;

    writeln!(writer)?;
    writeln!(writer, "// BEGIN_RECIPE_CONTENT")?;

    write_title(&mut writer, name, scale)?;

    if let Some(desc) = recipe.metadata.description() {
        write_description(&mut writer, desc)?;
    }

    if let Some(tags) = recipe.metadata.tags() {
        let tags_vec: Vec<String> = tags.into_iter().map(|t| t.to_string()).collect();
        write_tags(&mut writer, &tags_vec)?;
    }

    write_metadata(&mut writer, recipe)?;

    write_ingredients(&mut writer, recipe, converter)?;

    write_cookware(&mut writer, recipe, converter)?;

    write_instructions(&mut writer, recipe)?;

    writeln!(writer, "// END_RECIPE_CONTENT")?;

    write_document_footer(&mut writer)?;

    Ok(())
}

fn write_document_header(w: &mut impl io::Write) -> Result<()> {
    writeln!(
        w,
        r#"#set page(paper: "a4", margin: (left: 2.5cm, right: 2.5cm, top: 2.5cm, bottom: 2.5cm))"#
    )?;
    writeln!(w)?;
    writeln!(w, r"#set text(size: 11pt)")?;
    writeln!(w)?;
    writeln!(w, r"// Define colors for recipe elements")?;
    writeln!(w, r"#let ingredientcolor = rgb(204, 85, 0)")?;
    writeln!(w, r"#let cookwarecolor = rgb(34, 139, 34)")?;
    writeln!(w, r"#let timercolor = rgb(220, 20, 60)")?;
    writeln!(w)?;
    writeln!(w, r"// Custom commands for recipe elements")?;
    writeln!(
        w,
        "#let ingredient(ingredient) = {{ text(fill: ingredientcolor)[*#ingredient*] }}"
    )?;
    writeln!(
        w,
        r"#let cookware(cookware) = {{ text(fill: cookwarecolor)[*#cookware*] }}"
    )?;
    writeln!(
        w,
        r"#let timer(timer) = {{ text(fill: timercolor)[*#timer*] }}"
    )?;

    Ok(())
}

fn write_document_footer(w: &mut impl io::Write) -> Result<()> {
    writeln!(w)?;
    writeln!(w, r"#v(1fr)")?;
    writeln!(w, r"#set align(center)")?;
    writeln!(w, r"#set text(10pt)")?;
    writeln!(w, r"_Created with CookCLI_")?;

    Ok(())
}

fn write_title(w: &mut impl io::Write, name: &str, scale: f64) -> Result<()> {
    writeln!(w)?;
    writeln!(w, "// BEGIN_TITLE")?;
    let escaped_name = escape_typst(name);
    writeln!(w, r"#set align(center)")?;
    if scale != 1.0 {
        writeln!(w, r"= {escaped_name} @ {scale}")?;
    } else {
        writeln!(w, r"= {escaped_name}")?;
    }
    writeln!(w, r"#set align(left)")?;
    writeln!(w, r"#v(0.5cm)")?;
    writeln!(w, "// END_TITLE")?;
    writeln!(w)?;
    Ok(())
}

fn write_description(w: &mut impl io::Write, description: &str) -> Result<()> {
    writeln!(w, "// DESCRIPTION: {}", description.replace('\n', " "))?;
    writeln!(w, r#"#quote[_"{}"_]"#, escape_typst(description))?;
    writeln!(w)?;
    Ok(())
}

fn write_tags(w: &mut impl io::Write, tags: &[String]) -> Result<()> {
    if !tags.is_empty() {
        writeln!(w, "// TAGS: {}", tags.join(", "))?;
        write!(w, r"*Tags:* ")?;
        for (i, tag) in tags.iter().enumerate() {
            write!(w, r"`{}`", escape_typst(tag))?;
            if i < tags.len() - 1 {
                write!(w, ", ")?;
            }
        }
        writeln!(w)?;
        writeln!(w)?;
    }
    Ok(())
}

fn write_metadata(w: &mut impl io::Write, recipe: &Recipe) -> Result<()> {
    let mut metadata_items = Vec::new();

    if let Some(servings) = recipe.metadata.servings() {
        writeln!(w, "// SERVINGS: {servings}")?;
        metadata_items.push(format!("Servings: {servings}"));
    }

    // Get prep time from metadata
    if let Some(prep_time_val) = recipe.metadata.get("prep time") {
        if let Some(prep_time_str) = prep_time_val.as_str() {
            writeln!(w, "// PREP_TIME: {prep_time_str}")?;
            metadata_items.push(format!("Prep time: {prep_time_str}"));
        }
    }

    // Get cook time from metadata
    if let Some(cook_time_val) = recipe.metadata.get("cook time") {
        if let Some(cook_time_str) = cook_time_val.as_str() {
            writeln!(w, "// COOK_TIME: {cook_time_str}")?;
            metadata_items.push(format!("Cook time: {cook_time_str}"));
        }
    }

    // Add author if present
    if let Some(author) = recipe.metadata.author() {
        if let Some(author_name) = author.name() {
            writeln!(w, "// AUTHOR: {author_name}")?;
        }
    }

    // Add source if present
    if let Some(source) = recipe.metadata.source() {
        if let Some(url) = source.url() {
            writeln!(w, "// SOURCE: {url}")?;
        } else if let Some(name) = source.name() {
            writeln!(w, "// SOURCE: {name}")?;
        }
    }

    if !metadata_items.is_empty() {
        writeln!(w, "// BEGIN_METADATA")?;
        writeln!(w, r"#set align(center)")?;
        for (i, item) in metadata_items.iter().enumerate() {
            write!(w, "{}", escape_typst(item))?;
            if i < metadata_items.len() - 1 {
                write!(w, r" #h(1em) ")?;
            }
        }
        writeln!(w)?;
        writeln!(w, r"#set align(left)")?;
        writeln!(w, "// END_METADATA")?;
        writeln!(w)?;
    }

    Ok(())
}

fn write_ingredients(w: &mut impl io::Write, recipe: &Recipe, converter: &Converter) -> Result<()> {
    if recipe.ingredients.is_empty() {
        return Ok(());
    }

    writeln!(w, r"== Ingredients")?;
    writeln!(w)?;

    //Typst does offer a column element, but ut does not balance the element height, instead it fills the parent container height. Balancing is planned for the future now. I think it's best to omit multiple columns for now.
    //writeln!(w, r"\begin{{multicols}}{{2}}")?;

    for entry in recipe.group_ingredients(converter) {
        let ingredient = entry.ingredient;

        if !ingredient.modifiers().should_be_listed() {
            continue;
        }

        write!(w, r"- ")?;

        if !entry.quantity.is_empty() {
            write!(w, r"*{}* ", escape_typst(&entry.quantity.to_string()))?;
        }

        if let Some(reference) = &ingredient.reference {
            let sep = std::path::MAIN_SEPARATOR.to_string();
            let path = reference.components.join(&sep);
            write!(
                w,
                r#"#ingredient("{}")"#,
                escape_typst(&ingredient.display_name())
            )?;
            write!(
                w,
                r" _(see recipe: {}{}{})_",
                escape_typst(&path),
                sep,
                escape_typst(&ingredient.name)
            )?;
        } else {
            write!(
                w,
                r#"#ingredient("{}")"#,
                escape_typst(&ingredient.display_name())
            )?;
        }

        if ingredient.modifiers().is_optional() {
            write!(w, r" _(optional)_")?;
        }

        if let Some(note) = &ingredient.note {
            write!(w, r" --- {}", escape_typst(note))?;
        }

        writeln!(w)?;
    }

    writeln!(w)?;

    Ok(())
}

fn write_cookware(w: &mut impl io::Write, recipe: &Recipe, converter: &Converter) -> Result<()> {
    if recipe.cookware.is_empty() {
        return Ok(());
    }

    writeln!(w, r"== Cookware")?;
    writeln!(w)?;

    for item in recipe.group_cookware(converter) {
        let cw = item.cookware;

        write!(w, r"- ")?;

        if !item.quantity.is_empty() {
            write!(w, r"*{}* ", escape_typst(&item.quantity.to_string()))?;
        }

        write!(w, r#"#cookware("{}")"#, escape_typst(cw.display_name()))?;

        if cw.modifiers().is_optional() {
            write!(w, r" _(optional)_")?;
        }

        if let Some(note) = &cw.note {
            write!(w, r" --- {}", escape_typst(note))?;
        }

        writeln!(w)?;
    }

    writeln!(w)?;

    Ok(())
}

fn write_instructions(w: &mut impl io::Write, recipe: &Recipe) -> Result<()> {
    writeln!(w, r"== Instructions")?;
    writeln!(w)?;

    for (idx, section) in recipe.sections.iter().enumerate() {
        write_section(w, section, recipe, idx + 1)?;
    }

    Ok(())
}

fn write_section(
    w: &mut impl io::Write,
    section: &Section,
    recipe: &Recipe,
    num: usize,
) -> Result<()> {
    if section.name.is_some() || recipe.sections.len() > 1 {
        if let Some(name) = &section.name {
            writeln!(w, r"=== {}", escape_typst(name))?;
        } else {
            writeln!(w, r"=== Section {num}")?;
        }
        writeln!(w)?;
    }

    for content in &section.content {
        match content {
            cooklang::Content::Step(step) => {
                write_step(w, step, recipe)?;
            }
            cooklang::Content::Text(text) => {
                if text.trim() != "-" {
                    writeln!(w)?;
                    writeln!(w, r"_Note: {}_", escape_typst(text.trim()))?;
                    writeln!(w)?;
                }
            }
        }
    }

    writeln!(w)?;

    Ok(())
}

fn write_step(w: &mut impl io::Write, step: &Step, recipe: &Recipe) -> Result<()> {
    write!(w, r"+ ")?;

    for item in &step.items {
        match item {
            Item::Text { value } => {
                write!(w, "{}", escape_typst(value))?;
            }
            &Item::Ingredient { index } => {
                let igr = &recipe.ingredients[index];
                write!(w, r#"#ingredient("{}")"#, escape_typst(&igr.display_name()))?;
            }
            &Item::Cookware { index } => {
                let cw = &recipe.cookware[index];
                write!(w, r#"#cookware("{}")"#, escape_typst(&cw.name))?;
            }
            &Item::Timer { index } => {
                let t = &recipe.timers[index];
                let timer_text = if let Some(name) = &t.name {
                    format!(
                        "{}: {}",
                        name,
                        t.quantity
                            .as_ref()
                            .map_or("".to_string(), |q| q.to_string())
                    )
                } else {
                    t.quantity
                        .as_ref()
                        .map_or("".to_string(), |q| q.to_string())
                };
                write!(w, r#"#timer("{}")"#, escape_typst(&timer_text))?;
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                write!(w, r"*{}*", escape_typst(&q.to_string()))?;
            }
        }
    }

    writeln!(w)?;

    Ok(())
}

fn escape_typst(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\\' => r"\\".to_string(),
            '{' => r"\{".to_string(),
            '}' => r"\}".to_string(),
            '$' => r"\$".to_string(),
            '&' => r"\&".to_string(),
            '#' => r"\#".to_string(),
            '^' => r"\^".to_string(),
            '_' => r"\_".to_string(),
            '~' => r"\~".to_string(),
            '%' => r"\%".to_string(),
            '<' => r"\<".to_string(),
            '>' => r"\>".to_string(),
            '`' => r"\`".to_string(),
            '@' => r"\@".to_string(),
            '=' => r"\=".to_string(),
            '-' => r"\-".to_string(),
            '+' => r"\+".to_string(),
            '/' => r"\/".to_string(),
            '*' => r"\*".to_string(),
            _ => c.to_string(),
        })
        .collect()
}
