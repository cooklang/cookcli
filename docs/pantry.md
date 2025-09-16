# Pantry Command

The `pantry` command helps you manage and analyze your pantry inventory, tracking what items are low on stock, expiring soon, or available for cooking.

## Overview

```bash
cook pantry [OPTIONS] <SUBCOMMAND>
```

## Options

- `-b, --base-path <PATH>` - Base path for recipes and configuration files
- `-f, --format <FORMAT>` - Output format: `human` (default), `json`, or `yaml`
- `-v, --verbose` - Increase verbosity (can be used multiple times)
- `-h, --help` - Print help information

## Subcommands

### `depleted` (alias: `d`)

Shows items that are out of stock or have low quantities.

```bash
cook pantry depleted [OPTIONS]
```

**Options:**
- `--all` - Show all items including those without quantities

**How it works:**
- Shows items where current quantity is at or below the defined `low` threshold
- If no `low` threshold is defined, uses heuristics (≤100g/ml, ≤1 item)
- Displays the low threshold for each item when available

**Example:**
```bash
$ cook pantry depleted
Depleted or Low Stock Items:
============================

PANTRY:
  • flour (400%g) [low when < 500%g]
  • garlic (5) [low when < 10]
  • onion (3) [low when < 5]
```

### `expiring` (alias: `e`)

Shows items that are expiring soon.

```bash
cook pantry expiring [OPTIONS]
```

**Options:**
- `-d, --days <DAYS>` - Number of days to look ahead (default: 7)
- `--include-unknown` - Include items without expiry dates

**Example:**
```bash
$ cook pantry expiring --days 14
Items Expiring Within 14 Days:
================================

Expiring Soon:
  • yogurt - 2025-09-17 (expires tomorrow) [fridge]
  • eggs - 2025-09-18 (expires in 2 days) [fridge]
  • milk - 2025-09-20 (expires in 4 days) [fridge]
```

### `recipes` (alias: `r`)

Lists recipes that can be made with items currently in pantry.

```bash
cook pantry recipes [OPTIONS]
```

**Options:**
- `-p, --partial` - Include partial matches (recipes where most ingredients are available)
- `--threshold <PERCENT>` - Minimum percentage of ingredients that must be available for partial matches (default: 75)

**Example:**
```bash
$ cook pantry recipes --partial --threshold 50
Recipes You Can Make with Pantry Items:
========================================

✓ Complete Matches (all ingredients available):
  • Pasta Carbonara
  • Simple Salad

⚠ Partial Matches (50%+ ingredients available):
  • Pizza Margherita (80% available)
    Missing: fresh basil, mozzarella
```

## Pantry Configuration

The pantry inventory is defined in `pantry.conf` (TOML format), which is searched for in:
1. `./config/pantry.conf` - Local to recipe directory
2. `~/.config/cook/pantry.conf` - Global configuration (Linux/macOS)

### Configuration Format

```toml
[fridge]
milk = { quantity = "500%ml", low = "200%ml", expire = "2025-09-20" }
eggs = { quantity = "12", low = "6", bought = "2025-09-10", expire = "2025-09-25" }
butter = "250%g"  # Simple format with just quantity

[pantry]
flour = { quantity = "2%kg", low = "500%g" }
sugar = { quantity = "1%kg", low = "200%g" }
olive oil = { quantity = "750%ml", low = "250%ml" }

[spices]
oregano = { quantity = "20%g" }
basil = { quantity = "15%g", low = "5%g" }
```

### Item Attributes

Each pantry item can have the following optional attributes:

- **`quantity`** - Current amount (e.g., "500%ml", "2%kg", "5")
- **`low`** - Threshold for low stock warning (same format as quantity)
- **`bought`** - Purchase date (e.g., "2025-09-10")
- **`expire`** - Expiry date (e.g., "2025-09-25")

### Low Stock Threshold

The `low` attribute defines when an item should be considered low on stock:

```toml
flour = { quantity = "400%g", low = "500%g" }  # Low: 400 < 500
sugar = { quantity = "2%kg", low = "1%kg" }    # OK: 2 > 1
```

**Important:** Comparisons only work when units match:
- ✅ `400%g` vs `500%g` (both grams)
- ✅ `2%kg` vs `1%kg` (both kilograms)
- ❌ `1%kg` vs `500%g` (different units - no comparison)

For items without units (counts), use plain numbers:
```toml
eggs = { quantity = "6", low = "12" }
```

## Integration with Other Commands

### Shopping List

The pantry configuration works with the shopping list command to:
- Filter out items already in stock
- Show what needs restocking based on low thresholds

### Recipe Command

Recipes can reference pantry items and the system will:
- Check availability before cooking
- Warn about low stock items used in recipes

## Examples

### Check Low Stock Items
```bash
# Show items that are low or out
cook pantry depleted

# Show all items including those with sufficient stock
cook pantry depleted --all
```

### Monitor Expiring Items
```bash
# Check items expiring in the next week
cook pantry expiring

# Check items expiring in the next month
cook pantry expiring --days 30

# Include items without expiry dates
cook pantry expiring --include-unknown
```

### Find Available Recipes
```bash
# Show only recipes with all ingredients available
cook pantry recipes

# Include recipes where you have most ingredients
cook pantry recipes --partial

# Show recipes with at least 60% of ingredients
cook pantry recipes --partial --threshold 60
```

### Different Recipe Collections
```bash
# Check pantry for a different recipe collection
cook pantry -b ~/recipes depleted

# Use pantry in another directory
cook pantry --base-path ../other-recipes recipes
```

### Output Formats

The pantry command supports multiple output formats for easy integration with other tools:

```bash
# Default human-readable format
cook pantry depleted

# JSON output for programmatic use
cook pantry -f json depleted

# YAML output
cook pantry -f yaml expiring --days 30

# Combine with other options
cook pantry -f json recipes --partial --threshold 60
```

**JSON Output Example:**
```json
{
  "items": [
    {
      "name": "flour",
      "section": "pantry",
      "quantity": "400%g",
      "low_threshold": "500%g",
      "is_low": true
    },
    {
      "name": "eggs",
      "section": "fridge",
      "quantity": "6",
      "low_threshold": "12",
      "is_low": true
    }
  ]
}
```

**YAML Output Example:**
```yaml
items:
- name: milk
  section: fridge
  expire_date: "2025-09-20"
  days_until_expiry: 4
  status: "expires in 4 days"
- name: eggs
  section: fridge
  expire_date: "2025-09-18"
  days_until_expiry: 2
  status: "expires in 2 days"
```

## Tips

1. **Regular Updates**: Keep your pantry.conf updated as you shop and use items
2. **Set Realistic Thresholds**: Set `low` values based on your shopping patterns
3. **Use Sections**: Organize items by storage location (fridge, freezer, pantry, etc.)
4. **Track Expiry**: Add expiry dates to perishables to reduce waste
5. **Combine with Shopping**: Use `pantry depleted` output to create shopping lists
6. **Automate with JSON/YAML**: Use `-f json` or `-f yaml` for scripting and automation
7. **Integration**: Parse JSON output in scripts to send notifications, generate reports, or update other systems