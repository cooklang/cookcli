//! Editing a pantry file **in place**, as text rather than as a model.
//!
//! # Why this exists
//!
//! [`add`](super::add), [`remove`](super::remove) and [`update`](super::update)
//! used to parse the whole file into `cooklang`'s [`PantryConf`], change that,
//! and serialise it back over the file. Everything the model does not carry was
//! therefore lost the first time anything was added, removed or updated —
//! silently, on a file people hand-write and hand-comment. Worse, the parse and
//! the serialiser disagreed about one shape: a top-level item written with
//! attributes,
//!
//! ```toml
//! salt = { quantity = "1%kg", expire = "2027-01-01" }
//! ```
//!
//! reads back as a *section* named `salt` holding items called `quantity` and
//! `expire`, and was rewritten as one — destroying the item and inventing two
//! (<https://github.com/cooklang/cookcli/issues/429>).
//!
//! Editing the document instead means the only thing a write touches is the
//! entry asked for. Comments, blank lines, key order, the short `name = "1%kg"`
//! form, attributes `cooklang` does not model, non-string values, and items
//! this crate never looked at all survive, because nothing re-emits them.
//!
//! [`PantryConf`]: cooklang::pantry::PantryConf
//!
//! # Sections
//!
//! A pantry file's sections are TOML tables, with one exception: items written
//! above the first `[header]` belong to a section the parser calls
//! [`GENERAL`]. Here that is the document root, so `general` addresses the
//! top-level entries and every other name addresses `[that_table]`.

use crate::CoreError;
use toml_edit::{DocumentMut, InlineTable, Item, Table, Value};

/// The section name that addresses the entries above the first `[header]`.
///
/// `cooklang`'s parser collects them under this name, so it is the name a user
/// sees in `cook pantry list` and the one they would pass back to `add`.
pub(super) const GENERAL: &str = "general";

/// Parse `text` as a TOML document, keeping its formatting.
pub(super) fn parse(text: &str, path: &camino::Utf8Path) -> Result<DocumentMut, CoreError> {
    text.parse::<DocumentMut>()
        .map_err(|source| CoreError::Config {
            path: Some(path.to_owned()),
            message: source.to_string(),
        })
}

/// Whether `section` exists in `doc`.
///
/// The root always exists, so `general` always does — an empty file has a
/// `general` section with nothing in it, which is the same thing the parser
/// reports.
pub(super) fn section_exists(doc: &DocumentMut, section: &str) -> bool {
    section == GENERAL || doc.get(section).is_some_and(Item::is_table_like)
}

/// A top-level item can only be written `name = "quantity"`, so refuse an edit
/// that would need somewhere else to put an attribute.
///
/// The alternative is what the old rewrite did: drop the attribute silently.
/// The one thing that must *not* happen is writing the inline table the
/// attribute would need, because the parser reads a top-level inline table as a
/// section header — turning the item into a section and inventing one item per
/// attribute, which is the corruption this module exists to stop.
pub(super) fn check_general_attributes(
    section: &str,
    name: &str,
    attributes: &Attributes,
) -> Result<(), CoreError> {
    if section != GENERAL {
        return Ok(());
    }
    let unwritable: Vec<&str> = [
        ("bought", &attributes.bought),
        ("expire", &attributes.expire),
        ("low", &attributes.low),
    ]
    .into_iter()
    .filter(|(_, value)| value.is_some())
    .map(|(key, _)| key)
    .collect();

    if unwritable.is_empty() {
        return Ok(());
    }
    Err(CoreError::PantryEdit {
        message: format!(
            "item '{name}' is above the first section header, where only a quantity can be \
             written; move it into a section to give it {}",
            unwritable.join(", ")
        ),
    })
}

/// Rewrite a section written as an array of names — `fridge = ["milk"]` — into
/// the equivalent table, so an edit has somewhere to put a key.
///
/// Returns the names that were converted, empty when there was nothing to do.
///
/// The array is a shape the parser accepts and the writer cannot extend: there
/// is no way to give `milk` a quantity inside it. Converting keeps every item
/// rather than overwriting the array with a fresh table, which is what would
/// otherwise happen the moment anything was added to such a section.
pub(super) fn normalise_array_section(doc: &mut DocumentMut, section: &str) -> Vec<String> {
    if section == GENERAL {
        return Vec::new();
    }
    let Some(array) = doc.get(section).and_then(Item::as_array) else {
        return Vec::new();
    };

    let names: Vec<String> = array
        .iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect();
    // Anything that is not a list of names is left alone rather than guessed
    // at; the edit will fail to find its section, which is the honest answer.
    if names.len() != array.len() {
        return Vec::new();
    }

    let mut table = Table::new();
    table.set_implicit(false);
    for name in &names {
        table.insert(name, toml_edit::value(""));
    }
    doc.insert(section, Item::Table(table));
    names
}

/// Whether `section` holds an item called `name`.
pub(super) fn item_exists(doc: &DocumentMut, section: &str, name: &str) -> bool {
    match section_entries(doc, section) {
        // A root key that is a table is a *section*, not an item, so `[fridge]`
        // does not answer to `pantry remove general fridge`.
        Some(entries) if section == GENERAL => {
            entries.get(name).is_some_and(|item| !item.is_table_like())
        }
        Some(entries) => entries.get(name).is_some(),
        None => false,
    }
}

/// The table holding `section`'s items, if it is there.
fn section_entries<'a>(doc: &'a DocumentMut, section: &str) -> Option<&'a Table> {
    if section == GENERAL {
        Some(doc.as_table())
    } else {
        doc.get(section).and_then(Item::as_table)
    }
}

/// Insert `name` into `section`, creating the section if it is not there.
///
/// The caller has already established that the item does not exist; this does
/// not check again.
pub(super) fn insert(doc: &mut DocumentMut, section: &str, name: &str, attributes: &Attributes) {
    let value = attributes.to_item(section);

    if section == GENERAL {
        doc.insert(name, value);
        return;
    }

    if !doc.get(section).is_some_and(Item::is_table_like) {
        let mut table = Table::new();
        // Implicit tables are not written out at all when empty, and this one
        // is about to be given an item, so it must be explicit to appear.
        table.set_implicit(false);
        doc.insert(section, Item::Table(table));
    }
    if let Some(table) = doc.get_mut(section).and_then(Item::as_table_like_mut) {
        table.insert(name, value);
    }
}

/// Take `name` out of `section`, and the section too if that empties it.
///
/// Removing an emptied section matches what the file would say after a
/// round-trip through the parser anyway — `cooklang` drops empty sections when
/// it reads — so leaving one behind would be a difference that does not
/// survive being read back.
pub(super) fn remove(doc: &mut DocumentMut, section: &str, name: &str) {
    if section == GENERAL {
        doc.remove(name);
        return;
    }

    let emptied = match doc.get_mut(section).and_then(Item::as_table_like_mut) {
        Some(table) => {
            table.remove(name);
            table.is_empty()
        }
        None => false,
    };
    if emptied {
        doc.remove(section);
    }
}

/// Apply `attributes` to the item already at `section`/`name`.
///
/// Only the attributes that are set are written. Everything else the entry
/// carries — including keys this crate does not model — is kept, which is the
/// whole difference from rebuilding the item from a parsed model.
pub(super) fn apply(
    doc: &mut DocumentMut,
    section: &str,
    name: &str,
    attributes: &Attributes,
) -> Result<(), CoreError> {
    let existing = if section == GENERAL {
        doc.as_table_mut().get_mut(name)
    } else {
        doc.get_mut(section)
            .and_then(Item::as_table_like_mut)
            .and_then(|table| table.get_mut(name))
    };
    let Some(existing) = existing else {
        return Ok(());
    };

    // `quantity` alone against a short-form item stays short form, so
    // `salt = "1%kg"` does not sprout braces just because the amount changed.
    let stays_short = existing.as_str().is_some()
        && attributes.bought.is_none()
        && attributes.expire.is_none()
        && attributes.low.is_none();
    if stays_short {
        if let Some(quantity) = &attributes.quantity {
            *existing = toml_edit::value(quantity.as_str());
        }
        return Ok(());
    }

    let mut table = as_inline_table(existing, section, name)?;
    attributes.write_into(&mut table);
    *existing = toml_edit::value(table);
    Ok(())
}

/// The entry as an inline table, converting the short `name = "1%kg"` form and
/// keeping every key an existing table already has.
///
/// # Errors
///
/// [`CoreError::PantryEdit`] if the entry is neither a string nor a table — a
/// hand-written `salt = 3` or `salt = ["a"]`. Refusing is deliberate: guessing
/// which attribute such a value meant, and overwriting it, is how the old
/// rewrite lost data in the first place.
fn as_inline_table(item: &Item, section: &str, name: &str) -> Result<InlineTable, CoreError> {
    if let Some(quantity) = item.as_str() {
        let mut table = InlineTable::new();
        table.insert("quantity", quantity.into());
        return Ok(table);
    }
    if let Some(table) = item.as_inline_table() {
        return Ok(table.clone());
    }
    if let Some(table) = item.as_table() {
        return Ok(table.clone().into_inline_table());
    }
    Err(CoreError::PantryEdit {
        message: format!(
            "item '{name}' in section '{section}' is written as {}, which is not a quantity or a \
             set of attributes — edit it by hand rather than have this overwrite it",
            describe(item)
        ),
    })
}

/// What kind of TOML value an item is, for an error message.
fn describe(item: &Item) -> &'static str {
    match item.as_value() {
        Some(Value::Integer(_)) | Some(Value::Float(_)) => "a bare number",
        Some(Value::Boolean(_)) => "a boolean",
        Some(Value::Array(_)) => "an array",
        Some(Value::Datetime(_)) => "a date",
        _ => "an unsupported value",
    }
}

/// The attributes to write onto an item.
///
/// `None` means "leave alone" on an update and "do not write" on an insert, so
/// the same struct serves both.
#[derive(Debug, Clone, Default)]
pub(super) struct Attributes {
    pub quantity: Option<String>,
    pub bought: Option<String>,
    pub expire: Option<String>,
    pub low: Option<String>,
}

impl Attributes {
    /// Whether nothing at all is set.
    pub(super) fn is_empty(&self) -> bool {
        self.quantity.is_none()
            && self.bought.is_none()
            && self.expire.is_none()
            && self.low.is_none()
    }

    /// The value a fresh item takes.
    ///
    /// Quantity alone is written in the short `name = "1%kg"` form, which is
    /// what a hand-written pantry mostly looks like, and what the parser
    /// normalises to. A `general` item is *always* written short — see
    /// [`check_general_attributes`], which is what stops one reaching here with
    /// an attribute that would need a table.
    fn to_item(&self, section: &str) -> Item {
        let short = section == GENERAL
            || (self.bought.is_none() && self.expire.is_none() && self.low.is_none());
        if short {
            return toml_edit::value(self.quantity.clone().unwrap_or_default());
        }
        let mut table = InlineTable::new();
        self.write_into(&mut table);
        toml_edit::value(table)
    }

    /// Set each attribute that is `Some`, leaving the rest of `table` alone.
    fn write_into(&self, table: &mut InlineTable) {
        for (key, value) in [
            ("quantity", &self.quantity),
            ("bought", &self.bought),
            ("expire", &self.expire),
            ("low", &self.low),
        ] {
            if let Some(value) = value {
                table.insert(key, value.as_str().into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> DocumentMut {
        parse(text, camino::Utf8Path::new("pantry.conf")).expect("parses")
    }

    fn quantity(value: &str) -> Attributes {
        Attributes {
            quantity: Some(value.to_string()),
            ..Default::default()
        }
    }

    /// The defect that started this: a top-level item with attributes is a
    /// *section* to the parser, and rebuilding the file from the parse turned
    /// it into one. Editing in place never looks at it.
    #[test]
    fn a_top_level_item_with_attributes_is_untouched_by_an_unrelated_edit() {
        let mut d = doc("# my pantry notes\n\
             salt = { quantity = \"1%kg\", expire = \"2027-01-01\" }\n\
             \n\
             [fridge]\n\
             milk = { quantity = \"1%l\", shelf = \"top\" }\n");
        insert(&mut d, "fridge", "butter", &quantity("200%g"));

        let out = d.to_string();
        assert!(
            out.contains("salt = { quantity = \"1%kg\", expire = \"2027-01-01\" }"),
            "the top-level item must survive verbatim: {out}"
        );
        assert!(out.contains("# my pantry notes"), "comment lost: {out}");
        assert!(
            out.contains("shelf = \"top\""),
            "unmodelled attribute lost: {out}"
        );
        assert!(out.contains("butter"), "the edit did not happen: {out}");
        assert!(!out.contains("[salt]"), "salt became a section: {out}");
    }

    #[test]
    fn comments_and_blank_lines_survive_a_removal() {
        let mut d = doc("# top\n\
             \n\
             [fridge]\n\
             # keep me\n\
             milk = \"1%l\"\n\
             butter = \"200%g\"\n");
        remove(&mut d, "fridge", "butter");

        let out = d.to_string();
        assert!(out.contains("# top"), "{out}");
        assert!(out.contains("# keep me"), "{out}");
        assert!(out.contains("milk = \"1%l\""), "{out}");
        assert!(!out.contains("butter"), "{out}");
    }

    #[test]
    fn removing_the_last_item_removes_the_section() {
        let mut d = doc("[fridge]\nmilk = \"1%l\"\n\n[larder]\nrice = \"1%kg\"\n");
        remove(&mut d, "fridge", "milk");

        let out = d.to_string();
        assert!(!out.contains("[fridge]"), "{out}");
        assert!(out.contains("[larder]"), "{out}");
    }

    /// An update writes the attribute asked for and leaves every other key
    /// alone, including one this crate has no idea about.
    #[test]
    fn an_update_keeps_attributes_it_does_not_know() {
        let mut d = doc("[fridge]\nmilk = { quantity = \"1%l\", shelf = \"top\" }\n");
        apply(&mut d, "fridge", "milk", &quantity("2%l")).expect("applies");

        let out = d.to_string();
        assert!(out.contains("quantity = \"2%l\""), "{out}");
        assert!(out.contains("shelf = \"top\""), "{out}");
    }

    /// Changing only the quantity of a short-form item leaves it short form,
    /// rather than expanding it to `{ quantity = "..." }`.
    #[test]
    fn a_short_form_item_stays_short_when_only_the_quantity_changes() {
        let mut d = doc("[fridge]\nmilk = \"1%l\"\n");
        apply(&mut d, "fridge", "milk", &quantity("2%l")).expect("applies");
        assert!(d.to_string().contains("milk = \"2%l\""), "{d}");
    }

    /// ...and grows into one only when an attribute needs somewhere to live.
    #[test]
    fn a_short_form_item_grows_attributes_when_one_is_set() {
        let mut d = doc("[fridge]\nmilk = \"1%l\"\n");
        apply(
            &mut d,
            "fridge",
            "milk",
            &Attributes {
                expire: Some("2027-01-01".to_string()),
                ..Default::default()
            },
        )
        .expect("applies");

        let out = d.to_string();
        assert!(out.contains("quantity = \"1%l\""), "quantity kept: {out}");
        assert!(out.contains("expire = \"2027-01-01\""), "{out}");
    }

    /// A value that is neither a quantity nor a set of attributes is refused
    /// rather than guessed at. Overwriting it is what lost data before.
    #[test]
    fn an_update_refuses_a_value_it_cannot_interpret() {
        let mut d = doc("[fridge]\nmilk = 3\n");
        let error = apply(&mut d, "fridge", "milk", &quantity("2%l")).expect_err("refuses");
        assert!(
            error.to_string().contains("bare number"),
            "the message should say what it found: {error}"
        );
        assert!(d.to_string().contains("milk = 3"), "unchanged: {d}");
    }

    /// A new item in `general` goes above the first section header, and always
    /// in short form — the parser reads a top-level inline table as a section.
    #[test]
    fn a_general_item_is_written_short_form_at_the_top() {
        let mut d = doc("[fridge]\nmilk = \"1%l\"\n");
        insert(
            &mut d,
            GENERAL,
            "salt",
            &Attributes {
                quantity: Some("1%kg".to_string()),
                expire: Some("2027-01-01".to_string()),
                ..Default::default()
            },
        );

        let out = d.to_string();
        assert!(out.contains("salt = \"1%kg\""), "{out}");
        assert!(
            out.find("salt").unwrap() < out.find("[fridge]").unwrap(),
            "a top-level item must come before the first header: {out}"
        );
        // Re-reading must see an item, not a section.
        let reparsed = cooklang::pantry::parse_lenient(&out);
        let conf = reparsed.output().expect("parses");
        assert!(conf.sections.contains_key(GENERAL), "{out}");
        assert!(!conf.sections.contains_key("salt"), "{out}");
    }

    #[test]
    fn a_new_section_is_created_for_a_new_item() {
        let mut d = doc("[fridge]\nmilk = \"1%l\"\n");
        insert(&mut d, "larder", "rice", &quantity("1%kg"));

        let out = d.to_string();
        assert!(out.contains("[larder]"), "{out}");
        assert!(out.contains("rice = \"1%kg\""), "{out}");
    }

    #[test]
    fn a_section_header_is_not_an_item_of_general() {
        let d = doc("salt = \"1%kg\"\n\n[fridge]\nmilk = \"1%l\"\n");
        assert!(item_exists(&d, GENERAL, "salt"));
        assert!(
            !item_exists(&d, GENERAL, "fridge"),
            "a table is a section, not a top-level item"
        );
        assert!(item_exists(&d, "fridge", "milk"));
    }

    /// An item with attributes and no quantity is written with attributes; one
    /// with nothing at all is written short with an empty quantity, which is
    /// what the parser normalises to.
    #[test]
    fn an_item_with_no_attributes_is_written_with_an_empty_quantity() {
        let mut d = doc("[fridge]\n");
        insert(&mut d, "fridge", "milk", &Attributes::default());
        assert!(d.to_string().contains("milk = \"\""), "{d}");
    }
}
