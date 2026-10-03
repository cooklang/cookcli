# Shopping List Command

Generate a combined shopping list from one or more recipes.

## Usage

```
cook shopping-list [OPTIONS] [RECIPES]...
```

## Arguments

| Argument | Description |
|----------|-------------|
| `[RECIPES]...` | Recipe files to include. Each can have a scaling factor using `:N` syntax (e.g., `"Pasta.cook:3"`). Glob patterns supported (e.g., `*.cook`). |

## Options

| Option | Description |
|--------|-------------|
| `-b, --base-path <PATH>` | Base directory to search for recipe files (default: current directory) |
| `-o, --output <FILE>` | Output file (format inferred from extension) |
| `-p, --plain` | Display ingredients without aisle categories |
| `-f, --format <FORMAT>` | Output format: `human` (default), `json`, `yaml`, `markdown` |
| `--pretty` | Pretty-print structured output |
| `-a, --aisle <FILE>` | Path to aisle configuration file |
| `-i, --ignore-references` | Don't expand referenced recipes |
| `--ingredients-only` | Display only ingredient names without quantities |
| `--extra <ITEM>` | Add an extra item no recipe calls for. Repeat for each item. A bare name (`"paper towels"`) has no amount; the brace form (`"eggs{12}"`, `"flour{200%g}"`) gives one. |
| `--include-optional` | Include optional ingredients (`@?chives`), marked as optional |

## Examples

```bash
# Shopping list from multiple recipes
cook shopping-list "Neapolitan Pizza" "Easy Pancakes"

# Scale individual recipes
cook shopping-list "Pizza.cook:2" "Salad.cook"

# Plain list without categories
cook shopping-list "Pizza.cook" --plain

# Export as JSON
cook shopping-list *.cook -f json -o list.json

# Names only
cook shopping-list "Cake.cook" --ingredients-only

# Use custom aisle config
cook shopping-list "Recipe.cook" -a ~/my-store.conf

# Add items no recipe calls for
cook shopping-list "Pizza.cook" --extra "paper towels" --extra "eggs{12}"

# Include optional ingredients, such as garnishes
cook shopping-list "Eggs on toast.cook" --include-optional

# From a menu file
cook shopping-list "2 Day Plan.menu"
```

## Notes

- Ingredients with the same name are automatically combined
- Items are grouped by aisle category from `aisle.conf` (use `--plain` to disable)
- Uncategorized items appear in an "other" category; run `cook doctor aisle` to find them
- Menu files (`.menu`) are supported with their own scaling
- Referenced recipes (`@./sauce{}`) are expanded into their ingredients; a
  reference leading back to a recipe already being expanded is skipped with a
  warning, so a cycle cannot inflate the quantities
- `--extra` items are merged with the recipe ingredients, so an extra sharing a
  name with one a recipe already needs adds to it, is grouped into its aisle
  category, and is subtracted from by the pantry
- [Optional ingredients](https://cooklang.org/docs/spec/#optional-ingredients-and-cookware)
  (`@?chives`) are left off the list unless `--include-optional` is given, and
  so is everything an optional recipe reference (`@?./sauce{}`) would add. When
  included, they are marked `(optional)` (`"optional": true` in JSON and YAML)
  and kept apart from any required amount of the same ingredient. Pantry stock
  counts against the required amount first
