//! The shopping list someone is keeping: `.shopping-list` and
//! `.shopping-checked`.
//!
//! [`ShoppingListStore`] owns the pair of dotfiles that live beside a recipe
//! collection. `.shopping-list` holds recipe references — which recipe, at what
//! multiplier, and which of its sub-recipe references to expand — and
//! `.shopping-checked` is an append-only log of what has been ticked off while
//! shopping.
//!
//! Neither file holds ingredients. Turning the stored references into a list of
//! things to buy is [`generate`](super::generate) and
//! [`extract_ingredients`](super::extract_ingredients); this module only
//! persists what was asked for. That split is why [`ShoppingListStore::compact`]
//! takes the ingredient names from its caller rather than working them out
//! itself.
//!
//! The two files are the same ones the Cooklang apps read and write, so
//! anything here has to keep their format exactly.

use crate::fs_atomic::write_atomically;
use crate::CoreError;
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::shopping_list::{self, CheckEntry, RecipeItem, ShoppingList, ShoppingListItem};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;

/// One entry of a saved shopping list: a recipe, or a menu with recipes
/// nested inside it.
///
/// A flattened view of what `.shopping-list` stores, so that a caller does not
/// have to walk [`cooklang::shopping_list::ShoppingListItem`] trees itself.
///
/// `included_references`:
///   - `None` → include ALL of the recipe's sub-recipe references (the default,
///     and what a menu entry carries)
///   - `Some([..])` → include only the listed reference paths
///
/// `recipes`:
///   - `None` → this is a regular recipe entry
///   - `Some([..])` → this is a menu entry, and these are the recipes in it
///
/// Not `#[non_exhaustive]`: callers construct one to hand to
/// [`ShoppingListStore::add`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEntry {
    /// Path to the recipe or menu, relative to the collection root — e.g.
    /// `Breakfast/Easy Pancakes.cook`.
    pub path: String,
    /// The display name, as [`recipe_display_name`] derives it from `path`.
    /// Ignored when adding: it is always re-derived on load.
    pub name: String,
    /// How much of it to make. `1.0` is stored as no multiplier at all.
    pub scale: f64,
    /// Which of the entry's sub-recipe references to expand, or `None` for all
    /// of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub included_references: Option<Vec<String>>,
    /// The recipes in a menu entry, or `None` for a plain recipe entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipes: Option<Vec<StoredEntry>>,
}

/// Reads and writes the `.shopping-list` / `.shopping-checked` pair beside a
/// recipe collection.
///
/// Cheap to construct and holds no state of its own — every method reads the
/// files afresh — so it is fine to build one per request rather than keep one
/// around.
///
/// Nothing here locks. Two callers changing the same list at once can lose one
/// of the two changes (last write wins), and the checked log in particular
/// needs mutual exclusion its callers provide: see
/// [`check`](ShoppingListStore::check) and
/// [`compact`](ShoppingListStore::compact).
pub struct ShoppingListStore {
    /// Path to `.shopping-list`
    list_path: Utf8PathBuf,
    /// Path to `.shopping-checked`
    checked_path: Utf8PathBuf,
    /// Path to the legacy `.shopping_list.txt` (for migration detection)
    legacy_path: Utf8PathBuf,
}

/// Convert a caller-supplied scale factor to the format's `multiplier` field.
/// 1.0 (or anything indistinguishable from 1.0 in f64) serializes as no
/// multiplier at all — the `.shopping-list` format treats a bare path as
/// `×1` implicitly.
fn to_multiplier(scale: f64) -> Option<f64> {
    if (scale - 1.0).abs() < f64::EPSILON {
        None
    } else {
        Some(scale)
    }
}

impl ShoppingListStore {
    /// Open the store for the collection rooted at `base_path`.
    ///
    /// Touches nothing: the files are read and created as they are needed.
    pub fn new(base_path: &Utf8Path) -> Self {
        Self {
            list_path: base_path.join(".shopping-list"),
            checked_path: base_path.join(".shopping-checked"),
            legacy_path: base_path.join(".shopping_list.txt"),
        }
    }

    /// Migrate from the old tab-delimited `.shopping_list.txt` if it exists
    /// and the new `.shopping-list` does not.
    ///
    /// Runs before anything that reads or changes the list, so a collection
    /// written by an older CookCLI is picked up rather than silently starting
    /// empty. The legacy file is renamed to `.shopping_list.txt.bak` once its
    /// contents are safely in the new file.
    fn migrate_if_needed(&self) -> Result<bool, CoreError> {
        if !self.legacy_path.exists() || self.list_path.exists() {
            return Ok(false);
        }

        tracing::info!(
            "Migrating shopping list from legacy format: {}",
            self.legacy_path
        );

        let content = fs::read_to_string(&self.legacy_path).map_err(|source| CoreError::Io {
            path: self.legacy_path.clone(),
            source,
        })?;

        let mut items = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let path = parts[0];
                let scale: f64 = parts[2].parse().unwrap_or(1.0);
                items.push(ShoppingListItem::Recipe(RecipeItem {
                    path: path.to_string(),
                    multiplier: to_multiplier(scale),
                    children: Vec::new(),
                }));
            }
        }

        let list = ShoppingList { items };
        self.save_list(&list)?;

        // Rename the old file so it's not picked up again
        let backup = self.legacy_path.with_extension("txt.bak");
        crate::fs_atomic::rename_replace(&self.legacy_path, &backup).map_err(|source| {
            CoreError::Io {
                path: self.legacy_path.clone(),
                source,
            }
        })?;

        tracing::info!("Migration complete. Legacy file renamed to {}", backup);
        Ok(true)
    }

    // -- Low-level I/O --

    fn read_list_raw(&self) -> Result<String, CoreError> {
        if !self.list_path.exists() {
            return Ok(String::new());
        }
        fs::read_to_string(&self.list_path).map_err(|source| CoreError::Io {
            path: self.list_path.clone(),
            source,
        })
    }

    fn read_checked_raw(&self) -> Result<String, CoreError> {
        if !self.checked_path.exists() {
            return Ok(String::new());
        }
        fs::read_to_string(&self.checked_path).map_err(|source| CoreError::Io {
            path: self.checked_path.clone(),
            source,
        })
    }

    // -- Shopping list operations --

    fn load_list(&self) -> Result<ShoppingList, CoreError> {
        let content = self.read_list_raw()?;
        if content.is_empty() {
            return Ok(ShoppingList::default());
        }
        shopping_list::parse(&content).map_err(|e| CoreError::InvalidShoppingList {
            path: self.list_path.clone(),
            message: e.to_string(),
        })
    }

    /// Serialise `list` over `.shopping-list`, atomically.
    ///
    /// Every change rewrites the whole file, so a write that failed half way
    /// would leave the user with a truncated shopping list — hence
    /// [`write_atomically`] rather than [`std::fs::write`].
    fn save_list(&self, list: &ShoppingList) -> Result<(), CoreError> {
        let mut buf = Vec::new();
        shopping_list::write(list, &mut buf).map_err(|source| CoreError::Io {
            path: self.list_path.clone(),
            source,
        })?;
        write_atomically(&self.list_path, &buf)
    }

    /// Everything currently on the shopping list, in the order it was added.
    ///
    /// Migrates a legacy list first, and answers with an empty `Vec` when there
    /// is no list rather than failing.
    pub fn load(&self) -> Result<Vec<StoredEntry>, CoreError> {
        self.migrate_if_needed()?;
        let list = self.load_list()?;
        Ok(entries_from_list(&list))
    }

    /// Add a recipe to the shopping list.
    ///
    /// Appends: adding the same recipe twice stores it twice, which is how
    /// someone asks for two batches of it.
    ///
    /// If `included_references` is `Some`, the listed reference paths are stored
    /// as child recipe entries so the shopping list generator knows which
    /// sub-recipes to expand. `StoredEntry::name` is ignored — it is derived
    /// from the path on load.
    pub fn add(&self, item: StoredEntry) -> Result<(), CoreError> {
        // Ensure we migrate before the first mutation — otherwise a write
        // here would create an empty `.shopping-list` and make the legacy
        // file invisible to future migration.
        self.migrate_if_needed()?;
        let mut list = self.load_list()?;

        // Store included references as child recipe entries.
        // Strip leading "./" from reference paths — the format writer adds it back.
        let children = match item.included_references {
            Some(refs) => refs.into_iter().map(child_reference).collect(),
            None => Vec::new(),
        };

        list.items.push(ShoppingListItem::Recipe(RecipeItem {
            path: item.path,
            multiplier: to_multiplier(item.scale),
            children,
        }));
        self.save_list(&list)
    }

    /// Add a menu to the shopping list as a single entry with nested recipes.
    ///
    /// Each recipe in `recipes` becomes a child of the menu entry. Each recipe's
    /// `included_references` become grandchildren (sub-recipe references).
    /// Removing the menu later removes everything under it in one go, which is
    /// the point of storing it this way rather than as loose recipes.
    pub fn add_menu(
        &self,
        menu_path: String,
        menu_scale: f64,
        recipes: Vec<StoredEntry>,
    ) -> Result<(), CoreError> {
        self.migrate_if_needed()?;
        let mut list = self.load_list()?;

        let children: Vec<ShoppingListItem> = recipes
            .into_iter()
            .map(|recipe| {
                let sub_children = match recipe.included_references {
                    Some(refs) => refs.into_iter().map(child_reference).collect(),
                    None => Vec::new(),
                };

                ShoppingListItem::Recipe(RecipeItem {
                    path: recipe.path,
                    multiplier: to_multiplier(recipe.scale),
                    children: sub_children,
                })
            })
            .collect();

        list.items.push(ShoppingListItem::Recipe(RecipeItem {
            path: menu_path,
            multiplier: to_multiplier(menu_scale),
            children,
        }));
        self.save_list(&list)
    }

    /// Remove the first entry whose path is `path`, and do nothing if there is
    /// none.
    ///
    /// Compaction of the checked log (which drops entries for ingredients
    /// no longer in any remaining recipe) is the caller's responsibility.
    /// The store has no parser context to expand recipe references into
    /// ingredient names, so if we invoked [`compact`](Self::compact) here with
    /// an empty ingredient list it would wipe every check.
    pub fn remove(&self, path: &str) -> Result<(), CoreError> {
        self.migrate_if_needed()?;
        let mut list = self.load_list()?;
        if let Some(pos) = list.items.iter().position(|i| match i {
            ShoppingListItem::Recipe(r) => r.path == path,
            _ => false,
        }) {
            list.items.remove(pos);
        }
        self.save_list(&list)
    }

    /// Empty the shopping list and forget everything that was ticked off.
    pub fn clear(&self) -> Result<(), CoreError> {
        self.save_list(&ShoppingList::default())?;
        if self.checked_path.exists() {
            fs::remove_file(&self.checked_path).map_err(|source| CoreError::Io {
                path: self.checked_path.clone(),
                source,
            })?;
        }
        Ok(())
    }

    // -- Checked state operations --

    /// The ingredients currently ticked off, **lowercased** — the checked log
    /// is matched case-insensitively, so compare against `name.to_lowercase()`.
    pub fn checked_set(&self) -> Result<HashSet<String>, CoreError> {
        let content = self.read_checked_raw()?;
        let entries = shopping_list::parse_checked(&content);
        Ok(shopping_list::checked_set(&entries))
    }

    /// Tick an ingredient off (appends `+ name` to the checked log).
    ///
    /// Mutual exclusion against a concurrent [`compact`](Self::compact) is the
    /// caller's responsibility — in CookCLI's web server this is the
    /// process-wide `AppState::checked_log_lock`. File-level `flock` would not
    /// help: it doesn't serialize callers in the same process (the kernel
    /// treats them as one lock owner), which is the case that actually matters
    /// here.
    pub fn check(&self, name: &str) -> Result<(), CoreError> {
        self.append_check_entry(&CheckEntry::Checked(name.to_string()))
    }

    /// Un-tick an ingredient (appends `- name` to the checked log). Same
    /// locking note as [`check`](Self::check).
    pub fn uncheck(&self, name: &str) -> Result<(), CoreError> {
        self.append_check_entry(&CheckEntry::Unchecked(name.to_string()))
    }

    /// Append a `+ name` / `- name` entry to the checked log.
    ///
    /// An append rather than a rewrite, so this one write does not need
    /// staging: it either lands whole or is the last, partial line of a log
    /// whose parser skips lines it cannot read.
    fn append_check_entry(&self, entry: &CheckEntry) -> Result<(), CoreError> {
        let failed = |source: std::io::Error| CoreError::Io {
            path: self.checked_path.clone(),
            source,
        };
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.checked_path)
            .map_err(failed)?;
        shopping_list::write_check_entry(entry, &mut file).map_err(failed)
    }

    /// Compact the checked log against the user-visible ingredient names.
    ///
    /// The on-disk `.shopping-list` persists only recipe references, not
    /// expanded ingredients. Callers must first aggregate the actual
    /// ingredient names (by parsing the referenced recipes) and pass them
    /// here — otherwise every checked entry would be treated as stale and the
    /// user would lose every tick.
    ///
    /// Mutual exclusion against concurrent `check`/`uncheck`/`compact` is the
    /// caller's responsibility (see [`check`](Self::check)).
    ///
    /// The rewrite is atomic, so a crash part way through cannot leave a
    /// zero-length `.shopping-checked`.
    pub fn compact<I, S>(&self, current_ingredients: I) -> Result<(), CoreError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let names: Vec<String> = current_ingredients
            .into_iter()
            .map(|s| s.as_ref().to_string())
            .collect();

        let content = self.read_checked_raw()?;
        let entries = shopping_list::parse_checked(&content);
        let compacted = shopping_list::compact_checked(&entries, names.iter().map(String::as_str));

        let mut buf = Vec::new();
        for entry in &compacted {
            shopping_list::write_check_entry(entry, &mut buf).map_err(|source| CoreError::Io {
                path: self.checked_path.clone(),
                source,
            })?;
        }

        write_atomically(&self.checked_path, &buf)
    }
}

// -- Conversion helpers --

/// A sub-recipe reference as it is stored under its parent. The `./` a
/// reference is written with is stripped, because the format writer adds it
/// back.
fn child_reference(path: String) -> ShoppingListItem {
    let path = path.strip_prefix("./").unwrap_or(&path).to_string();
    ShoppingListItem::Recipe(RecipeItem {
        path,
        multiplier: None,
        children: Vec::new(),
    })
}

/// The paths of an item's direct recipe children, ignoring any free-hand
/// ingredients among them.
fn child_paths(item: &RecipeItem) -> Vec<String> {
    item.children
        .iter()
        .filter_map(|child| match child {
            ShoppingListItem::Recipe(recipe) => Some(recipe.path.clone()),
            _ => None,
        })
        .collect()
}

fn entries_from_list(list: &ShoppingList) -> Vec<StoredEntry> {
    let mut items = Vec::new();
    for item in &list.items {
        if let ShoppingListItem::Recipe(r) = item {
            if r.path.ends_with(".menu") {
                // Menu entry — children are recipes, grandchildren are sub-references.
                let recipes: Vec<StoredEntry> = r
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ShoppingListItem::Recipe(cr) => Some(StoredEntry {
                            path: cr.path.clone(),
                            name: recipe_display_name(&cr.path),
                            scale: cr.multiplier.unwrap_or(1.0),
                            included_references: Some(child_paths(cr)),
                            recipes: None,
                        }),
                        _ => None,
                    })
                    .collect();
                items.push(StoredEntry {
                    path: r.path.clone(),
                    name: recipe_display_name(&r.path),
                    scale: r.multiplier.unwrap_or(1.0),
                    included_references: None,
                    recipes: Some(recipes),
                });
            } else {
                // Regular recipe entry — children are sub-references.
                items.push(StoredEntry {
                    path: r.path.clone(),
                    name: recipe_display_name(&r.path),
                    scale: r.multiplier.unwrap_or(1.0),
                    included_references: Some(child_paths(r)),
                    recipes: None,
                });
            }
        }
    }
    items
}

/// Derive a human-readable display name from a recipe or menu path.
///
/// E.g. `Breakfast/Easy Pancakes.cook` → `Easy Pancakes`, and
/// `Meal Plans/Week 1.menu` → `Week 1`.
pub fn recipe_display_name(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    let name = name.strip_suffix(".cook").unwrap_or(name);
    name.strip_suffix(".menu").unwrap_or(name).to_string()
}

#[cfg(test)]
mod tests;
