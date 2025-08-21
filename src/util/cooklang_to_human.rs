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

//! Format a recipe for humans to read
//!
//! This will always write ansi colours. Use something like
//! [`anstream`](https://docs.rs/anstream) to remove them if needed.

use std::{collections::HashMap, io, time::Duration};

use cooklang::{
    convert::Converter,
    ingredient_list::GroupedIngredient,
    metadata::CooklangValueExt,
    model::{Ingredient, Item},
    quantity::Quantity,
    Recipe, Section, Step,
};
use std::fmt::Write;
use tabular::{Row, Table};
use yansi::Paint;

mod style {
    use anstyle::Style;

    macro_rules! map_style_type {
        (Style) => {
            yansi::Style
        };
        ($other:ty) => {
            $other
        };
    }

    macro_rules! map_style_func {
        ($s:ident, $name:ident, Style) => {
            anstyle_yansi::to_yansi_style($s.$name)
        };
        ($s:ident, $name:ident, $type:ty) => {
            $s.$name
        };
    }

    macro_rules! generate_styles_struct {
        ($($v:vis $field_name:ident : $field_type:tt = $default:expr),+ $(,)?) => {
            #[derive(Debug, Clone)]
            #[non_exhaustive]
            pub struct CookStyles { $($v $field_name: $field_type),+ }

            #[derive(Debug, Clone)]
            pub(crate) struct OwoStyles { $(pub $field_name: map_style_type!($field_type)),* }

            impl From<CookStyles> for OwoStyles {
                fn from(s: CookStyles) -> OwoStyles {
                    OwoStyles {
                        $($field_name: map_style_func!(s, $field_name, $field_type)),+
                    }
                }
            }

            impl CookStyles {
                pub const fn default_styles() -> Self {
                    Self {
                        $($field_name: $default),+
                    }
                }
            }
        };
    }

    macro_rules! color {
        ($color:ident) => {
            Some(anstyle::Color::Ansi(anstyle::AnsiColor::$color))
        };
    }

    // macro magic to generate 2 struct CookStyles and OwoStyles same fields, but
    // when Style is used here, CookStyles will have anstyle::Style and OwoStyles
    // owo_colors::Style for internal use. Also, OwoStyles impl From<CookStyles>

    generate_styles_struct! {
        pub title: Style             = Style::new().fg_color(color!(White)).bg_color(color!(Magenta)).bold(),
        pub meta_key: Style          = Style::new().fg_color(color!(BrightGreen)).bold(),
        pub ingredient: Style        = Style::new().fg_color(color!(Green)),
        pub cookware: Style          = Style::new().fg_color(color!(Yellow)),
        pub timer: Style             = Style::new().fg_color(color!(Cyan)),
        pub inline_quantity: Style   = Style::new().fg_color(color!(BrightRed)),
        pub opt_marker: Style        = Style::new().fg_color(color!(BrightCyan)).italic(),
        pub reference_marker: Style   = Style::new().fg_color(color!(Blue)).italic(),
        pub section_name: Style      = Style::new().bold().underline(),
        pub step_igr_quantity: Style = Style::new().dimmed(),
    }

    static STYLE: std::sync::OnceLock<OwoStyles> = std::sync::OnceLock::new();

    #[inline]
    pub(crate) fn styles() -> &'static OwoStyles {
        STYLE.get_or_init(|| CookStyles::default_styles().into())
    }
}
use style::styles;

pub type Result<T = ()> = std::result::Result<T, io::Error>;

pub fn print_human(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
    mut writer: impl std::io::Write,
) -> Result {
    let w = &mut writer;

    header(w, recipe, name, scale)?;
    metadata(w, recipe, converter)?;
    ingredients(w, recipe, converter)?;
    cookware(w, recipe, converter)?;
    steps(w, recipe)?;

    Ok(())
}

fn header(w: &mut impl io::Write, recipe: &Recipe, name: &str, scale: f64) -> Result {
    let title_text = format!(
        " {}{}{} ",
        recipe
            .metadata
            .get("emoji")
            .and_then(|v| v.as_str())
            .map(|s| format!("{s} "))
            .unwrap_or_default(),
        name,
        if scale != 1.0 {
            format!(" @ {scale}")
        } else {
            "".to_string()
        }
    );

    writeln!(w, "{}", title_text.paint(styles().title))?;

    if let Some(tags) = recipe.metadata.tags() {
        let mut tags_str = String::new();
        for tag in tags {
            let color = tag_color(&tag);
            write!(&mut tags_str, "{} ", format!("#{tag}").paint(color)).unwrap();
        }
        print_wrapped(w, &tags_str)?;
    }
    writeln!(w)
}

fn tag_color(tag: &str) -> yansi::Color {
    let hash = tag
        .chars()
        .enumerate()
        .map(|(i, c)| c as usize * i)
        .reduce(usize::wrapping_add)
        .map(|h| (h % 7))
        .unwrap_or_default();
    match hash {
        0 => yansi::Color::Red,
        1 => yansi::Color::Blue,
        2 => yansi::Color::Cyan,
        3 => yansi::Color::Yellow,
        4 => yansi::Color::Green,
        5 => yansi::Color::Magenta,
        6 => yansi::Color::White,
        _ => unreachable!(),
    }
}

fn metadata(w: &mut impl io::Write, recipe: &Recipe, converter: &Converter) -> Result {
    if let Some(desc) = recipe.metadata.description() {
        print_wrapped_with_options(w, desc, |o| {
            o.initial_indent("\u{2502} ").subsequent_indent("\u{2502}")
        })?;
        writeln!(w)?;
    }

    let mut meta_fmt =
        |name: &str, value: &str| writeln!(w, "{}: {}", name.paint(styles().meta_key), value);
    if let Some(author) = recipe.metadata.author() {
        let text = author.name().or(author.url()).unwrap_or("-");
        meta_fmt("author", text)?;
    }
    if let Some(source) = recipe.metadata.source() {
        let text = source.name().or(source.url()).unwrap_or("-");
        meta_fmt("source", text)?;
    }
    if let Some(time) = recipe.metadata.time(converter) {
        let time_fmt = |t: u32| {
            format!(
                "{}",
                humantime::format_duration(Duration::from_secs(t as u64 * 60))
            )
        };
        match time {
            cooklang::metadata::RecipeTime::Total(t) => meta_fmt("time", &time_fmt(t))?,
            cooklang::metadata::RecipeTime::Composed {
                prep_time,
                cook_time,
            } => {
                if let Some(p) = prep_time {
                    meta_fmt("prep time", &time_fmt(p))?
                }
                if let Some(c) = cook_time {
                    meta_fmt("cook time", &time_fmt(c))?;
                }
                meta_fmt("total time", &time_fmt(time.total()))?;
            }
        }
    }

    if let Some(servings) = recipe.metadata.servings() {
        meta_fmt("servings", &servings.to_string())?;
    }

    for (key, value) in recipe.metadata.map.iter().filter_map(|(key, value)| {
        let key = key.as_str_like()?;
        match key.as_ref() {
            "name" | "title" | "description" | "tags" | "author" | "source" | "emoji" | "time"
            | "prep time" | "cook time" | "servings" => return None,
            _ => {}
        }
        let value = value.as_str_like()?;
        Some((key, value))
    }) {
        meta_fmt(&key, &value)?;
    }
    if !recipe.metadata.map.is_empty() {
        writeln!(w)?;
    }
    Ok(())
}

fn ingredients(w: &mut impl io::Write, recipe: &Recipe, converter: &Converter) -> Result {
    if recipe.ingredients.is_empty() {
        return Ok(());
    }
    writeln!(w, "Ingredients:")?;
    let mut table = Table::new("  {:<} {:<}    {:<} {:<} {:<}");
    for entry in recipe.group_ingredients(converter) {
        let GroupedIngredient {
            ingredient: igr,
            quantity,
            ..
        } = entry;

        let outcome_style = yansi::Style::new();
        let outcome_char = "";

        let mut row = Row::new().with_cell(igr.display_name());

        if igr.reference.is_some() {
            let path = igr.reference.as_ref().unwrap().components.join("/");
            row.add_ansi_cell(
                format!("(recipe: {}/{})", path, igr.name).paint(styles().reference_marker),
            );
        } else {
            row.add_cell("");
        }

        if igr.modifiers().is_optional() {
            row.add_ansi_cell("(optional)".paint(styles().opt_marker));
        } else {
            row.add_cell("");
        }

        let content = quantity
            .iter()
            .map(|q| quantity_fmt(q).paint(outcome_style).to_string())
            .reduce(|s, q| format!("{s}, {q}"))
            .unwrap_or_default();

        row.add_ansi_cell(format!("{content}{}", outcome_char.paint(outcome_style)));

        if let Some(note) = &igr.note {
            row.add_cell(format!("({note})"));
        } else {
            row.add_cell("");
        }

        table.add_row(row);
    }
    write!(w, "{table}")?;
    writeln!(w)
}

fn cookware(w: &mut impl io::Write, recipe: &Recipe, converter: &Converter) -> Result {
    if recipe.cookware.is_empty() {
        return Ok(());
    }
    writeln!(w, "Cookware:")?;
    let mut table = Table::new("  {:<} {:<}    {:<} {:<}");
    for item in recipe
        .cookware
        .iter()
        .filter(|cw| cw.modifiers().should_be_listed())
    {
        let mut row = Row::new().with_cell(item.display_name()).with_cell(
            if item.modifiers().is_optional() {
                "(optional)"
            } else {
                ""
            },
        );

        let amount = item.group_quantities(&recipe.cookware, converter);
        if amount.is_empty() {
            row.add_cell("");
        } else {
            let t = amount
                .iter()
                .map(|q| q.to_string())
                .reduce(|s, q| format!("{s}, {q}"))
                .unwrap();
            row.add_ansi_cell(t);
        }

        if let Some(note) = &item.note {
            row.add_cell(format!("({note})"));
        } else {
            row.add_cell("");
        }

        table.add_row(row);
    }
    writeln!(w, "{table}")?;
    Ok(())
}

fn steps(w: &mut impl io::Write, recipe: &Recipe) -> Result {
    writeln!(w, "Steps:")?;
    for (section_index, section) in recipe.sections.iter().enumerate() {
        if recipe.sections.len() > 1 {
            writeln!(
                w,
                "{: ^width$}",
                format!("─── § {} ───", section_index + 1),
                width = TERM_WIDTH
            )?;
        }

        if let Some(name) = &section.name {
            writeln!(w, "{}:", name.paint(styles().section_name))?;
        }

        for content in &section.content {
            match content {
                cooklang::Content::Step(step) => {
                    let (step_text, step_ingredients) = step_text(recipe, section, step);
                    let step_text = format!("{:>2}. {}", step.number, step_text.trim());
                    print_wrapped_with_options(w, &step_text, |o| o.subsequent_indent("    "))?;
                    print_wrapped_with_options(w, &step_ingredients, |o| {
                        let indent = "     "; // 5
                        o.initial_indent(indent)
                            .subsequent_indent(indent)
                            .word_separator(textwrap::WordSeparator::Custom(|s| {
                                Box::new(
                                    s.split_inclusive(", ")
                                        .map(|part| textwrap::core::Word::from(part)),
                                )
                            }))
                    })?;
                }
                cooklang::Content::Text(t) => {
                    writeln!(w)?;
                    print_wrapped_with_options(w, t.trim(), |o| o.initial_indent("  "))?;
                    writeln!(w)?;
                }
            }
        }
    }
    Ok(())
}

fn step_text(recipe: &Recipe, _section: &Section, step: &Step) -> (String, String) {
    let mut step_text = String::new();

    let step_igrs_dedup = build_step_igrs_dedup(step, recipe);

    // contains the ingredient and index (if any) in the line under
    // the step that shows the ingredients
    let mut step_igrs_line: Vec<(&Ingredient, Option<usize>)> = Vec::new();

    for item in &step.items {
        match item {
            Item::Text { value } => step_text += value,
            &Item::Ingredient { index } => {
                let igr = &recipe.ingredients[index];
                write!(
                    &mut step_text,
                    "{}",
                    igr.display_name().paint(styles().ingredient)
                )
                .unwrap();
                let pos = write_igr_count(&mut step_text, &step_igrs_dedup, index, &igr.name);
                if step_igrs_dedup[igr.name.as_str()].contains(&index) {
                    step_igrs_line.push((igr, pos));
                }
            }
            &Item::Cookware { index } => {
                let cookware = &recipe.cookware[index];
                write!(&mut step_text, "{}", cookware.name.paint(styles().cookware)).unwrap();
            }
            &Item::Timer { index } => {
                let timer = &recipe.timers[index];

                match (&timer.quantity, &timer.name) {
                    (Some(quantity), Some(name)) => {
                        let s = format!(
                            "{} ({})",
                            quantity_fmt(quantity).paint(styles().timer),
                            name.paint(styles().timer),
                        );
                        write!(&mut step_text, "{s}").unwrap();
                    }
                    (Some(quantity), None) => {
                        write!(
                            &mut step_text,
                            "{}",
                            quantity_fmt(quantity).paint(styles().timer)
                        )
                        .unwrap();
                    }
                    (None, Some(name)) => {
                        write!(&mut step_text, "{}", name.paint(styles().timer)).unwrap();
                    }
                    (None, None) => unreachable!(), // guaranteed in parsing
                }
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                write!(
                    &mut step_text,
                    "{}",
                    quantity_fmt(q).paint(styles().inline_quantity)
                )
                .unwrap()
            }
        }
    }

    // This is only for the line where ingredients are placed

    if step_igrs_line.is_empty() {
        return (step_text, "[-]".into());
    }
    let mut igrs_text = String::from("[");
    for (i, (igr, pos)) in step_igrs_line.iter().enumerate() {
        write!(&mut igrs_text, "{}", igr.display_name()).unwrap();
        if let Some(pos) = pos {
            write_subscript(&mut igrs_text, &pos.to_string());
        }
        if igr.modifiers().is_optional() {
            write!(&mut igrs_text, "{}", " (opt)".paint(styles().opt_marker)).unwrap();
        }

        if let Some(q) = &igr.quantity {
            write!(
                &mut igrs_text,
                ": {}",
                quantity_fmt(q).paint(styles().step_igr_quantity)
            )
            .unwrap();
        }
        if i != step_igrs_line.len() - 1 {
            igrs_text += ", ";
        }
    }
    igrs_text += "]";
    (step_text, igrs_text)
}

fn build_step_igrs_dedup<'a>(step: &'a Step, recipe: &'a Recipe) -> HashMap<&'a str, Vec<usize>> {
    // contain all ingredients used in the step (the names), the vec
    // contains the exact indices used
    let mut step_igrs_dedup: HashMap<&str, Vec<usize>> = HashMap::new();
    for item in &step.items {
        if let Item::Ingredient { index } = item {
            let igr = &recipe.ingredients[*index];
            step_igrs_dedup.entry(&igr.name).or_default().push(*index);
        }
    }

    // for each name only keep entries that provide information:
    // - if it has a quantity
    // - at least one if it's empty
    for group in step_igrs_dedup.values_mut() {
        let first = group.first().copied().unwrap();
        group.retain(|&i| {
            let igr = &recipe.ingredients[i];
            igr.quantity.is_some()
        });
        if group.is_empty() {
            group.push(first);
        }
    }
    step_igrs_dedup
}

fn write_igr_count(
    buffer: &mut String,
    step_igrs: &HashMap<&str, Vec<usize>>,
    index: usize,
    name: &str,
) -> Option<usize> {
    let entries = &step_igrs[name];
    if entries.len() <= 1 {
        return None;
    }
    if let Some(mut pos) = entries.iter().position(|&i| i == index) {
        pos += 1;
        write_subscript(buffer, &pos.to_string());
        Some(pos)
    } else {
        None
    }
}

fn quantity_fmt(qty: &Quantity) -> String {
    if let Some(unit) = qty.unit() {
        format!("{} {}", qty.value(), unit.italic())
    } else {
        format!("{}", qty.value())
    }
}

fn write_subscript(buffer: &mut String, s: &str) {
    buffer.reserve(s.len());
    s.chars()
        .map(|c| match c {
            '0' => '₀',
            '1' => '₁',
            '2' => '₂',
            '3' => '₃',
            '4' => '₄',
            '5' => '₅',
            '6' => '₆',
            '7' => '₇',
            '8' => '₈',
            '9' => '₉',
            _ => c,
        })
        .for_each(|c| buffer.push(c))
}

fn print_wrapped(w: &mut impl io::Write, text: &str) -> Result {
    print_wrapped_with_options(w, text, |o| o)
}

static TERM_WIDTH: std::sync::LazyLock<usize> =
    std::sync::LazyLock::new(|| textwrap::termwidth().min(80));

fn print_wrapped_with_options<F>(w: &mut impl io::Write, text: &str, f: F) -> Result
where
    F: FnOnce(textwrap::Options) -> textwrap::Options,
{
    let options = f(textwrap::Options::new(*TERM_WIDTH));
    let lines = textwrap::wrap(text, options);
    for line in lines {
        writeln!(w, "{line}")?;
    }
    Ok(())
}
