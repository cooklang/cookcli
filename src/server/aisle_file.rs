//! Edits to `aisle.conf` made one line at a time.
//!
//! `cooklang::aisle::write` would rewrite the whole file from the parsed
//! configuration, dropping every comment and the blank lines people use to
//! group a long aisle. These edits touch only the lines they are about, so the
//! rest of the file stays as its owner wrote it.
//!
//! Lines are read the way `cooklang::aisle::parse` reads them: anything after
//! `//` is a comment, `[name]` starts an aisle, and any other line lists one
//! ingredient's names separated by `|`, the first being the name the shopping
//! list shows.

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Range;

/// Why an edit was refused. The file is left as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// A name the file could not hold as typed.
    InvalidName(String),
    /// The name is already used.
    Taken(String),
    /// Nothing has that name.
    NotFound(String),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::InvalidName(message)
            | EditError::Taken(message)
            | EditError::NotFound(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for EditError {}

/// A short fingerprint of the file's text. A page sends back the one it was
/// shown, so an edit made against a file someone else has changed since is
/// refused instead of applied to lines that moved.
///
/// `DefaultHasher::new` uses fixed keys, so the value is the same for the
/// same text for as long as the server runs, which is all it is compared
/// across.
pub fn revision(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Blank,
    Comment,
    Aisle(String),
    Ingredient(Vec<String>),
}

/// The line without its comment, and the comment from its `//` on.
fn split_comment(line: &str) -> (&str, Option<&str>) {
    match line.find("//") {
        Some(at) => (&line[..at], Some(&line[at..])),
        None => (line, None),
    }
}

fn kind(line: &str) -> Kind {
    let (code, comment) = split_comment(line);
    let code = code.trim_ascii();
    if code.starts_with('[') && code.ends_with(']') {
        // Not trimmed inside the brackets: neither is the parser.
        Kind::Aisle(code[1..code.len() - 1].to_string())
    } else if !code.is_empty() {
        Kind::Ingredient(
            code.split('|')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect(),
        )
    } else if comment.is_some() {
        Kind::Comment
    } else {
        Kind::Blank
    }
}

/// `line` with `code` in place of what it had before its comment, keeping
/// the indentation and the comment.
fn replace_code(line: &str, code: &str) -> String {
    let (old, comment) = split_comment(line);
    let indent = &old[..old.len() - old.trim_start().len()];
    match comment {
        Some(comment) => {
            let gap = &old[old.trim_end().len()..];
            let gap = if gap.is_empty() { " " } else { gap };
            format!("{indent}{code}{gap}{comment}")
        }
        None => format!("{indent}{code}"),
    }
}

fn clean(name: &str, what: &str) -> Result<String, EditError> {
    let name = name.trim();
    let problem = if name.is_empty() {
        "cannot be empty"
    } else if name.contains('|') {
        "cannot contain '|'"
    } else if name.contains("//") {
        "cannot contain '//'"
    } else if name.contains(['\n', '\r']) {
        "must fit on one line"
    } else {
        return Ok(name.to_string());
    };
    Err(EditError::InvalidName(format!("{what} {problem}")))
}

fn aisle_name(name: &str) -> Result<String, EditError> {
    clean(name, "An aisle name")
}

fn ingredient_names(names: &[String]) -> Result<Vec<String>, EditError> {
    let names = names
        .iter()
        .map(|name| clean(name, "An ingredient name"))
        .collect::<Result<Vec<_>, _>>()?;
    let (Some(first), Some(last)) = (names.first(), names.last()) else {
        return Err(EditError::InvalidName(
            "An ingredient needs a name".to_string(),
        ));
    };
    if first.starts_with('[') && last.ends_with(']') {
        return Err(EditError::InvalidName(format!(
            "\"{}\" would be read as an aisle",
            names.join(" | ")
        )));
    }
    Ok(names)
}

/// An `aisle.conf` held as its lines.
#[derive(Debug, Clone)]
pub struct AisleFile {
    lines: Vec<String>,
    eol: &'static str,
}

impl AisleFile {
    pub fn parse(text: &str) -> Self {
        let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
        Self {
            lines: text.lines().map(str::to_string).collect(),
            eol,
        }
    }

    /// The file's text, ending with a line break.
    pub fn to_text(&self) -> String {
        let mut text = self.lines.join(self.eol);
        if !self.lines.is_empty() {
            text.push_str(self.eol);
        }
        text
    }

    /// Each aisle's header line and name, in file order.
    fn aisles(&self) -> Vec<(usize, String)> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(at, line)| match kind(line) {
                Kind::Aisle(name) => Some((at, name)),
                _ => None,
            })
            .collect()
    }

    fn find_aisle(&self, name: &str) -> Result<usize, EditError> {
        self.aisles()
            .into_iter()
            .find(|(_, aisle)| aisle == name)
            .map(|(at, _)| at)
            .ok_or_else(|| EditError::NotFound(format!("There is no aisle called \"{name}\"")))
    }

    /// The aisle a line falls under, if any.
    fn aisle_of(&self, line: usize) -> Option<String> {
        self.lines[..line]
            .iter()
            .rev()
            .find_map(|line| match kind(line) {
                Kind::Aisle(name) => Some(name),
                _ => None,
            })
    }

    fn check_aisle_free(&self, name: &str, except: Option<usize>) -> Result<(), EditError> {
        let folded = name.to_lowercase();
        match self
            .aisles()
            .into_iter()
            .find(|(at, aisle)| Some(*at) != except && aisle.to_lowercase() == folded)
        {
            Some((_, aisle)) => Err(EditError::Taken(format!(
                "There is already an aisle called \"{aisle}\""
            ))),
            None => Ok(()),
        }
    }

    /// The lines that go with the aisle whose header is at `header`: the
    /// comments right above it, the header, and what follows up to the next
    /// aisle and the comments right above that one.
    fn block(&self, header: usize) -> Range<usize> {
        let is_comment = |at: usize| kind(&self.lines[at]) == Kind::Comment;
        let mut start = header;
        while start > 0 && is_comment(start - 1) {
            start -= 1;
        }
        let next = (header + 1..self.lines.len())
            .find(|&at| matches!(kind(&self.lines[at]), Kind::Aisle(_)))
            .unwrap_or(self.lines.len());
        let mut end = next;
        if next < self.lines.len() {
            while end > header + 1 && is_comment(end - 1) {
                end -= 1;
            }
        }
        start..end
    }

    /// Takes `range` out, returning its lines without the blank ones that
    /// ended it.
    fn take_block(&mut self, range: Range<usize>) -> Vec<String> {
        let at_end = range.end == self.lines.len();
        let mut block: Vec<String> = self.lines.drain(range).collect();
        while block.last().is_some_and(|line| kind(line) == Kind::Blank) {
            block.pop();
        }
        if at_end {
            while self
                .lines
                .last()
                .is_some_and(|line| kind(line) == Kind::Blank)
            {
                self.lines.pop();
            }
        }
        block
    }

    /// Puts `block` before the aisle now at `position`, or after the last one,
    /// with a blank line on either side.
    fn put_block(&mut self, mut block: Vec<String>, position: Option<usize>) {
        let before = position.and_then(|position| self.aisles().get(position).map(|a| a.0));
        match before {
            Some(header) => {
                let at = self.block(header).start;
                block.push(String::new());
                if at > 0 && kind(&self.lines[at - 1]) != Kind::Blank {
                    block.insert(0, String::new());
                }
                self.lines.splice(at..at, block);
            }
            None => {
                if self
                    .lines
                    .last()
                    .is_some_and(|line| kind(line) != Kind::Blank)
                {
                    self.lines.push(String::new());
                }
                self.lines.extend(block);
            }
        }
    }

    /// Adds an empty aisle at `position` among the aisles, or last.
    pub fn add_aisle(&mut self, name: &str, position: Option<usize>) -> Result<(), EditError> {
        let name = aisle_name(name)?;
        self.check_aisle_free(&name, None)?;
        self.put_block(vec![format!("[{name}]")], position);
        Ok(())
    }

    pub fn rename_aisle(&mut self, name: &str, new_name: &str) -> Result<(), EditError> {
        let header = self.find_aisle(name)?;
        let new_name = aisle_name(new_name)?;
        self.check_aisle_free(&new_name, Some(header))?;
        self.lines[header] = replace_code(&self.lines[header], &format!("[{new_name}]"));
        Ok(())
    }

    /// Removes an aisle along with the ingredients listed under it.
    pub fn remove_aisle(&mut self, name: &str) -> Result<(), EditError> {
        let header = self.find_aisle(name)?;
        let range = self.block(header);
        self.take_block(range);
        Ok(())
    }

    /// Moves an aisle so that it becomes the one at `position`, or the last
    /// when `position` is past the end. The shopping list follows the file's
    /// order.
    pub fn move_aisle(&mut self, name: &str, position: usize) -> Result<(), EditError> {
        let header = self.find_aisle(name)?;
        let aisles = self.aisles();
        let current = aisles.iter().position(|(at, _)| *at == header);
        if current == Some(position)
            || (current == Some(aisles.len() - 1) && position >= aisles.len())
        {
            return Ok(());
        }
        let range = self.block(header);
        let block = self.take_block(range);
        self.put_block(block, Some(position));
        Ok(())
    }

    /// Each ingredient line and the names on it.
    fn ingredients(&self) -> impl Iterator<Item = (usize, Vec<String>)> + '_ {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(at, line)| match kind(line) {
                Kind::Ingredient(names) => Some((at, names)),
                _ => None,
            })
    }

    /// The line naming `name`, compared ignoring case as the shopping list
    /// does.
    fn find_ingredient(&self, name: &str) -> Result<usize, EditError> {
        let folded = name.trim().to_lowercase();
        self.ingredients()
            .find(|(_, names)| names.iter().any(|n| n.to_lowercase() == folded))
            .map(|(at, _)| at)
            .ok_or_else(|| EditError::NotFound(format!("\"{}\" is not in any aisle", name.trim())))
    }

    /// Refuses names listed twice among `names`, or already on a line other
    /// than `except`. The shopping list looks names up ignoring case, so so
    /// does this.
    fn check_names_free(&self, names: &[String], except: Option<usize>) -> Result<(), EditError> {
        for (i, name) in names.iter().enumerate() {
            let folded = name.to_lowercase();
            if names[..i]
                .iter()
                .any(|other| other.to_lowercase() == folded)
            {
                return Err(EditError::Taken(format!("\"{name}\" is listed twice")));
            }
            let taken = self.ingredients().find(|(at, names)| {
                Some(*at) != except && names.iter().any(|n| n.to_lowercase() == folded)
            });
            if let Some((at, _)) = taken {
                return Err(EditError::Taken(match self.aisle_of(at) {
                    Some(aisle) => format!("\"{name}\" is already in the \"{aisle}\" aisle"),
                    None => format!("\"{name}\" is already in the file"),
                }));
            }
        }
        Ok(())
    }

    /// Puts an ingredient line under `aisle`, after its last ingredient.
    fn put_ingredient(&mut self, aisle: &str, line: String) -> Result<(), EditError> {
        let header = self.find_aisle(aisle)?;
        let range = self.block(header);
        let at = (header + 1..range.end)
            .rev()
            .find(|&at| matches!(kind(&self.lines[at]), Kind::Ingredient(_)))
            .map_or(header + 1, |at| at + 1);
        self.lines.insert(at, line);
        Ok(())
    }

    /// Lists an ingredient under `aisle`; the first of `names` is the one the
    /// shopping list shows, the others are its synonyms.
    pub fn add_ingredient(&mut self, aisle: &str, names: &[String]) -> Result<(), EditError> {
        let names = ingredient_names(names)?;
        self.find_aisle(aisle)?;
        self.check_names_free(&names, None)?;
        self.put_ingredient(aisle, names.join(" | "))
    }

    /// Gives the ingredient called `name` (by any of its names) the names
    /// `names`, and moves it to `aisle` when that is another aisle.
    pub fn update_ingredient(
        &mut self,
        name: &str,
        names: &[String],
        aisle: Option<&str>,
    ) -> Result<(), EditError> {
        let at = self.find_ingredient(name)?;
        let names = ingredient_names(names)?;
        self.check_names_free(&names, Some(at))?;
        let line = replace_code(&self.lines[at], &names.join(" | "));
        match aisle {
            Some(aisle) if Some(aisle) != self.aisle_of(at).as_deref() => {
                self.find_aisle(aisle)?;
                self.lines.remove(at);
                self.put_ingredient(aisle, line.trim_start().to_string())
            }
            _ => {
                self.lines[at] = line;
                Ok(())
            }
        }
    }

    pub fn remove_ingredient(&mut self, name: &str) -> Result<(), EditError> {
        let at = self.find_ingredient(name)?;
        self.lines.remove(at);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
// Store layout, front to back

[fruit and veg]
apples
avocado | avocados // ripe ones

onion | onions

// cold stuff
[dairy]
milk
butter

[bakery]
bread
";

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn edited(edit: impl FnOnce(&mut AisleFile) -> Result<(), EditError>) -> String {
        let mut file = AisleFile::parse(SAMPLE);
        edit(&mut file).unwrap();
        let text = file.to_text();
        cooklang::aisle::parse(&text).expect("the edit left a file the parser rejects");
        text
    }

    fn aisle_order(text: &str) -> Vec<String> {
        cooklang::aisle::parse(text)
            .unwrap()
            .categories
            .iter()
            .map(|c| c.name.to_string())
            .collect()
    }

    #[test]
    fn an_untouched_file_comes_back_as_it_was() {
        assert_eq!(AisleFile::parse(SAMPLE).to_text(), SAMPLE);
    }

    #[test]
    fn windows_line_endings_are_kept() {
        let text = SAMPLE.replace('\n', "\r\n");
        let mut file = AisleFile::parse(&text);
        file.add_ingredient("dairy", &names(&["eggs"])).unwrap();
        let out = file.to_text();
        assert!(out.contains("butter\r\neggs\r\n"));
        assert!(!out.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn an_ingredient_goes_after_the_last_one_of_its_aisle() {
        let text = edited(|f| f.add_ingredient("fruit and veg", &names(&[" leek ", "leeks"])));
        assert_eq!(
            text,
            SAMPLE.replace("onion | onions\n", "onion | onions\nleek | leeks\n")
        );
    }

    #[test]
    fn an_ingredient_can_go_into_an_empty_aisle() {
        let mut file = AisleFile::parse("[empty]\n\n[other]\nx\n");
        file.add_ingredient("empty", &names(&["y"])).unwrap();
        assert_eq!(file.to_text(), "[empty]\ny\n\n[other]\nx\n");
    }

    #[test]
    fn names_already_listed_are_refused_ignoring_case() {
        let mut file = AisleFile::parse(SAMPLE);
        assert_eq!(
            file.add_ingredient("dairy", &names(&["Avocados"])),
            Err(EditError::Taken(
                "\"Avocados\" is already in the \"fruit and veg\" aisle".to_string()
            ))
        );
        assert!(matches!(
            file.add_ingredient("dairy", &names(&["cream", "Cream"])),
            Err(EditError::Taken(_))
        ));
        assert_eq!(file.to_text(), SAMPLE);
    }

    #[test]
    fn names_the_file_could_not_hold_are_refused() {
        let mut file = AisleFile::parse(SAMPLE);
        for bad in [
            names(&[]),
            names(&["  "]),
            names(&["a|b"]),
            names(&["a // b"]),
            names(&["a\nb"]),
            names(&["[x]"]),
            names(&["[x", "y]"]),
        ] {
            assert!(
                matches!(
                    file.add_ingredient("dairy", &bad),
                    Err(EditError::InvalidName(_))
                ),
                "{bad:?} was accepted"
            );
        }
        assert!(matches!(
            file.add_aisle("a|b", None),
            Err(EditError::InvalidName(_))
        ));
        assert_eq!(file.to_text(), SAMPLE);
    }

    #[test]
    fn renaming_an_ingredient_keeps_its_comment() {
        let text = edited(|f| {
            f.update_ingredient("AVOCADOS", &names(&["avocado", "avocados", "hass"]), None)
        });
        assert_eq!(
            text,
            SAMPLE.replace(
                "avocado | avocados // ripe ones",
                "avocado | avocados | hass // ripe ones"
            )
        );
    }

    #[test]
    fn an_ingredient_keeps_its_own_names_when_edited() {
        // Its current names are not "taken" by itself.
        edited(|f| {
            f.update_ingredient("onion", &names(&["Onion", "onions"]), Some("fruit and veg"))
        });
    }

    #[test]
    fn moving_an_ingredient_takes_its_comment_along() {
        let text = edited(|f| f.update_ingredient("avocado", &names(&["avocado"]), Some("bakery")));
        assert!(!text.contains("[fruit and veg]\napples\navocado"));
        assert!(text.ends_with("[bakery]\nbread\navocado // ripe ones\n"));
    }

    #[test]
    fn removing_an_ingredient_removes_only_its_line() {
        let text = edited(|f| f.remove_ingredient("butter"));
        assert_eq!(text, SAMPLE.replace("butter\n", ""));
        let mut file = AisleFile::parse(SAMPLE);
        assert!(matches!(
            file.remove_ingredient("caviar"),
            Err(EditError::NotFound(_))
        ));
    }

    #[test]
    fn a_new_aisle_goes_last_or_where_asked() {
        let text = edited(|f| f.add_aisle("frozen", None));
        assert!(text.ends_with("bread\n\n[frozen]\n"));

        let text = edited(|f| f.add_aisle("frozen", Some(1)));
        assert_eq!(
            aisle_order(&text),
            ["fruit and veg", "frozen", "dairy", "bakery"]
        );
        // Before the comment that belongs to "dairy", not between the two.
        assert!(text.contains("onion | onions\n\n[frozen]\n\n// cold stuff\n[dairy]"));
    }

    #[test]
    fn aisle_names_are_unique_ignoring_case() {
        let mut file = AisleFile::parse(SAMPLE);
        assert!(matches!(
            file.add_aisle("Dairy", None),
            Err(EditError::Taken(_))
        ));
        assert!(matches!(
            file.rename_aisle("bakery", "DAIRY"),
            Err(EditError::Taken(_))
        ));
        // A change of case of the aisle itself is fine.
        file.rename_aisle("dairy", "Dairy").unwrap();
        assert!(file.to_text().contains("// cold stuff\n[Dairy]\nmilk"));
    }

    #[test]
    fn removing_an_aisle_takes_its_comment_and_ingredients() {
        let text = edited(|f| f.remove_aisle("dairy"));
        assert_eq!(
            text,
            SAMPLE.replace("// cold stuff\n[dairy]\nmilk\nbutter\n\n", "")
        );
        let text = edited(|f| f.remove_aisle("bakery"));
        assert!(text.ends_with("milk\nbutter\n"));
    }

    #[test]
    fn moving_an_aisle_carries_its_block() {
        let text = edited(|f| f.move_aisle("dairy", 0));
        assert_eq!(aisle_order(&text), ["dairy", "fruit and veg", "bakery"]);
        // The file's own heading stays at the top, the aisle's comment moves.
        assert!(text.starts_with(
            "// Store layout, front to back\n\n// cold stuff\n[dairy]\nmilk\nbutter\n\n[fruit and veg]\n"
        ));
        assert!(text.contains("onion | onions\n\n[bakery]\nbread\n"));

        let text = edited(|f| f.move_aisle("fruit and veg", 9));
        assert_eq!(aisle_order(&text), ["dairy", "bakery", "fruit and veg"]);
        assert!(text.ends_with(
            "bread\n\n[fruit and veg]\napples\navocado | avocados // ripe ones\n\nonion | onions\n"
        ));

        assert_eq!(edited(|f| f.move_aisle("dairy", 1)), SAMPLE);
        assert_eq!(edited(|f| f.move_aisle("bakery", 7)), SAMPLE);
    }

    #[test]
    fn an_empty_file_gets_its_first_aisle() {
        let mut file = AisleFile::parse("");
        file.add_aisle("pantry", None).unwrap();
        file.add_ingredient("pantry", &names(&["rice"])).unwrap();
        assert_eq!(file.to_text(), "[pantry]\nrice\n");
    }

    #[test]
    fn the_revision_follows_the_text() {
        assert_eq!(revision(SAMPLE), revision(SAMPLE));
        assert_ne!(revision(SAMPLE), revision(&SAMPLE.replace("milk", "Milk")));
    }
}
