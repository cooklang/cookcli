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

//! Format a recipe as cooklang

use std::{fmt::Write, io};

use anyhow::{Context, Result};
use cooklang::{
    metadata::Metadata,
    model::{Item, Section, Step},
    parser::Modifiers,
    quantity::Quantity,
    Recipe,
};
use regex::Regex;

pub fn print_cooklang(recipe: &Recipe, mut writer: impl io::Write) -> Result<()> {
    let w = &mut writer;

    metadata(w, &recipe.metadata).context("Failed to write metadata")?;
    writeln!(w).context("Failed to write newline")?;
    sections(w, recipe).context("Failed to write sections")?;

    Ok(())
}

fn metadata(w: &mut impl io::Write, metadata: &Metadata) -> Result<()> {
    // TODO if the recipe has been scaled and multiple servings are defined
    // it can lead to the recipe not parsing.
    if metadata.map.is_empty() {
        return Ok(());
    }

    let map = metadata.map.clone();

    const FRONTMATTER_FENCE: &str = "---";
    writeln!(w, "{FRONTMATTER_FENCE}").context("Failed to write frontmatter start")?;
    serde_yaml::to_writer(&mut *w, &map).context("Failed to serialize frontmatter")?;
    writeln!(w, "{FRONTMATTER_FENCE}\n").context("Failed to write frontmatter end")?;
    Ok(())
}

fn sections(w: &mut impl io::Write, recipe: &Recipe) -> Result<()> {
    for (index, section) in recipe.sections.iter().enumerate() {
        w_section(w, section, recipe, index).context("Failed to write section")?;
    }
    Ok(())
}

fn w_section(
    w: &mut impl io::Write,
    section: &Section,
    recipe: &Recipe,
    index: usize,
) -> Result<()> {
    if let Some(name) = &section.name {
        writeln!(w, "== {name} ==").context("Failed to write section name")?;
    } else if index > 0 {
        writeln!(w, "====").context("Failed to write section separator")?;
    }
    for content in &section.content {
        match content {
            cooklang::Content::Step(step) => {
                w_step(w, step, recipe).context("Failed to write step")?
            }
            cooklang::Content::Text(text) => {
                w_text_block(w, text).context("Failed to write text block")?
            }
        }
        writeln!(w).context("Failed to write newline")?;
    }
    Ok(())
}

fn w_step(w: &mut impl io::Write, step: &Step, recipe: &Recipe) -> Result<()> {
    let mut step_str = String::new();
    for item in &step.items {
        match item {
            Item::Text { value } => step_str.push_str(value),
            &Item::Ingredient { index } => {
                let igr = &recipe.ingredients[index];

                let name = if let Some(reference) = &igr.reference {
                    let sep = std::path::MAIN_SEPARATOR.to_string();
                    format!(
                        ".{}{}{}{}",
                        sep,
                        reference.components.join(&sep),
                        sep,
                        &igr.name
                    )
                } else {
                    igr.name.clone()
                };

                ComponentFormatter {
                    kind: ComponentKind::Ingredient,
                    modifiers: igr.modifiers(),
                    name: Some(&name),
                    alias: igr.alias.as_deref(),
                    quantity: igr.quantity.as_ref(),
                    note: igr.note.as_deref(),
                }
                .format(&mut step_str)
            }
            &Item::Cookware { index } => {
                let cw = &recipe.cookware[index];
                ComponentFormatter {
                    kind: ComponentKind::Cookware,
                    modifiers: cw.modifiers(),
                    name: Some(&cw.name),
                    alias: cw.alias.as_deref(),
                    quantity: cw.quantity.as_ref(),
                    note: None,
                }
                .format(&mut step_str)
            }
            &Item::Timer { index } => {
                let t = &recipe.timers[index];
                ComponentFormatter {
                    kind: ComponentKind::Timer,
                    modifiers: Modifiers::empty(),
                    name: t.name.as_deref(),
                    alias: None,
                    quantity: t.quantity.as_ref(),
                    note: None,
                }
                .format(&mut step_str)
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                write!(&mut step_str, "{}", q.value())
                    .context("Failed to write inline quantity")?;
                if let Some(u) = q.unit() {
                    step_str.push_str(u);
                }
            }
        }
    }
    let width = textwrap::termwidth().min(80);
    let options = textwrap::Options::new(width)
        .word_separator(textwrap::WordSeparator::Custom(component_word_separator));
    let lines = textwrap::wrap(step_str.trim(), options);
    for line in lines {
        writeln!(w, "{line}").context("Failed to write step line")?;
    }
    Ok(())
}

fn w_text_block(w: &mut impl io::Write, text: &str) -> Result<()> {
    let width = textwrap::termwidth().min(80);
    let indent = "> ";
    let options = textwrap::Options::new(width)
        .initial_indent(indent)
        .subsequent_indent(indent);
    let lines = textwrap::wrap(text.trim(), options);
    for line in lines {
        writeln!(w, "{line}").context("Failed to write text block line")?;
    }
    Ok(())
}

// This prevents spliting a multi word component in two lines, because that's
// invalid.
fn component_word_separator<'a>(
    line: &'a str,
) -> Box<dyn Iterator<Item = textwrap::core::Word<'a>> + 'a> {
    use textwrap::core::Word;

    let re = {
        static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new(r"[@#~][^@#~]*\{[^\}]*\}").unwrap())
    };

    let mut words = vec![];
    let mut last_added = 0;
    let default_separator = textwrap::WordSeparator::new();

    for component in re.find_iter(line) {
        if last_added < component.start() {
            words.extend(default_separator.find_words(&line[last_added..component.start()]));
        }
        words.push(Word::from(&line[component.range()]));
        last_added = component.end();
    }
    if last_added < line.len() {
        words.extend(default_separator.find_words(&line[last_added..]));
    }
    Box::new(words.into_iter())
}

struct ComponentFormatter<'a> {
    kind: ComponentKind,
    modifiers: Modifiers,
    name: Option<&'a str>,
    alias: Option<&'a str>,
    quantity: Option<&'a Quantity>,
    note: Option<&'a str>,
}

enum ComponentKind {
    Ingredient,
    Cookware,
    Timer,
}

impl ComponentFormatter<'_> {
    fn format(self, w: &mut String) {
        w.push(match self.kind {
            ComponentKind::Ingredient => '@',
            ComponentKind::Cookware => '#',
            ComponentKind::Timer => '~',
        });
        for m in self.modifiers {
            w.push(match m {
                Modifiers::RECIPE => '@',
                Modifiers::HIDDEN => '-',
                Modifiers::OPT => '?',
                Modifiers::REF => '&',
                Modifiers::NEW => '+',
                _ => panic!("Unknown modifier: {m:?}"),
            });
        }
        let mut multi_word = false;
        if let Some(name) = self.name {
            if name.chars().any(|c| !c.is_alphanumeric()) {
                multi_word = true;
            }
            w.push_str(name);
            if let Some(alias) = self.alias {
                multi_word = true;
                w.push('|');
                w.push_str(alias);
            }
        }
        if let Some(q) = self.quantity {
            w.push('{');
            w.push_str(&q.value().to_string());
            if let Some(unit) = q.unit() {
                write!(w, "%{unit}").unwrap();
            }
            w.push('}');
        } else if multi_word {
            w.push_str("{}");
        }
        if let Some(note) = self.note {
            write!(w, "({note})").unwrap();
        }
    }
}
