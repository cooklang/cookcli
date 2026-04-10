use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use cooklang::shopping_list::{
    self, CheckEntry, RecipeItem, ShoppingList, ShoppingListItem,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;

/// An item exposed to the API layer — a recipe reference with path, name, and
/// scale factor. This preserves backward compatibility with existing API
/// consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShoppingListApiItem {
    pub path: String,
    pub name: String,
    pub scale: f64,
}

pub struct ShoppingListStore {
    /// Path to `.shopping-list`
    list_path: Utf8PathBuf,
    /// Path to `.shopping-checked`
    checked_path: Utf8PathBuf,
    /// Path to the legacy `.shopping_list.txt` (for migration detection)
    legacy_path: Utf8PathBuf,
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

        let content = fs::read_to_string(&self.legacy_path)
            .context("reading legacy shopping list")?;

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
    pub fn add(&self, item: ShoppingListApiItem) -> Result<()> {
        let mut list = self.load_list()?;
        let multiplier = if (item.scale - 1.0).abs() < f64::EPSILON {
            None
        } else {
            Some(item.scale)
        };
        list.items.push(ShoppingListItem::Recipe(RecipeItem {
            path: item.path,
            multiplier,
            children: Vec::new(),
        }));
        self.save_list(&list)
    }

    /// Remove the first recipe matching `path` and compact the checked log.
    pub fn remove(&self, path: &str) -> Result<()> {
        let mut list = self.load_list()?;
        if let Some(pos) = list.items.iter().position(|i| match i {
            ShoppingListItem::Recipe(r) => r.path == path,
            _ => false,
        }) {
            list.items.remove(pos);
        }
        self.save_list(&list)?;
        // Compact checked log — entries for removed ingredients become stale
        if self.checked_path.exists() {
            self.compact()?;
        }
        Ok(())
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
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.checked_path)
            .context("opening .shopping-checked for append")?;
        shopping_list::write_check_entry(&CheckEntry::Checked(name.to_string()), &mut file)?;
        Ok(())
    }

    /// Uncheck an ingredient (append `- name`).
    pub fn uncheck(&self, name: &str) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.checked_path)
            .context("opening .shopping-checked for append")?;
        shopping_list::write_check_entry(&CheckEntry::Unchecked(name.to_string()), &mut file)?;
        Ok(())
    }

    /// Compact the checked file against the current shopping list.
    pub fn compact(&self) -> Result<()> {
        let list = self.load_list()?;
        let content = self.read_checked_raw()?;
        let entries = shopping_list::parse_checked(&content);
        let compacted = shopping_list::compact_checked(&entries, &list);

        let mut buf = Vec::new();
        for entry in &compacted {
            shopping_list::write_check_entry(entry, &mut buf)?;
        }
        fs::write(&self.checked_path, buf).context("writing compacted .shopping-checked")
    }
}

// -- Conversion helpers --

fn api_items_from_list(list: &ShoppingList) -> Vec<ShoppingListApiItem> {
    let mut items = Vec::new();
    for item in &list.items {
        if let ShoppingListItem::Recipe(r) = item {
            items.push(ShoppingListApiItem {
                path: r.path.clone(),
                name: recipe_display_name(&r.path),
                scale: r.multiplier.unwrap_or(1.0),
            });
        }
    }
    items
}

/// Derive a human-readable display name from a recipe path.
/// E.g. "Breakfast/Easy Pancakes" → "Easy Pancakes"
fn recipe_display_name(path: &str) -> String {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}
