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

use std::{fmt::Write, io};

use anyhow::{Context, Result};
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
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
#[non_exhaustive]
pub struct Options {
    /// Show the tags in the markdown body
    ///
    /// They will apear just after the title.
    ///
    /// The tags will have the following format:
    /// ```md
    /// #tag1 #tag2 #tag3
    /// ```
    pub tags: bool,
    /// Set the description style in the markdown body
    ///
    /// It will appear just after the tags (if its enabled and
    /// there are any tags; if not, after the title).
    #[serde(deserialize_with = "des_or_bool")]
    pub description: DescriptionStyle,
    /// Make every step a regular paragraph
    ///
    /// A `cooklang` extensions allows to add paragraphs between steps. Because
    /// some `Markdown` parser may not be able to set the start number of the
    /// list, step numbers may be wrong. With this option enabled, all steps are
    /// paragraphs because the number is escaped like:
    /// ```md
    /// 1\. Step.
    /// ```
    pub escape_step_numbers: bool,
    /// Display amounts in italics
    ///
    /// This will affect the ingredients list, cookware list and inline
    /// quantities such as temperature.
    pub italic_amounts: bool,
    /// Add the name of the recipe to the front-matter
    ///
    /// A key `name` in the metadata has preference over this.
    #[serde(deserialize_with = "des_or_bool")]
    pub front_matter_name: FrontMatterName,
    /// Text to write in headings
    pub heading: Headings,
    /// Text to write when an ingredient or cookware item is optional
    pub optional_marker: String,
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

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DescriptionStyle {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
pub struct FrontMatterName(pub Option<String>);

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Headings {
    /// Heading for steps sections without name
    ///
    /// If found, `%n` is replaced by the section number.
    pub section: String,
    /// Ingredients section
    pub ingredients: String,
    /// Cookware section
    pub cookware: String,
    /// Steps section
    pub steps: String,
    /// Description section
    ///
    /// The description is only shown in a section if enabled.
    pub description: String,
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
/// This is an alias for [`print_md_with_options`] where the options are the
/// default value.
pub fn print_md(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
    writer: impl io::Write,
) -> Result<()> {
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
pub fn print_md_with_options(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    opts: &Options,
    converter: &Converter,
    mut writer: impl io::Write,
) -> Result<()> {
    frontmatter(&mut writer, &recipe.metadata, name, opts)
        .context("Failed to write frontmatter")?;

    writeln!(
        writer,
        "# {}{}\n",
        name,
        if scale != 1.0 {
            format!(" @ {scale}")
        } else {
            "".to_string()
        }
    )
    .context("Failed to write title")?;

    if opts.tags {
        if let Some(tags) = recipe.metadata.tags() {
            for (i, tag) in tags.iter().enumerate() {
                write!(writer, "#{tag}").context("Failed to write tag")?;
                if i < tags.len() - 1 {
                    write!(writer, " ").context("Failed to write tag separator")?;
                }
            }
            writeln!(writer, "\n").context("Failed to write newline after tags")?;
        }
    }

    if let Some(desc) = recipe.metadata.description() {
        match opts.description {
            DescriptionStyle::Hidden => {}
            DescriptionStyle::Blockquote => {
                print_wrapped_with_options(&mut writer, desc, |o| {
                    o.initial_indent("> ").subsequent_indent("> ")
                })
                .context("Failed to write description as blockquote")?;
                writeln!(writer).context("Failed to write newline after description")?;
            }
            DescriptionStyle::Heading => {
                writeln!(writer, "## {}\n", opts.heading.description)
                    .context("Failed to write description heading")?;
                print_wrapped(&mut writer, desc).context("Failed to write description")?;
                writeln!(writer).context("Failed to write newline after description")?;
            }
        }
    }

    ingredients(&mut writer, recipe, converter, opts).context("Failed to write ingredients")?;
    cookware(&mut writer, recipe, opts, converter).context("Failed to write cookware")?;
    sections(&mut writer, recipe, opts).context("Failed to write sections")?;

    Ok(())
}

fn frontmatter(
    mut w: impl io::Write,
    metadata: &Metadata,
    name: &str,
    opts: &Options,
) -> Result<()> {
    if metadata.map.is_empty() {
        return Ok(());
    }

    let mut map = metadata.map.clone();

    if let Some(name_key) = &opts.front_matter_name.0 {
        // add name, will be overrided if other given
        map.insert(name_key.as_str().into(), name.into());
    }

    const FRONTMATTER_FENCE: &str = "---";
    writeln!(w, "{FRONTMATTER_FENCE}").context("Failed to write frontmatter start")?;
    serde_yaml::to_writer(&mut w, &map).context("Failed to serialize frontmatter")?;
    writeln!(w, "{FRONTMATTER_FENCE}\n").context("Failed to write frontmatter end")?;
    Ok(())
}

fn ingredients(
    w: &mut impl io::Write,
    recipe: &Recipe,
    converter: &Converter,
    opts: &Options,
) -> Result<()> {
    if recipe.ingredients.is_empty() {
        return Ok(());
    }

    writeln!(w, "## {}\n", opts.heading.ingredients)
        .context("Failed to write ingredients header")?;

    for entry in recipe.group_ingredients(converter) {
        let ingredient = entry.ingredient;

        if !ingredient.modifiers().should_be_listed() {
            continue;
        }

        write!(w, "- ").context("Failed to write ingredient bullet")?;
        if !entry.quantity.is_empty() {
            if opts.italic_amounts {
                write!(w, "*{}* ", entry.quantity)
                    .context("Failed to write italicized quantity")?;
            } else {
                write!(w, "{} ", entry.quantity).context("Failed to write quantity")?;
            }
        }

        if ingredient.reference.is_some() {
            let path = ingredient.reference.as_ref().unwrap().components.join("/");
            write!(
                w,
                "[{}]({}/{})",
                ingredient.display_name(),
                path,
                ingredient.name
            )
            .context("Failed to write reference")?;
        } else {
            write!(w, "{}", ingredient.display_name())
                .context("Failed to write ingredient name")?;
        }

        if ingredient.modifiers().is_optional() {
            write!(w, " {}", opts.optional_marker).context("Failed to write optional marker")?;
        }

        if let Some(note) = &ingredient.note {
            write!(w, " ({note})").context("Failed to write ingredient note")?;
        }
        writeln!(w).context("Failed to write newline after ingredient")?;
    }
    writeln!(w).context("Failed to write newline after ingredients")?;

    Ok(())
}

fn cookware(
    w: &mut impl io::Write,
    recipe: &Recipe,
    opts: &Options,
    converter: &Converter,
) -> Result<()> {
    if recipe.cookware.is_empty() {
        return Ok(());
    }

    writeln!(w, "## {}\n", opts.heading.cookware).context("Failed to write cookware header")?;
    for item in recipe.group_cookware(converter) {
        let cw = item.cookware;
        write!(w, "- ").context("Failed to write cookware bullet")?;
        if !item.quantity.is_empty() {
            if opts.italic_amounts {
                write!(w, "*{} * ", item.quantity).context("Failed to write italicized amount")?;
            } else {
                write!(w, "{} ", item.quantity).context("Failed to write amount")?;
            }
        }
        write!(w, "{}", cw.display_name()).context("Failed to write cookware name")?;

        if cw.modifiers().is_optional() {
            write!(w, " {}", opts.optional_marker).context("Failed to write optional marker")?;
        }

        if let Some(note) = &cw.note {
            write!(w, " ({note})").context("Failed to write cookware note")?;
        }
        writeln!(w).context("Failed to write newline after cookware")?;
    }

    writeln!(w).context("Failed to write newline after cookware list")?;
    Ok(())
}

fn sections(w: &mut impl io::Write, recipe: &Recipe, opts: &Options) -> Result<()> {
    writeln!(w, "## {}\n", opts.heading.steps).context("Failed to write steps header")?;
    for (idx, section) in recipe.sections.iter().enumerate() {
        w_section(w, section, recipe, idx + 1, opts)
            .context(format!("Failed to write section {}", idx + 1))?;
    }
    Ok(())
}

fn w_section(
    w: &mut impl io::Write,
    section: &Section,
    recipe: &Recipe,
    num: usize,
    opts: &Options,
) -> Result<()> {
    if section.name.is_some() || recipe.sections.len() > 1 {
        if let Some(name) = &section.name {
            writeln!(w, "### {name}\n").context("Failed to write section name")?;
        } else {
            let s = opts.heading.section.replace("%n", &num.to_string());
            writeln!(w, "### {s}\n").context("Failed to write section number")?;
        }
    }
    for content in &section.content {
        match content {
            cooklang::Content::Step(step) => {
                w_step(w, step, recipe, opts).context("Failed to write step")?
            }
            cooklang::Content::Text(text) => {
                print_wrapped(w, text).context("Failed to write text content")?
            }
        };
        writeln!(w).context("Failed to write newline after content")?;
    }
    Ok(())
}

fn w_step(w: &mut impl io::Write, step: &Step, recipe: &Recipe, opts: &Options) -> Result<()> {
    let mut step_str = step.number.to_string();
    if opts.escape_step_numbers {
        step_str.push_str("\\. ")
    } else {
        step_str.push_str(". ")
    }

    for item in &step.items {
        match item {
            Item::Text { value } => step_str.push_str(value),
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
                    write!(&mut step_str, "({name})").context("Failed to write timer name")?;
                }
                if let Some(quantity) = &t.quantity {
                    write!(&mut step_str, "{quantity}")
                        .context("Failed to write timer quantity")?;
                }
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                if opts.italic_amounts {
                    write!(&mut step_str, "*{q}*")
                        .context("Failed to write italicized inline quantity")?;
                } else {
                    write!(&mut step_str, "{q}").context("Failed to write inline quantity")?;
                }
            }
        }
    }
    print_wrapped(w, &step_str).context("Failed to write wrapped step")?;
    Ok(())
}

fn print_wrapped(w: &mut impl io::Write, text: &str) -> Result<()> {
    print_wrapped_with_options(w, text, |o| o)
}

static TERM_WIDTH: std::sync::LazyLock<usize> =
    std::sync::LazyLock::new(|| textwrap::termwidth().min(80));

fn print_wrapped_with_options<F>(w: &mut impl io::Write, text: &str, f: F) -> Result<()>
where
    F: FnOnce(textwrap::Options) -> textwrap::Options,
{
    let options = f(textwrap::Options::new(*TERM_WIDTH));
    let lines = textwrap::wrap(text, options);
    for line in lines {
        writeln!(w, "{line}").context("Failed to write wrapped line")?;
    }
    Ok(())
}
