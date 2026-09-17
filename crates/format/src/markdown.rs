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

//! Format a recipe as markdown

use crate::quantity::grouped_quantity_fmt;
use std::{fmt::Write, io};

use cooklang::{
    convert::Converter,
    metadata::Metadata,
    model::{Item, Section, Step},
    Recipe,
};
use serde::{Deserialize, Serialize};

/// Options for [`print_md_with_options`]
///
/// This implements [`Serialize`] and [`Deserialize`], so you can embed it in
/// other configuration.
///
/// Crate-private for now: every toggle here is untested and only
/// [`print_md`] uses it, with the defaults. Publishing it would pin both the
/// API and the serde wire format before anything exercises either.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
#[non_exhaustive]
pub(crate) struct Options {
    /// Show the tags in the markdown body
    ///
    /// They will apear just after the title.
    ///
    /// The tags will have the following format:
    /// ```md
    /// #tag1 #tag2 #tag3
    /// ```
    pub(crate) tags: bool,
    /// Set the description style in the markdown body
    ///
    /// It will appear just after the tags (if its enabled and
    /// there are any tags; if not, after the title).
    #[serde(deserialize_with = "des_or_bool")]
    pub(crate) description: DescriptionStyle,
    /// Make every step a regular paragraph
    ///
    /// A `cooklang` extensions allows to add paragraphs between steps. Because
    /// some `Markdown` parser may not be able to set the start number of the
    /// list, step numbers may be wrong. With this option enabled, all steps are
    /// paragraphs because the number is escaped like:
    /// ```md
    /// 1\. Step.
    /// ```
    pub(crate) escape_step_numbers: bool,
    /// Display amounts in italics
    ///
    /// This will affect the ingredients list, cookware list and inline
    /// quantities such as temperature.
    pub(crate) italic_amounts: bool,
    /// Add the name of the recipe to the front-matter
    ///
    /// A key `name` in the metadata has preference over this.
    #[serde(deserialize_with = "des_or_bool")]
    pub(crate) front_matter_name: FrontMatterName,
    /// Text to write in headings
    pub(crate) heading: Headings,
    /// Text to write when an ingredient or cookware item is optional
    pub(crate) optional_marker: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            tags: true,
            description: DescriptionStyle::Blockquote,
            escape_step_numbers: false,
            italic_amounts: true,
            front_matter_name: FrontMatterName::default(),
            heading: Headings::default(),
            optional_marker: "(optional)".to_string(),
        }
    }
}

/// Where, if anywhere, the recipe description appears in the body
///
/// Deserializes from a bool too: `true` is the default style, `false` is
/// [`Hidden`](DescriptionStyle::Hidden).
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub(crate) enum DescriptionStyle {
    /// Do not show the description in the body
    Hidden,
    /// Show as a blockquote
    #[default]
    #[serde(alias = "default")]
    Blockquote,
    /// Show as a heading
    Heading,
}

impl From<bool> for DescriptionStyle {
    fn from(value: bool) -> Self {
        match value {
            true => Self::default(),
            false => Self::Hidden,
        }
    }
}

/// The front-matter key the recipe name is written under, if any
///
/// Deserializes from a bool too: `true` is `name`, `false` writes no key.
/// Left constructible (no `#[non_exhaustive]`) because callers configure it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
pub(crate) struct FrontMatterName(
    /// The key, or `None` to leave the name out of the front-matter.
    pub(crate) Option<String>,
);

impl Default for FrontMatterName {
    fn default() -> Self {
        Self(Some("name".to_string()))
    }
}

impl From<bool> for FrontMatterName {
    fn from(value: bool) -> Self {
        match value {
            true => Self::default(),
            false => Self(None),
        }
    }
}

/// The text used for each generated heading
///
/// Left constructible (no `#[non_exhaustive]`) because callers configure it;
/// `#[serde(default)]` fills in the headings a config file leaves out.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub(crate) struct Headings {
    /// Heading for steps sections without name
    ///
    /// If found, `%n` is replaced by the section number.
    pub(crate) section: String,
    /// Ingredients section
    pub(crate) ingredients: String,
    /// Cookware section
    pub(crate) cookware: String,
    /// Steps section
    pub(crate) steps: String,
    /// Description section
    ///
    /// The description is only shown in a section if enabled.
    pub(crate) description: String,
}

impl Default for Headings {
    fn default() -> Self {
        Self {
            section: "Section %n".into(),
            ingredients: "Ingredients".into(),
            cookware: "Cookware".into(),
            steps: "Steps".into(),
            description: "Description".into(),
        }
    }
}

fn des_or_bool<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + From<bool>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Wrapper<T> {
        Bool(bool),
        Thing(T),
    }

    let v = match Wrapper::deserialize(deserializer)? {
        Wrapper::Bool(v) => T::from(v),
        Wrapper::Thing(val) => val,
    };
    Ok(v)
}

/// Writes a recipe in Markdown format
///
/// This is an alias for `print_md_with_options` where the options are the
/// default value.
pub fn print_md(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
    writer: impl io::Write,
) -> io::Result<()> {
    print_md_with_options(recipe, name, scale, &Options::default(), converter, writer)
}

/// Writes a recipe in Markdown format
///
/// The metadata of the recipe will be in a YAML front-matter. Some special keys
/// like `autor` or `servings` will be mappings or sequences instead of text if
/// they were parsed correctly.
///
/// The [`Options`] are used to further customize the output. See it's
/// documentation to know about them.
pub(crate) fn print_md_with_options(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    opts: &Options,
    converter: &Converter,
    mut writer: impl io::Write,
) -> io::Result<()> {
    frontmatter(&mut writer, &recipe.metadata, name, opts)?;

    writeln!(
        writer,
        "# {}{}\n",
        name,
        if scale != 1.0 {
            format!(" @ {scale}")
        } else {
            "".to_string()
        }
    )?;

    if opts.tags {
        if let Some(tags) = recipe.metadata.tags() {
            for (i, tag) in tags.iter().enumerate() {
                write!(writer, "#{tag}")?;
                if i < tags.len() - 1 {
                    write!(writer, " ")?;
                }
            }
            writeln!(writer, "\n")?;
        }
    }

    if let Some(desc) = recipe.metadata.description() {
        match opts.description {
            DescriptionStyle::Hidden => {}
            DescriptionStyle::Blockquote => {
                write_block(&mut writer, desc, "> ", "> ")?;
                writeln!(writer)?;
            }
            DescriptionStyle::Heading => {
                writeln!(writer, "## {}\n", opts.heading.description)?;
                write_block(&mut writer, desc, "", "")?;
                writeln!(writer)?;
            }
        }
    }

    ingredients(&mut writer, recipe, converter, opts)?;
    cookware(&mut writer, recipe, opts, converter)?;
    sections(&mut writer, recipe, opts)?;

    Ok(())
}

fn frontmatter(
    mut w: impl io::Write,
    metadata: &Metadata,
    name: &str,
    opts: &Options,
) -> io::Result<()> {
    if metadata.map.is_empty() {
        return Ok(());
    }

    let mut map = metadata.map.clone();

    if let Some(name_key) = &opts.front_matter_name.0 {
        // add name, will be overrided if other given
        map.insert(name_key.as_str().into(), name.into());
    }

    const FRONTMATTER_FENCE: &str = "---";
    writeln!(w, "{FRONTMATTER_FENCE}")?;
    serde_yaml::to_writer(&mut w, &map)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(w, "{FRONTMATTER_FENCE}\n")?;
    Ok(())
}

fn ingredients(
    w: &mut impl io::Write,
    recipe: &Recipe,
    converter: &Converter,
    opts: &Options,
) -> io::Result<()> {
    if recipe.ingredients.is_empty() {
        return Ok(());
    }

    writeln!(w, "## {}\n", opts.heading.ingredients)?;

    for entry in recipe.group_ingredients(converter) {
        let ingredient = entry.ingredient;

        if !ingredient.modifiers().should_be_listed() {
            continue;
        }

        write!(w, "- ")?;
        if !entry.quantity.is_empty() {
            let quantity = grouped_quantity_fmt(&entry.quantity);
            if opts.italic_amounts {
                write!(w, "*{quantity}* ")?;
            } else {
                write!(w, "{quantity} ")?;
            }
        }

        if let Some(reference) = &ingredient.reference {
            let sep = crate::REFERENCE_SEPARATOR;
            let path = reference.components.join(sep);
            write!(
                w,
                "[{}]({}{}{})",
                ingredient.display_name(),
                path,
                sep,
                ingredient.name
            )?;
        } else {
            write!(w, "{}", ingredient.display_name())?;
        }

        if ingredient.modifiers().is_optional() {
            write!(w, " {}", opts.optional_marker)?;
        }

        if let Some(note) = &ingredient.note {
            write!(w, " ({note})")?;
        }
        writeln!(w)?;
    }
    writeln!(w)?;

    Ok(())
}

fn cookware(
    w: &mut impl io::Write,
    recipe: &Recipe,
    opts: &Options,
    converter: &Converter,
) -> io::Result<()> {
    if recipe.cookware.is_empty() {
        return Ok(());
    }

    writeln!(w, "## {}\n", opts.heading.cookware)?;
    for item in recipe.group_cookware(converter) {
        let cw = item.cookware;
        write!(w, "- ")?;
        if !item.quantity.is_empty() {
            let quantity = grouped_quantity_fmt(&item.quantity);
            if opts.italic_amounts {
                write!(w, "*{quantity}* ")?;
            } else {
                write!(w, "{quantity} ")?;
            }
        }
        write!(w, "{}", cw.display_name())?;

        if cw.modifiers().is_optional() {
            write!(w, " {}", opts.optional_marker)?;
        }

        if let Some(note) = &cw.note {
            write!(w, " ({note})")?;
        }
        writeln!(w)?;
    }

    writeln!(w)?;
    Ok(())
}

fn sections(w: &mut impl io::Write, recipe: &Recipe, opts: &Options) -> io::Result<()> {
    writeln!(w, "## {}\n", opts.heading.steps)?;
    for (idx, section) in recipe.sections.iter().enumerate() {
        w_section(w, section, recipe, idx + 1, opts)?;
    }
    Ok(())
}

fn w_section(
    w: &mut impl io::Write,
    section: &Section,
    recipe: &Recipe,
    num: usize,
    opts: &Options,
) -> io::Result<()> {
    if section.name.is_some() || recipe.sections.len() > 1 {
        if let Some(name) = &section.name {
            writeln!(w, "### {name}\n")?;
        } else {
            let s = opts.heading.section.replace("%n", &num.to_string());
            writeln!(w, "### {s}\n")?;
        }
    }
    for content in &section.content {
        match content {
            cooklang::Content::Step(step) => w_step(w, step, recipe, opts)?,
            cooklang::Content::Text(text) => {
                // Check if this is a list bullet item
                if text.trim() == "-" {
                    // Add extra newline for list separation
                    writeln!(w)?
                } else {
                    // Format as a note with blockquote style
                    writeln!(w, "> **Note:** {}", text.trim())?
                }
            }
        };
        writeln!(w)?;
    }
    Ok(())
}

fn w_step(w: &mut impl io::Write, step: &Step, recipe: &Recipe, opts: &Options) -> io::Result<()> {
    let mut marker = step.number.to_string();
    if opts.escape_step_numbers {
        marker.push_str("\\. ")
    } else {
        marker.push_str(". ")
    }

    let mut step_str = String::new();
    for item in &step.items {
        match item {
            Item::Text { value } => {
                // Check if this is a list bullet and format it properly for markdown
                if value.trim() == "-" {
                    step_str.push_str("\n- ");
                } else {
                    step_str.push_str(value);
                }
            }
            &Item::Ingredient { index } => {
                let igr = &recipe.ingredients[index];
                step_str.push_str(igr.display_name().as_ref());
            }
            &Item::Cookware { index } => {
                let cw = &recipe.cookware[index];
                step_str.push_str(&cw.name);
            }
            &Item::Timer { index } => {
                let t = &recipe.timers[index];
                if let Some(name) = &t.name {
                    write!(&mut step_str, "({name})").expect("writing to a String is infallible");
                }
                if let Some(quantity) = &t.quantity {
                    write!(&mut step_str, "{quantity}").expect("writing to a String is infallible");
                }
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                if opts.italic_amounts {
                    write!(&mut step_str, "*{q}*").expect("writing to a String is infallible");
                } else {
                    write!(&mut step_str, "{q}").expect("writing to a String is infallible");
                }
            }
        }
    }
    // A list item's content starts after its marker, so a line break inside the
    // step has to be indented to that column to stay inside the item.
    let indent = " ".repeat(marker.chars().count());
    write_block(w, &step_str, &marker, &indent)?;
    Ok(())
}

/// Writes `text` with `first` before its first line and `rest` before the rest.
///
/// Nothing is re-wrapped. A Markdown document is not a terminal: hard wrapping
/// a step broke sentences at whatever column the terminal happened to be, and
/// the continuation lines then sat at column zero, which ends the ordered list
/// for a CommonMark parser (<https://github.com/cooklang/cookcli/issues/497>).
/// Only the line breaks the recipe itself has survive, indented so they stay
/// part of the block they belong to.
fn write_block(w: &mut impl io::Write, text: &str, first: &str, rest: &str) -> io::Result<()> {
    // `split` rather than `lines` so an empty `text` still writes its prefix —
    // a step with no items must keep its number.
    for (i, line) in text.trim_end_matches('\n').split('\n').enumerate() {
        let prefix = if i == 0 { first } else { rest };
        // Trailing whitespace is invisible in Markdown source but two spaces of
        // it are a hard line break, so a blank quoted line must not keep the
        // space from its prefix.
        writeln!(w, "{}", format!("{prefix}{line}").trim_end())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{parse_recipe, PARSER};

    /// The step from the issue: both it and the description run well past any
    /// terminal width, and the step puts a multi-word ingredient right where
    /// the wrapping used to break the line.
    const FIXTURE: &str = "\
---
title: Long Lines
description: A description that is comfortably longer than eighty columns so that any hard wrapping shows up as an extra line.
---

Place the base on a lightly floured surface and spread @San Marzano tomato \
sauce{5%tbsp} on it. Add some fresh @basil leaves{} and fresh \
@mozzarella cheese{100%grams}.
";

    fn render(text: &str) -> String {
        let recipe = parse_recipe(text, "Long Lines", 1.0)
            .expect("the fixture parses")
            .value;
        let mut buf = Vec::new();
        print_md(&recipe, "Long Lines", 1.0, PARSER.converter(), &mut buf).expect("formats");
        String::from_utf8(buf).expect("utf8")
    }

    /// <https://github.com/cooklang/cookcli/issues/497>: the step was hard
    /// wrapped at the terminal width, putting a line break in the middle of a
    /// sentence — and, worse, making the output depend on the terminal.
    #[test]
    fn step_text_is_not_hard_wrapped() {
        let md = render(FIXTURE);

        let step = md
            .lines()
            .find(|l| l.starts_with("1. "))
            .expect("the step is numbered");
        assert_eq!(
            step,
            "1. Place the base on a lightly floured surface and spread San Marzano tomato \
sauce on it. Add some fresh basil leaves and fresh mozzarella cheese.",
            "the step should be one line, in:\n{md}"
        );
    }

    #[test]
    fn description_is_not_hard_wrapped() {
        let md = render(FIXTURE);

        let quoted: Vec<_> = md.lines().filter(|l| l.starts_with("> ")).collect();
        assert_eq!(
            quoted,
            [
                "> A description that is comfortably longer than eighty columns so that any hard \
wrapping shows up as an extra line."
            ],
            "the description should be one blockquote line, in:\n{md}"
        );
    }

    /// A line break the source *does* have stays, but it has to be indented to
    /// the step's content column or CommonMark ends the ordered list at it.
    #[test]
    fn a_list_inside_a_step_is_indented_under_the_number() {
        let md = render("Gather the following:\n- @plain flour{200%g}\n- @whole milk{250%ml}\n");

        assert!(
            md.contains("1. Gather the following: - plain flour\n   - whole milk\n"),
            "the bullet should be indented under the step, in:\n{md}"
        );
    }
}
