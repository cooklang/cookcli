# Recipe Command

The `recipe` command parses and displays Cooklang recipe files. It's your primary tool for viewing recipes, validating syntax, and converting between formats.

## Basic Usage

```bash
cook recipe "Neapolitan Pizza.cook"
```

This displays the recipe in a human-readable format with ingredients, steps, and metadata clearly organized.

## Menu Files

CookCLI also supports `.menu` files for meal planning. Menu files can reference multiple recipes and organize them by meals or days:

```bash
# View a menu file
cook recipe "weekly_plan.menu"

# Scale entire menu (scales all referenced recipes)
cook recipe "weekly_plan.menu:2"
```

Menu files use the same scaling notation as regular recipes, and the scaling applies to all recipe references within the menu.

## Reading Recipes

The simplest way to view a recipe:

```bash
# View a recipe file
cook recipe "Pasta Carbonara.cook"

# Read from stdin
cat recipe.cook | cook recipe

# View with explicit path
cook recipe ./recipes/italian/Pizza.cook
```

## Scaling Recipes

Scale recipes on the fly using the `:` notation or the `--scale` flag:

```bash
# Double a recipe
cook recipe "Pizza.cook:2"

# Scale to 1.5x
cook recipe "Cake.cook" --scale 1.5

# Quarter a recipe
cook recipe "Soup.cook:0.25"
```

All ingredient quantities are automatically adjusted:

```
Original:                    Scaled x2:
flour         500 g    →     flour         1 kg
water         300 ml   →     water         600 ml
```

## Output Formats

Export recipes in different formats for various uses:

### Human-Readable (Default)

```bash
cook recipe "Pizza.cook"
```

Output:
```
Metadata:
    servings: 4
    time: 2 hours

Ingredients:
    flour                         500 g
    water                         300 ml
    salt                          10 g
    yeast                         5 g

Steps:
    1. Mix flour, water, salt, and yeast...
    2. Knead for 10 minutes...
    3. Let rise for 1 hour...
```

### JSON Format

Perfect for processing with other tools:

```bash
cook recipe "Pizza.cook" -f json | jq '.ingredients'
```

Output:
```json
[
  {
    "name": "flour",
    "quantity": { "value": 500, "unit": "g" }
  },
  {
    "name": "water",
    "quantity": { "value": 300, "unit": "ml" }
  }
]
```

### YAML Format

```bash
cook recipe "Pizza.cook" -f yaml
```

### Markdown Format

Great for documentation or sharing:

```bash
cook recipe "Pizza.cook" -f markdown > recipe.md
```

### Cooklang Format

Regenerate clean Cooklang markup:

```bash
cook recipe "Pizza.cook" -f cooklang
```

## Saving Output

Write the output to a file:

```bash
# Save as JSON
cook recipe "Pizza.cook" -f json -o pizza.json

# Save as Markdown
cook recipe "Pizza.cook" -f markdown -o pizza.md

# Format is inferred from extension
cook recipe "Pizza.cook" -o pizza.yaml
```

## Pretty Printing

For JSON and YAML outputs, use `--pretty` for formatted output:

```bash
cook recipe "Pizza.cook" -f json --pretty
```

## Recipe Discovery

CookCLI can find recipes by name without the full path:

```bash
# Searches in current directory and subdirectories
cook recipe "Pizza"

# Automatically adds .cook extension
cook recipe "Pasta Carbonara"

# Searches in recipe directories
cook recipe "Neapolitan Pizza"
```

## Advanced Examples

### Recipe Analysis Pipeline

Combine with UNIX tools for analysis:

```bash
# Count total ingredients across all recipes
for recipe in *.cook; do
  cook recipe "$recipe" -f json
done | jq -s '[.[].ingredients[].name] | group_by(.) | map({name: .[0], count: length})'

# Find recipes by cooking time
cook recipe "*.cook" -f json | jq 'select(.metadata.time <= 30)'
```

### Batch Processing

Process multiple recipes:

```bash
# Convert all recipes to Markdown
for recipe in *.cook; do
  name=$(basename "$recipe" .cook)
  cook recipe "$recipe" -f markdown -o "docs/${name}.md"
done

# Validate all recipes
for recipe in *.cook; do
  echo "Checking $recipe..."
  cook recipe "$recipe" > /dev/null || echo "Error in $recipe"
done
```

### Recipe Comparison

Compare scaled versions:

```bash
# Compare original and doubled recipe
diff <(cook recipe "Pizza.cook") <(cook recipe "Pizza.cook:2")

# See how scaling affects specific ingredients
cook recipe "Cake.cook" -f json | jq '.ingredients'
cook recipe "Cake.cook:3" -f json | jq '.ingredients'
```

## Tips

### Quick Validation

Use the recipe command to validate syntax:

```bash
# Validates and displays any errors
cook recipe "new-recipe.cook"

# Silent validation (returns exit code)
cook recipe "recipe.cook" > /dev/null 2>&1 && echo "Valid" || echo "Invalid"
```

### Template Generation

Use existing recipes as templates:

```bash
# Get the structure of a recipe
cook recipe "Pizza.cook" -f cooklang > template.cook

# Extract just ingredients
cook recipe "Pizza.cook" -f json | jq -r '.ingredients[].name'
```

### Integration with Editors

Set up your editor to validate on save:

```vim
" Vim - Add to .vimrc
autocmd BufWritePost *.cook !cook recipe %
```

## Common Issues

### Recipe Not Found

If a recipe isn't found by name, try:

1. Using the full path
2. Adding the `.cook` extension
3. Checking you're in the right directory

### Scaling Limitations

* Some ingredients shouldn't be scaled linearly (like salt or spices)
* Consider adding metadata hints for non-linear scaling
* Always review scaled recipes before cooking

### Format Detection

* Output format is inferred from file extension
* Use `-f` flag to override detection
* Default is human-readable format

## See Also

* [Shopping List](shopping-list.md) – Create shopping lists from recipes
* [Doctor](doctor.md) – Validate recipe syntax and references
* [Search](search.md) – Find recipes by content