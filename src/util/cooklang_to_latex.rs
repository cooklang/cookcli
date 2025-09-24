use anyhow::Result;
use cooklang::{
    convert::Converter,
    model::{Item, Section, Step},
    Recipe,
};
use std::io;

pub fn print_latex(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
    mut writer: impl io::Write,
) -> Result<()> {
    write_document_header(&mut writer)?;

    writeln!(writer, "% BEGIN_RECIPE_CONTENT")?;

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

    writeln!(writer, "% END_RECIPE_CONTENT")?;

    write_document_footer(&mut writer)?;

    Ok(())
}

fn write_document_header(w: &mut impl io::Write) -> Result<()> {
    writeln!(w, r"\documentclass[11pt,a4paper]{{article}}")?;
    writeln!(w, r"\usepackage[utf8]{{inputenc}}")?;
    writeln!(w, r"\usepackage[T1]{{fontenc}}")?;
    writeln!(w, r"\usepackage{{lmodern}}")?;
    writeln!(w, r"\usepackage{{textcomp}}")?;
    writeln!(w, r"\usepackage{{microtype}}")?;
    writeln!(w, r"\usepackage{{enumitem}}")?;
    writeln!(w, r"\usepackage{{multicol}}")?;
    writeln!(w, r"\usepackage{{graphicx}}")?;
    writeln!(w, r"\usepackage{{xcolor}}")?;
    writeln!(w, r"\usepackage{{titlesec}}")?;
    writeln!(w, r"\usepackage{{geometry}}")?;
    writeln!(w, r"\usepackage{{hyperref}}")?;
    writeln!(w)?;
    writeln!(
        w,
        r"\geometry{{left=2.5cm,right=2.5cm,top=2.5cm,bottom=2.5cm}}"
    )?;
    writeln!(w)?;
    writeln!(w, r"% Define colors for recipe elements")?;
    writeln!(w, r"\definecolor{{ingredientcolor}}{{RGB}}{{204, 85, 0}}")?;
    writeln!(w, r"\definecolor{{cookwarecolor}}{{RGB}}{{34, 139, 34}}")?;
    writeln!(w, r"\definecolor{{timercolor}}{{RGB}}{{220, 20, 60}}")?;
    writeln!(w)?;
    writeln!(w, r"% Custom commands for recipe elements")?;
    writeln!(
        w,
        r"\newcommand{{\ingredient}}[1]{{\textcolor{{ingredientcolor}}{{\textbf{{#1}}}}}}"
    )?;
    writeln!(
        w,
        r"\newcommand{{\cookware}}[1]{{\textcolor{{cookwarecolor}}{{\textbf{{#1}}}}}}"
    )?;
    writeln!(
        w,
        r"\newcommand{{\timer}}[1]{{\textcolor{{timercolor}}{{\textbf{{#1}}}}}}"
    )?;
    writeln!(w)?;
    writeln!(w, r"% Customize section headers")?;
    writeln!(
        w,
        r"\titleformat{{\section}}{{\Large\bfseries}}{{}}{{0em}}{{}}"
    )?;
    writeln!(
        w,
        r"\titleformat{{\subsection}}{{\large\bfseries}}{{}}{{0em}}{{}}"
    )?;
    writeln!(w)?;
    writeln!(w, r"\begin{{document}}")?;
    writeln!(w)?;
    writeln!(w, r"\pagestyle{{empty}}")?;
    writeln!(w)?;
    Ok(())
}

fn write_document_footer(w: &mut impl io::Write) -> Result<()> {
    writeln!(w)?;
    writeln!(w, r"\vfill")?;
    writeln!(w, r"\begin{{center}}")?;
    writeln!(w, r"\small\textit{{Created with CookCLI}}")?;
    writeln!(w, r"\end{{center}}")?;
    writeln!(w)?;
    writeln!(w, r"\end{{document}}")?;
    Ok(())
}

fn write_title(w: &mut impl io::Write, name: &str, scale: f64) -> Result<()> {
    writeln!(w, "% BEGIN_TITLE")?;
    let escaped_name = escape_latex(name);
    if scale != 1.0 {
        writeln!(w, r"\begin{{center}}")?;
        writeln!(w, r"\huge\textbf{{{} @ {}}}", escaped_name, scale)?;
        writeln!(w, r"\end{{center}}")?;
    } else {
        writeln!(w, r"\begin{{center}}")?;
        writeln!(w, r"\huge\textbf{{{}}}", escaped_name)?;
        writeln!(w, r"\end{{center}}")?;
    }
    writeln!(w, r"\vspace{{0.5cm}}")?;
    writeln!(w, "% END_TITLE")?;
    writeln!(w)?;
    Ok(())
}

fn write_description(w: &mut impl io::Write, description: &str) -> Result<()> {
    writeln!(w, "% DESCRIPTION: {}", description.replace('\n', " "))?;
    writeln!(w, r"\begin{{quote}}")?;
    writeln!(w, r"\textit{{{}}}", escape_latex(description))?;
    writeln!(w, r"\end{{quote}}")?;
    writeln!(w)?;
    Ok(())
}

fn write_tags(w: &mut impl io::Write, tags: &[String]) -> Result<()> {
    if !tags.is_empty() {
        writeln!(w, "% TAGS: {}", tags.join(", "))?;
        write!(w, r"\textbf{{Tags:}} ")?;
        for (i, tag) in tags.iter().enumerate() {
            write!(w, r"\texttt{{{}}}", escape_latex(tag))?;
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
        writeln!(w, "% SERVINGS: {}", servings)?;
        metadata_items.push(format!("Servings: {}", servings));
    }

    // Get prep time from metadata
    if let Some(prep_time_val) = recipe.metadata.get("prep time") {
        if let Some(prep_time_str) = prep_time_val.as_str() {
            writeln!(w, "% PREP_TIME: {}", prep_time_str)?;
            metadata_items.push(format!("Prep time: {}", prep_time_str));
        }
    }

    // Get cook time from metadata
    if let Some(cook_time_val) = recipe.metadata.get("cook time") {
        if let Some(cook_time_str) = cook_time_val.as_str() {
            writeln!(w, "% COOK_TIME: {}", cook_time_str)?;
            metadata_items.push(format!("Cook time: {}", cook_time_str));
        }
    }

    // Add author if present
    if let Some(author) = recipe.metadata.author() {
        if let Some(author_name) = author.name() {
            writeln!(w, "% AUTHOR: {}", author_name)?;
        }
    }

    // Add source if present
    if let Some(source) = recipe.metadata.source() {
        if let Some(url) = source.url() {
            writeln!(w, "% SOURCE: {}", url)?;
        } else if let Some(name) = source.name() {
            writeln!(w, "% SOURCE: {}", name)?;
        }
    }

    if !metadata_items.is_empty() {
        writeln!(w, "% BEGIN_METADATA")?;
        writeln!(w, r"\begin{{center}}")?;
        for (i, item) in metadata_items.iter().enumerate() {
            write!(w, "{}", escape_latex(item))?;
            if i < metadata_items.len() - 1 {
                write!(w, r" \quad ")?;
            }
        }
        writeln!(w)?;
        writeln!(w, r"\end{{center}}")?;
        writeln!(w, "% END_METADATA")?;
        writeln!(w)?;
    }

    Ok(())
}

fn write_ingredients(w: &mut impl io::Write, recipe: &Recipe, converter: &Converter) -> Result<()> {
    if recipe.ingredients.is_empty() {
        return Ok(());
    }

    writeln!(w, r"\section*{{Ingredients}}")?;
    writeln!(w)?;

    writeln!(w, r"\begin{{multicols}}{{2}}")?;
    writeln!(w, r"\begin{{itemize}}[leftmargin=*]")?;

    for entry in recipe.group_ingredients(converter) {
        let ingredient = entry.ingredient;

        if !ingredient.modifiers().should_be_listed() {
            continue;
        }

        write!(w, r"\item ")?;

        if !entry.quantity.is_empty() {
            write!(
                w,
                r"\textbf{{{}}} ",
                escape_latex(&entry.quantity.to_string())
            )?;
        }

        if ingredient.reference.is_some() {
            let path = ingredient.reference.as_ref().unwrap().components.join("/");
            write!(
                w,
                r"\ingredient{{{}}}",
                escape_latex(&ingredient.display_name())
            )?;
            write!(
                w,
                r" \textit{{(see recipe: {}/{})}}",
                escape_latex(&path),
                escape_latex(&ingredient.name)
            )?;
        } else {
            write!(
                w,
                r"\ingredient{{{}}}",
                escape_latex(&ingredient.display_name())
            )?;
        }

        if ingredient.modifiers().is_optional() {
            write!(w, r" \textit{{(optional)}}")?;
        }

        if let Some(note) = &ingredient.note {
            write!(w, r" --- {}", escape_latex(note))?;
        }

        writeln!(w)?;
    }

    writeln!(w, r"\end{{itemize}}")?;
    writeln!(w, r"\end{{multicols}}")?;
    writeln!(w)?;

    Ok(())
}

fn write_cookware(w: &mut impl io::Write, recipe: &Recipe, converter: &Converter) -> Result<()> {
    if recipe.cookware.is_empty() {
        return Ok(());
    }

    writeln!(w, r"\section*{{Cookware}}")?;
    writeln!(w)?;

    writeln!(w, r"\begin{{itemize}}[leftmargin=*]")?;

    for item in recipe.group_cookware(converter) {
        let cw = item.cookware;

        write!(w, r"\item ")?;

        if !item.quantity.is_empty() {
            write!(
                w,
                r"\textbf{{{}}} ",
                escape_latex(&item.quantity.to_string())
            )?;
        }

        write!(w, r"\cookware{{{}}}", escape_latex(cw.display_name()))?;

        if cw.modifiers().is_optional() {
            write!(w, r" \textit{{(optional)}}")?;
        }

        if let Some(note) = &cw.note {
            write!(w, r" --- {}", escape_latex(note))?;
        }

        writeln!(w)?;
    }

    writeln!(w, r"\end{{itemize}}")?;
    writeln!(w)?;

    Ok(())
}

fn write_instructions(w: &mut impl io::Write, recipe: &Recipe) -> Result<()> {
    writeln!(w, r"\section*{{Instructions}}")?;
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
            writeln!(w, r"\subsection*{{{}}}", escape_latex(name))?;
        } else {
            writeln!(w, r"\subsection*{{Section {}}}", num)?;
        }
        writeln!(w)?;
    }

    writeln!(w, r"\begin{{enumerate}}")?;

    for content in &section.content {
        match content {
            cooklang::Content::Step(step) => {
                write_step(w, step, recipe)?;
            }
            cooklang::Content::Text(text) => {
                if text.trim() != "-" {
                    writeln!(w)?;
                    writeln!(w, r"\textit{{Note: {}}}", escape_latex(text.trim()))?;
                    writeln!(w)?;
                }
            }
        }
    }

    writeln!(w, r"\end{{enumerate}}")?;
    writeln!(w)?;

    Ok(())
}

fn write_step(w: &mut impl io::Write, step: &Step, recipe: &Recipe) -> Result<()> {
    write!(w, r"\item ")?;

    for item in &step.items {
        match item {
            Item::Text { value } => {
                write!(w, "{}", escape_latex(value))?;
            }
            &Item::Ingredient { index } => {
                let igr = &recipe.ingredients[index];
                write!(w, r"\ingredient{{{}}}", escape_latex(&igr.display_name()))?;
            }
            &Item::Cookware { index } => {
                let cw = &recipe.cookware[index];
                write!(w, r"\cookware{{{}}}", escape_latex(&cw.name))?;
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
                write!(w, r"\timer{{{}}}", escape_latex(&timer_text))?;
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                write!(w, r"\textbf{{{}}}", escape_latex(&q.to_string()))?;
            }
        }
    }

    writeln!(w)?;

    Ok(())
}

fn escape_latex(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\\' => r"\\".to_string(),
            '{' => r"\{".to_string(),
            '}' => r"\}".to_string(),
            '$' => r"\$".to_string(),
            '&' => r"\&".to_string(),
            '#' => r"\#".to_string(),
            '^' => r"\^{}".to_string(),
            '_' => r"\_".to_string(),
            '~' => r"\~{}".to_string(),
            '%' => r"\%".to_string(),
            '<' => r"\textless{}".to_string(),
            '>' => r"\textgreater{}".to_string(),
            '|' => r"\textbar{}".to_string(),
            '°' => r"\textdegree{}".to_string(),
            '½' => r"\textonehalf{}".to_string(),
            '¼' => r"\textonequarter{}".to_string(),
            '¾' => r"\textthreequarters{}".to_string(),
            '⅓' => r"\textfrac{1}{3}".to_string(),
            '⅔' => r"\textfrac{2}{3}".to_string(),
            '×' => r"\texttimes{}".to_string(),
            '÷' => r"\textdiv{}".to_string(),
            '–' => r"\textendash{}".to_string(),
            '—' => r"\textemdash{}".to_string(),
            '€' => r"\texteuro{}".to_string(),
            '£' => r"\textsterling{}".to_string(),
            '™' => r"\texttrademark{}".to_string(),
            '®' => r"\textregistered{}".to_string(),
            '©' => r"\textcopyright{}".to_string(),
            _ => c.to_string(),
        })
        .collect()
}
