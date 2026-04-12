use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use cooklang::shopping_list::{self, CheckEntry, RecipeItem, ShoppingList, ShoppingListItem};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;

/// An item exposed to the API layer — a recipe reference with path, name,
/// scale factor, and optionally which sub-recipe references to include.
///
/// `included_references`:
///   - `None`  → include ALL references (default, backward-compat, menus)
///   - `Some([..])` → include only the listed reference paths
///
/// `recipes`:
///   - `None`  → this is a regular recipe entry
///   - `Some([..])` → this is a plan/menu entry with nested recipe items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShoppingListApiItem {
    pub path: String,
    pub name: String,
    pub scale: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub included_references: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipes: Option<Vec<ShoppingListApiItem>>,
}

pub struct ShoppingListStore {
    /// Path to `.shopping-list`
    list_path: Utf8PathBuf,
    /// Path to `.shopping-checked`
    checked_path: Utf8PathBuf,
    /// Path to the legacy `.shopping_list.txt` (for migration detection)
    legacy_path: Utf8PathBuf,
}

/// Convert a UI-supplied scale factor to the format's `multiplier` field.
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
    pub fn new(base_path: &Utf8PathBuf) -> Self {
        Self {
            list_path: base_path.join(".shopping-list"),
            checked_path: base_path.join(".shopping-checked"),
            legacy_path: base_path.join(".shopping_list.txt"),
        }
    }

    /// Migrate from the old tab-delimited `.shopping_list.txt` if it exists
    /// and the new `.shopping-list` does not.
    pub fn migrate_if_needed(&self) -> Result<bool> {
        if !self.legacy_path.exists() || self.list_path.exists() {
            return Ok(false);
        }

        tracing::info!(
            "Migrating shopping list from legacy format: {}",
            self.legacy_path
        );

        let content =
            fs::read_to_string(&self.legacy_path).context("reading legacy shopping list")?;

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
                let multiplier = if (scale - 1.0).abs() < f64::EPSILON {
                    None
                } else {
                    Some(scale)
                };
                items.push(ShoppingListItem::Recipe(RecipeItem {
                    path: path.to_string(),
                    multiplier,
                    children: Vec::new(),
                }));
            }
        }

        let list = ShoppingList { items };
        self.save_list(&list)?;

        // Rename the old file so it's not picked up again
        let backup = self.legacy_path.with_extension("txt.bak");
        fs::rename(&self.legacy_path, backup.as_std_path())
            .context("renaming legacy shopping list to .bak")?;

        tracing::info!("Migration complete. Legacy file renamed to {}", backup);
        Ok(true)
    }

    // -- Low-level I/O --

    fn read_list_raw(&self) -> Result<String> {
        if !self.list_path.exists() {
            return Ok(String::new());
        }
        fs::read_to_string(&self.list_path).context("reading .shopping-list")
    }

    fn read_checked_raw(&self) -> Result<String> {
        if !self.checked_path.exists() {
            return Ok(String::new());
        }
        fs::read_to_string(&self.checked_path).context("reading .shopping-checked")
    }

    // -- Shopping list operations --

    pub fn load_list(&self) -> Result<ShoppingList> {
        let content = self.read_list_raw()?;
        if content.is_empty() {
            return Ok(ShoppingList::default());
        }
        shopping_list::parse(&content).map_err(|e| anyhow::anyhow!("{}", e))
    }

    fn save_list(&self, list: &ShoppingList) -> Result<()> {
        let mut buf = Vec::new();
        shopping_list::write(list, &mut buf)?;
        fs::write(&self.list_path, buf).context("writing .shopping-list")
    }

    /// Return API-compatible items for the shopping list.
    pub fn load(&self) -> Result<Vec<ShoppingListApiItem>> {
        self.migrate_if_needed()?;
        let list = self.load_list()?;
        Ok(api_items_from_list(&list))
    }

    /// Add a recipe to the shopping list.
    ///
    /// If `included_references` is `Some`, the listed reference paths are stored
    /// as child recipe entries so the shopping list generator knows which
    /// sub-recipes to expand.
    pub fn add(&self, item: ShoppingListApiItem) -> Result<()> {
        // Ensure we migrate before the first mutation — otherwise a write
        // here would create an empty `.shopping-list` and make the legacy
        // file invisible to future migration.
        self.migrate_if_needed()?;
        let mut list = self.load_list()?;

        // Store included references as child recipe entries.
        // Strip leading "./" from reference paths — the format writer adds it back.
        let children = match item.included_references {
            Some(refs) => refs
                .into_iter()
                .map(|path| {
                    let path = path.strip_prefix("./").unwrap_or(&path).to_string();
                    ShoppingListItem::Recipe(RecipeItem {
                        path,
                        multiplier: None,
                        children: Vec::new(),
                    })
                })
                .collect(),
            None => Vec::new(),
        };

        list.items.push(ShoppingListItem::Recipe(RecipeItem {
            path: item.path,
            multiplier: to_multiplier(item.scale),
            children,
        }));
        self.save_list(&list)
    }

    /// Add a menu/plan to the shopping list as a single entry with nested recipes.
    ///
    /// Each recipe in `recipes` becomes a child of the menu entry. Each recipe's
    /// `included_references` become grandchildren (sub-recipe references).
    pub fn add_menu(
        &self,
        menu_path: String,
        menu_scale: f64,
        recipes: Vec<ShoppingListApiItem>,
    ) -> Result<()> {
        self.migrate_if_needed()?;
        let mut list = self.load_list()?;

        let children: Vec<ShoppingListItem> = recipes
            .into_iter()
            .map(|recipe| {
                let recipe_multiplier = to_multiplier(recipe.scale);

                let sub_children = match recipe.included_references {
                    Some(refs) => refs
                        .into_iter()
                        .map(|path| {
                            let path = path.strip_prefix("./").unwrap_or(&path).to_string();
                            ShoppingListItem::Recipe(RecipeItem {
                                path,
                                multiplier: None,
                                children: Vec::new(),
                            })
                        })
                        .collect(),
                    None => Vec::new(),
                };

                ShoppingListItem::Recipe(RecipeItem {
                    path: recipe.path,
                    multiplier: recipe_multiplier,
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

    /// Remove the first recipe matching `path`.
    ///
    /// Compaction of the checked log (which drops entries for ingredients
    /// no longer in any remaining recipe) is the caller's responsibility.
    /// The store has no parser context to expand recipe references into
    /// ingredient names, so if we invoked `compact()` here with an empty
    /// ingredient list it would wipe every check.
    pub fn remove(&self, path: &str) -> Result<()> {
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

    /// Clear the shopping list and checked state.
    pub fn clear(&self) -> Result<()> {
        self.save_list(&ShoppingList::default())?;
        if self.checked_path.exists() {
            fs::remove_file(&self.checked_path).context("removing .shopping-checked")?;
        }
        Ok(())
    }

    // -- Checked state operations --

    /// Return the set of currently checked ingredient names (lowercased).
    pub fn checked_set(&self) -> Result<HashSet<String>> {
        let content = self.read_checked_raw()?;
        let entries = shopping_list::parse_checked(&content);
        Ok(shopping_list::checked_set(&entries))
    }

    /// Check an ingredient (append `+ name`).
    pub fn check(&self, name: &str) -> Result<()> {
        self.append_check_entry(&CheckEntry::Checked(name.to_string()))
    }

    /// Uncheck an ingredient (append `- name`).
    pub fn uncheck(&self, name: &str) -> Result<()> {
        self.append_check_entry(&CheckEntry::Unchecked(name.to_string()))
    }

    /// Append a `+ name` / `- name` entry to the checked log.
    ///
    /// Mutual exclusion against a concurrent `compact()` is the caller's
    /// responsibility — in the server this is the process-wide
    /// `AppState::checked_log_lock`. File-level `flock` would not help: it
    /// doesn't serialize callers in the same process (the kernel treats them
    /// as one lock owner), which is the case that actually matters here.
    fn append_check_entry(&self, entry: &CheckEntry) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.checked_path)
            .context("opening .shopping-checked for append")?;
        shopping_list::write_check_entry(entry, &mut file)?;
        Ok(())
    }

    /// Compact the checked file against the user-visible ingredient names.
    ///
    /// The on-disk `.shopping-list` persists only recipe references, not
    /// expanded ingredients. Callers must first aggregate the actual
    /// ingredient names (by parsing the referenced recipes) and pass them
    /// here — otherwise every checked entry would be treated as stale.
    ///
    /// Mutual exclusion against concurrent `check()`/`uncheck()`/`compact()`
    /// is the caller's responsibility (see `append_check_entry` for context).
    ///
    /// The rewrite is atomic: we stage the compacted output to a sibling
    /// temp file, fsync it, then `rename` it into place. `rename` is atomic
    /// on POSIX, so a crash between truncate and write can't leave a
    /// zero-length `.shopping-checked`.
    pub fn compact<I, S>(&self, current_ingredients: I) -> Result<()>
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
            shopping_list::write_check_entry(entry, &mut buf)?;
        }

        // Stage the new content in a sibling temp file, fsync it, and rename
        // into place. Temp path is scoped to base_path so it ends up on the
        // same filesystem — otherwise rename would fall back to copy+delete
        // and lose atomicity.
        let tmp_path = self.checked_path.with_file_name(".shopping-checked.tmp");
        {
            use std::io::Write as _;
            let mut tmp = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp_path)
                .context("opening .shopping-checked temp file")?;
            tmp.write_all(&buf).context("writing compacted temp file")?;
            tmp.sync_all().context("fsync-ing compacted temp file")?;
        }
        fs::rename(&tmp_path, &self.checked_path)
            .context("renaming compacted temp file into place")?;

        Ok(())
    }
}

// -- Conversion helpers --

fn api_items_from_list(list: &ShoppingList) -> Vec<ShoppingListApiItem> {
    let mut items = Vec::new();
    for item in &list.items {
        if let ShoppingListItem::Recipe(r) = item {
            if r.path.ends_with(".menu") {
                // Plan/menu entry — children are recipes, grandchildren are sub-references.
                let recipes: Vec<ShoppingListApiItem> = r
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ShoppingListItem::Recipe(cr) => {
                            let refs: Vec<String> = cr
                                .children
                                .iter()
                                .filter_map(|gc| match gc {
                                    ShoppingListItem::Recipe(gcr) => Some(gcr.path.clone()),
                                    _ => None,
                                })
                                .collect();
                            Some(ShoppingListApiItem {
                                path: cr.path.clone(),
                                name: recipe_display_name(&cr.path),
                                scale: cr.multiplier.unwrap_or(1.0),
                                included_references: Some(refs),
                                recipes: None,
                            })
                        }
                        _ => None,
                    })
                    .collect();
                items.push(ShoppingListApiItem {
                    path: r.path.clone(),
                    name: recipe_display_name(&r.path),
                    scale: r.multiplier.unwrap_or(1.0),
                    included_references: None,
                    recipes: Some(recipes),
                });
            } else {
                // Regular recipe entry — children are sub-references.
                let refs: Vec<String> = r
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ShoppingListItem::Recipe(cr) => Some(cr.path.clone()),
                        _ => None,
                    })
                    .collect();
                items.push(ShoppingListApiItem {
                    path: r.path.clone(),
                    name: recipe_display_name(&r.path),
                    scale: r.multiplier.unwrap_or(1.0),
                    included_references: Some(refs),
                    recipes: None,
                });
            }
        }
    }
    items
}

/// Derive a human-readable display name from a recipe path.
/// E.g. "Breakfast/Easy Pancakes.cook" → "Easy Pancakes"
/// E.g. "Meal Plans/Week 1.menu" → "Week 1"
pub fn recipe_display_name(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    let name = name.strip_suffix(".cook").unwrap_or(name);
    name.strip_suffix(".menu").unwrap_or(name).to_string()
}
