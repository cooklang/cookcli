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

use crate::number::format_quantity_with_unit;
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
        escape_md(name),
        if scale != 1.0 {
            format!(" @ {scale}")
        } else {
            "".to_string()
        }
    )?;

    if opts.tags {
        if let Some(tags) = recipe.metadata.tags() {
            for (i, tag) in tags.iter().enumerate() {
                write!(writer, "#{}", escape_md(tag))?;
                if i < tags.len() - 1 {
                    write!(writer, " ")?;
                }
            }
            writeln!(writer, "\n")?;
        }
    }

    if let Some(desc) = recipe.metadata.description() {
        let desc = escape_md(desc);
        match opts.description {
            DescriptionStyle::Hidden => {}
            DescriptionStyle::Blockquote => {
                write_block(&mut writer, &desc, "> ", "> ")?;
                writeln!(writer)?;
            }
            DescriptionStyle::Heading => {
                writeln!(writer, "## {}\n", opts.heading.description)?;
                write_block(&mut writer, &desc, "", "")?;
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
            let quantity = escape_md(&grouped_quantity_fmt(&entry.quantity));
            if opts.italic_amounts {
                write!(w, "*{quantity}* ")?;
            } else {
                write!(w, "{quantity} ")?;
            }
        }

        if let Some(reference) = &ingredient.reference {
            let sep = crate::REFERENCE_SEPARATOR;
            let path = reference
                .components
                .iter()
                .map(|c| escape_md_url(c))
                .collect::<Vec<_>>()
                .join(sep);
            write!(
                w,
                "[{}]({}{}{})",
                escape_md(ingredient.display_name().as_ref()),
                path,
                sep,
                escape_md_url(&ingredient.name)
            )?;
        } else {
            write!(w, "{}", escape_md(ingredient.display_name().as_ref()))?;
        }

        if ingredient.modifiers().is_optional() {
            write!(w, " {}", opts.optional_marker)?;
        }

        if let Some(note) = &ingredient.note {
            write!(w, " ({})", escape_md(note))?;
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
            let quantity = escape_md(&grouped_quantity_fmt(&item.quantity));
            if opts.italic_amounts {
                write!(w, "*{quantity}* ")?;
            } else {
                write!(w, "{quantity} ")?;
            }
        }
        write!(w, "{}", escape_md(cw.display_name()))?;

        if cw.modifiers().is_optional() {
            write!(w, " {}", opts.optional_marker)?;
        }

        if let Some(note) = &cw.note {
            write!(w, " ({})", escape_md(note))?;
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
            writeln!(w, "### {}\n", escape_md(name))?;
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
                    // Format as a note with blockquote style — the text
                    // follows `**Note:**`, so it is mid-line.
                    writeln!(w, "> **Note:** {}", escape_md_at(text.trim(), false))?
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
    // Line-start rules apply only to text that actually opens a line: an
    // item mid-step can hold `-` or `1.` without forming a list.
    let at_line_start = |step_str: &String| step_str.is_empty() || step_str.ends_with('\n');
    for item in &step.items {
        let line_start = at_line_start(&step_str);
        match item {
            Item::Text { value } => {
                // Check if this is a list bullet and format it properly for markdown
                if value.trim() == "-" {
                    step_str.push_str("\n- ");
                } else {
                    step_str.push_str(&escape_md_at(value, line_start));
                }
            }
            &Item::Ingredient { index } => {
                let igr = &recipe.ingredients[index];
                step_str.push_str(&escape_md_at(igr.display_name().as_ref(), line_start));
            }
            &Item::Cookware { index } => {
                let cw = &recipe.cookware[index];
                step_str.push_str(&escape_md_at(&cw.name, line_start));
            }
            &Item::Timer { index } => {
                let t = &recipe.timers[index];
                if let Some(name) = &t.name {
                    write!(&mut step_str, "({})", escape_md_at(name, line_start))
                        .expect("writing to a String is infallible");
                }
                if let Some(quantity) = &t.quantity {
                    write!(
                        &mut step_str,
                        "{}",
                        escape_md_at(&format_quantity_with_unit(quantity), line_start)
                    )
                    .expect("writing to a String is infallible");
                }
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                if opts.italic_amounts {
                    write!(
                        &mut step_str,
                        "*{}*",
                        escape_md_at(&format_quantity_with_unit(q), line_start)
                    )
                    .expect("writing to a String is infallible");
                } else {
                    write!(
                        &mut step_str,
                        "{}",
                        escape_md_at(&format_quantity_with_unit(q), line_start)
                    )
                    .expect("writing to a String is infallible");
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

/// Escape Markdown in user text — inline and at line starts.
///
/// Ingredient and cookware names, step text, notes, tags and metadata are
/// recipe content, not markup: a name like `*flour*` or `<b>` must render
/// literally rather than becoming emphasis or raw HTML. The escape set is
/// the characters that can still form markup — `` \ ` * [ ] < > # ~ `` —
/// plus `_` at a word boundary only: intra-word `_` cannot emphasise, so
/// `a_b_c` in a URL or identifier stays readable. `(`, `)`, `!`,
/// `{`, `}` and `|` get no escape — once the brackets are escaped no link
/// or image can form. `&` is escaped only when a complete named entity
/// follows (`&copy;`), which would otherwise decode to `©`; a bare `&` in
/// `salt & pepper` cannot form markup and is left alone.
///
/// Per line, the first character of a `-`/`+`/`=` line, the `.`/`)` of a
/// `1.`/`1)` marker, and the first `|`/`:` of a GFM delimiter row
/// (`|---|---|`, `:-- | --:`) are escaped too, so a description line that
/// happens to look like a list item, a rule, a setext heading or a table
/// stays text — a header row alone cannot make a table without one.
pub fn escape_md(text: &str) -> String {
    escape_md_at(text, true)
}

/// [`escape_md`], but `at_line_start` says whether the first line opens a
/// line — step text between components does not, and escaping what only
/// matters at a line start there would just add noise.
fn escape_md_at(text: &str, at_line_start: bool) -> String {
    text.split('\n')
        .enumerate()
        .map(|(i, line)| escape_md_line(line, i > 0 || at_line_start))
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_md_line(line: &str, at_line_start: bool) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '\\' | '`' | '*' | '[' | ']' | '<' | '>' | '#' | '~' => out.extend(['\\', c]),
            // `&copy;` decodes to `©` even where the brackets are escaped;
            // numeric entities (`&#35;`) are already safe — the `#` is
            // escaped. Escape `&` only when a complete named entity
            // follows, so `salt & pepper` stays readable.
            '&' => {
                let mut end = i + 1;
                while chars.get(end).is_some_and(|c| c.is_ascii_alphanumeric()) {
                    end += 1;
                }
                let named_entity = end > i + 1
                    && chars[i + 1].is_ascii_alphabetic()
                    && chars.get(end) == Some(&';');
                if named_entity {
                    out.extend(['\\', c]);
                } else {
                    out.push(c);
                }
            }
            '_' => {
                let intra_word = i > 0
                    && chars[i - 1].is_alphanumeric()
                    && chars.get(i + 1).is_some_and(|c| c.is_alphanumeric());
                if intra_word {
                    out.push(c);
                } else {
                    out.extend(['\\', c]);
                }
            }
            _ => out.push(c),
        }
    }
    // What the line would open left alone: a `-`/`+` bullet or `---`/`===`
    // rule or setext heading, or a `1.`/`1)` ordered list, or a GFM table
    // delimiter row — a line of only `|`, `-`, `:` and space, which needs a
    // header row to do anything, so escaping its first character is enough
    // to keep the row plain text. Each marker is ASCII punctuation, so a
    // backslash prints it literally.
    if at_line_start {
        let body = line.trim_start_matches([' ', '\t']);
        let at = line.len() - body.len();
        if body.starts_with(['-', '+', '='])
            || body
                .bytes()
                .all(|b| matches!(b, b'|' | b'-' | b':' | b' ' | b'\t'))
                && body.contains(['|', ':'])
        {
            out.insert(at, '\\');
        } else if let Some(marker) = ordered_list_delimiter(body) {
            out.insert(at + marker, '\\');
        }
    }
    out
}

/// The byte index of the `.` or `)` that ends a `1.`/`1)` ordered-list
/// marker at the start of `line`, if it starts with one.
fn ordered_list_delimiter(line: &str) -> Option<usize> {
    let digits = line.bytes().take_while(|b| b.is_ascii_digit()).count();
    match line.as_bytes()[digits..] {
        [b'.' | b')'] | [b'.' | b')', b' ' | b'\t', ..] if digits > 0 => Some(digits),
        _ => None,
    }
}

/// Percent-encode a path segment for a Markdown `(...)` link destination.
///
/// A bare `)` or space ends the destination, a `#` or `?` would start a
/// fragment or query, and `%` must be encoded first, so a reference like
/// `we]ird (sauce)` or `#1 Burger` produces a usable link.
fn escape_md_url(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '%' => "%25".to_string(),
            '(' => "%28".to_string(),
            ')' => "%29".to_string(),
            ' ' => "%20".to_string(),
            '\\' => "%5C".to_string(),
            '#' => "%23".to_string(),
            '?' => "%3F".to_string(),
            _ => c.to_string(),
        })
        .collect()
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

    /// Recipe content is text, not markup: emphasis markers, HTML and link
    /// syntax must come out escaped so they render literally.
    #[test]
    fn user_text_is_escaped() {
        let out = render("Add @flour{2%cups} and beat **hard** <i>gently</i>.\n");
        assert!(out.contains(r"\*\*hard\*\*"), "{out}");
        assert!(out.contains(r"\<i\>gently\</i\>"), "{out}");
    }

    #[test]
    fn escapes_cover_the_markup_characters() {
        assert_eq!(escape_md("*flour* [x] <b>"), r"\*flour\* \[x\] \<b\>");
        // `(` `)` `!` `{` `}` and a `|` mid-line cannot form markup on
        // their own — brackets are escaped, and a pipe row is only a table
        // header next to a delimiter row. A bare `&` likewise.
        assert_eq!(
            escape_md("dish (serves 4)! salt & pepper ~1 tsp"),
            "dish (serves 4)! salt & pepper \\~1 tsp"
        );
        // `_` emphasises only at a word boundary: `a_b_c` stays readable,
        // `_x` is escaped.
        assert_eq!(
            escape_md("https://x/a_b_c and _x_"),
            "https://x/a_b_c and \\_x\\_"
        );
    }

    /// A named entity (`&copy;`, `&amp;`) decodes even inside otherwise
    /// escaped text, so `&` is escaped when — and only when — a complete
    /// `&name;` follows it.
    #[test]
    fn named_entities_do_not_decode_user_text() {
        assert_eq!(escape_md("&copy; &amp; &x1z;"), r"\&copy; \&amp; \&x1z;");
        // Incomplete patterns and a bare `&` are untouched.
        assert_eq!(
            escape_md("salt & pepper &copy &; 1 & 2"),
            "salt & pepper &copy &; 1 & 2"
        );
        // A numeric entity is already safe: the `#` is escaped.
        assert_eq!(escape_md("&#169;"), "&\\#169;");
    }

    /// A description or note line that looks like a list item, a rule or
    /// a setext heading must stay text (#721).
    #[test]
    fn escapes_cover_line_starts() {
        assert_eq!(
            escape_md("1. first\n2) then\n- item\n+ plus\n---\n===\nx. not a list"),
            "1\\. first\n2\\) then\n\\- item\n\\+ plus\n\\---\n\\===\nx. not a list"
        );
        // `#`, `>` and `~` at a line start are already caught inline.
        assert_eq!(escape_md("# heading\n> quote"), "\\# heading\n\\> quote");
    }

    /// A GFM delimiter row in a multi-line description makes the row above
    /// it a table header in GFM renderers, and swallows what follows. Its
    /// first `|` or `:` is escaped; a header row on its own is harmless, so
    /// its pipes stay plain (#721).
    #[test]
    fn a_delimiter_row_cannot_start_a_table() {
        assert_eq!(
            escape_md("Intro\n| a | b |\n|---|---|\n| 1 | 2 |\nafter"),
            "Intro\n| a | b |\n\\|---|---|\n| 1 | 2 |\nafter"
        );
        // The row need not start with a pipe.
        assert_eq!(escape_md("a | b\n:-- | --:"), "a | b\n\\:-- | --:");
        // `--` and `-` are handled by the rule/bullet escape as before.
        assert_eq!(escape_md("---\n===\n--"), "\\---\n\\===\n\\--");
    }

    /// Between components nothing opens a line, so `-` and `=` mid-step
    /// keep their plain spelling rather than collecting noise escapes.
    #[test]
    fn mid_step_text_gets_no_line_start_escapes() {
        let out = render("Mix @flour{1%kg} - then add @sugar{2%g} = sweet.\n");
        assert!(
            out.contains(" - then add"),
            "mid-step `-` must not be escaped: {out}"
        );
        assert!(
            out.contains(" = sweet."),
            "mid-step `=` must not be escaped: {out}"
        );
        // An item that really does open the line still is.
        let out = render("- then stir.\n");
        assert!(out.contains("\\- then stir"), "{out}");
    }

    #[test]
    fn url_encoding_covers_fragment_and_query() {
        assert_eq!(escape_md_url("we]ird (sauce)"), "we]ird%20%28sauce%29");
        assert_eq!(escape_md_url("#1 Burger?x"), "%231%20Burger%3Fx");
    }
}
