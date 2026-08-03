# Seed Command

Initialize a directory with example Cooklang recipes.

## Usage

```
cook seed [OPTIONS] [DIR]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `[DIR]` | Directory to create example recipes in (default: current directory). Created if it doesn't exist. |

## Examples

```bash
# Add examples to current directory
cook seed ./

# Create examples in a specific directory
cook seed ~/my-recipes
```

## Notes

- Creates example recipes demonstrating Cooklang features (ingredients, cookware, timers, metadata, recipe references)
- Includes folder structure and an `aisle.conf` configuration file
- Populates the target directory directly, so run it in an empty or dedicated directory rather than an existing one you care about
