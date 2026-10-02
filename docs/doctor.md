# Doctor Command

Analyze your recipe collection for issues and improvements.

## Usage

```
cook doctor [OPTIONS] [COMMAND]
```

## Subcommands

### `validate`

Validate all recipes for syntax errors and warnings.

```
cook doctor validate [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-b, --base-path <PATH>` | Directory to scan for recipe files (default: current directory) |
| `--strict` | Exit with error code 1 if any issues are found (useful for CI/CD) |

Checks for: syntax errors, warnings, missing recipe references, invalid units or quantities.

A timer quantity is checked too. It has to be a number with a time unit —
`minutes`, `hours`, `seconds`, `days`, and the usual short forms — so
`Cook for ~{a few%minutes}.` is reported, and so is a unit that is not one of
those. `~{40%minutes}`, `~{1.5%hours}` and a range such as `~{10-20%minutes}`
are accepted. An ingredient may still be text (`@salt{to taste}`). A number
with no unit (`~{30}`) is a warning. A named timer with no quantity (`~dough`)
is left as written.

### `aisle`

Check for ingredients missing from your aisle configuration.

```
cook doctor aisle [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-b, --base-path <PATH>` | Directory to scan for recipe files (default: current directory) |
| `--show-recipes` | List the recipes each uncategorized ingredient comes from |

Finds ingredients not assigned to any store section in `aisle.conf`. Each one
is reported with how many recipes use it:

```
3 ingredients not found in aisle configuration:
  - cumin powder (1 recipe)
  - cummin (2 recipes)
  - ground cinnamon (12 recipes)
```

An ingredient used by one recipe where the rest of the collection spells it
another way is usually a typo rather than a missing aisle entry. Pass
`--show-recipes` to name the files to go and fix:

```
3 ingredients not found in aisle configuration:
  - cumin powder (1 recipe)
      Curries/Rogan Josh.cook
  - cummin (2 recipes)
      Curries/Dal.cook
      Soups/Harira.cook
```

### `pantry`

Check which recipe ingredients are in your pantry inventory.

```
cook doctor pantry [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-b, --base-path <PATH>` | Directory to scan for recipe files (default: current directory) |
| `--show-recipes` | List the recipes each pantry ingredient comes from |

Shows which ingredients are already tracked in `pantry.conf`, with how many
recipes use each. `--show-recipes` names them, so you can see which of your
recipes an item in stock is keeping off the shopping list.

## Examples

```bash
# Run all checks
cook doctor

# Validate recipes
cook doctor validate

# Strict mode for CI/CD
cook doctor validate --strict

# Check for uncategorized ingredients
cook doctor aisle

# ...and find the recipes that misspell them
cook doctor aisle --show-recipes

# Check pantry coverage
cook doctor pantry
```
