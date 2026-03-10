# Pantry Command

Manage and analyze your pantry inventory. Supports full CRUD operations
on pantry items as well as analysis commands for stock levels, expiry
dates, and recipe planning.

## Usage

```
cook pantry [OPTIONS] <COMMAND>
```

## Options

| Option | Description |
|--------|-------------|
| `-b, --base-path <PATH>` | Base path for recipes and configuration files |
| `-f, --format <FORMAT>` | Output format: `human` (default), `json`, `yaml` |

## Subcommands

### `list` (alias: `ls`)

Display all pantry items organized by section.

```
cook pantry list [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--section <NAME>` | Only show items from this section |

```bash
cook pantry list                     # All sections
cook pantry list --section dairy     # Only the dairy section
cook pantry -f json list             # JSON output
```

### `add` (alias: `a`)

Add a new item to the pantry. Creates the pantry file and the section if
they do not exist yet.

```
cook pantry add <SECTION> <NAME> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--quantity <VALUE>` | Quantity on hand (e.g. `2%kg`, `500%ml`, `12`) |
| `--low <VALUE>` | Low-stock threshold (e.g. `200%g`) |
| `--expire <DATE>` | Expiry date (e.g. `2025-06-01`) |
| `--bought <DATE>` | Purchase date |

```bash
cook pantry add pantry flour                                    # Simple item
cook pantry add dairy milk --quantity "2%l" --low "500%ml"      # With attributes
cook pantry add dairy yogurt --quantity "500%g" --expire 2025-06-01
```

### `remove` (alias: `rm`)

Remove an item from a section. If the section becomes empty it is deleted
from the file too.

```
cook pantry remove <SECTION> <NAME>
```

```bash
cook pantry remove dairy milk
cook pantry rm pantry flour
```

### `update` (alias: `up`)

Update one or more attributes of an existing pantry item. Only the flags
you provide are changed; everything else is kept as-is.

```
cook pantry update <SECTION> <NAME> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--quantity <VALUE>` | New quantity |
| `--low <VALUE>` | New low-stock threshold |
| `--expire <DATE>` | New expiry date |
| `--bought <DATE>` | New purchase date |

```bash
cook pantry update dairy milk --quantity "1%l"
cook pantry update dairy milk --expire 2025-06-15 --low "500%ml"
cook pantry up pantry flour --quantity "2%kg"
```

### `depleted` (alias: `d`)

Show items that are out of stock or have low quantities.

```
cook pantry depleted [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--all` | Show all items including those without quantities |

Items are flagged when current quantity is at or below the `low` threshold. Without a `low` threshold, heuristics are used.

### `expiring` (alias: `e`)

Show items that are expiring soon.

```
cook pantry expiring [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-d, --days <DAYS>` | Number of days to look ahead (default: 7) |
| `--include-unknown` | Include items without expiry dates |

### `recipes` (alias: `r`)

List recipes that can be made with items currently in pantry.

```
cook pantry recipes [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-p, --partial` | Include partial matches (most ingredients available) |
| `--threshold <PERCENT>` | Minimum percentage of ingredients for partial matches (default: 75) |

### `plan` (alias: `pl`)

Analyze ingredient usage across recipes to help plan pantry items.

```
cook pantry plan [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-n, --max-ingredients <N>` | Maximum number of ingredients to show (default: all needed for 100% coverage) |
| `-s, --skip <N>` | Skip the first N ingredients (default: 0) |
| `-m, --allow-missing <N>` | Allow recipes to be considered cookable even if N ingredients are missing (default: 0) |

## Configuration

The pantry inventory is defined in `pantry.conf` (TOML format), searched in:
1. `./config/pantry.conf` — local to recipe directory
2. `~/.config/cook/pantry.conf` — global configuration

When you run `cook pantry add` and no pantry file exists, one is created
at `./config/pantry.conf` automatically.

### Format

```toml
[fridge]
milk = { quantity = "500%ml", low = "200%ml", expire = "2025-09-20" }
eggs = { quantity = "12", low = "6", bought = "2025-09-10", expire = "2025-09-25" }
butter = "250%g"

[pantry]
flour = { quantity = "2%kg", low = "500%g" }
salt = "1%kg"
```

Item attributes: `quantity`, `low` (threshold), `bought` (date), `expire` (date). Simple format (`item = "quantity"`) is also supported.

## Examples

```bash
# List everything in the pantry
cook pantry list

# Add a new item
cook pantry add dairy milk --quantity "2%l" --low "500%ml"

# Update quantity after shopping
cook pantry update dairy milk --quantity "3%l"

# Remove an item
cook pantry remove dairy milk

# Show low/out-of-stock items
cook pantry depleted

# Items expiring in the next 14 days
cook pantry expiring --days 14

# Recipes you can fully make
cook pantry recipes

# Recipes with at least 60% of ingredients available
cook pantry recipes --partial --threshold 60

# Plan pantry stocking
cook pantry plan

# JSON output
cook pantry -f json list
cook pantry -f json depleted
```

## Notes

- Unit comparisons only work when units match (e.g., `g` vs `g`, not `kg` vs `g`)
- For items without units, use plain numbers (e.g., `eggs = { quantity = "6", low = "2" }`)
- `pantry update` merges new values over existing attributes — omitted flags are left unchanged
