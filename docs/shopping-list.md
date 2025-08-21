# Shopping List Command

The `shopping-list` command creates organized shopping lists from one or more recipes. It automatically combines ingredients, converts units, and groups items by store section.

## Basic Usage

```bash
cook shopping-list "Pasta.cook" "Salad.cook"
```

This creates a combined shopping list with all ingredients from both recipes.

## Creating Shopping Lists

### Single Recipe

```bash
cook shopping-list "Neapolitan Pizza.cook"
```

Output:
```
PRODUCE
    garlic                        3 cloves
    fresh basil                   18 leaves

DAIRY
    mozzarella                    3 packs
    
PANTRY
    tipo zero flour               820 g
    salt                          25 g
```

### Multiple Recipes

Combine ingredients from several recipes:

```bash
cook shopping-list "Pizza.cook" "Caesar Salad.cook" "Tiramisu.cook"

# Using wildcards
cook shopping-list *.cook

# Specific pattern
cook shopping-list Dinner/*.cook Desserts/*.cook
```

## Scaling Recipes

Scale individual recipes in your shopping list:

```bash
# Double the pizza, regular salad
cook shopping-list "Pizza.cook:2" "Salad.cook"

# Different scaling for each
cook shopping-list "Pasta.cook:3" "Bread.cook:0.5" "Soup.cook:2"

# Dinner party for 8
cook shopping-list "Main Course.cook:2" "Side Dish.cook:2" "Dessert.cook:2"
```

## Smart Ingredient Combining

CookCLI automatically combines ingredients with the same name:

```
Recipe 1: flour 500 g
Recipe 2: flour 300 g
→ Shopping list: flour 800 g

Recipe 1: milk 200 ml
Recipe 2: milk 0.5 liters
→ Shopping list: milk 700 ml
```

## Output Formats

### Human-Readable (Default)

Organized by store section:

```bash
cook shopping-list "Pizza.cook" "Pasta.cook"
```

### Plain List

Without categories:

```bash
cook shopping-list "Pizza.cook" --plain
```

Output:
```
flour                         1 kg
tomatoes                      500 g
mozzarella                    200 g
olive oil                     50 ml
```

### JSON Format

For integration with other tools:

```bash
cook shopping-list "Pizza.cook" -f json
```

```json
{
  "categories": {
    "produce": [
      {"name": "tomatoes", "quantity": "500 g"}
    ],
    "dairy": [
      {"name": "mozzarella", "quantity": "200 g"}
    ]
  }
}
```

### YAML Format

```bash
cook shopping-list "Pizza.cook" -f yaml
```

### Markdown Format

Perfect for sharing or printing:

```bash
cook shopping-list "Menu/*.cook" -f markdown > shopping.md
```

## Aisle Configuration

Organize items by store section using `aisle.conf`:

```bash
# Default location: ./config/aisle.conf
[produce]
tomatoes
onions
garlic
basil

[dairy]
milk
cheese
yogurt
mozzarella

[meat]
chicken
beef
pork

[pantry]
flour
sugar
salt
pasta
```

### Using Custom Aisle Configuration

```bash
cook shopping-list "Recipe.cook" --aisle ~/my-store-layout.conf
```

### Checking Missing Aisles

Find ingredients not in your aisle configuration:

```bash
cook doctor aisle
```

## Recipe References

Shopping lists handle recipe references (includes) automatically:

```cooklang
# Main.cook
Include @Pizza Dough.cook

Add @tomato sauce{200%ml} and @mozzarella{150%g}.
```

```bash
# Includes ingredients from both Main.cook and Pizza Dough.cook
cook shopping-list "Main.cook"
```

To ignore references:

```bash
cook shopping-list "Main.cook" --ignore-references
```

## Saving Lists

### To File

```bash
# Save as text
cook shopping-list "Menu/*.cook" -o list.txt

# Save as JSON
cook shopping-list "Menu/*.cook" -f json -o list.json

# Format inferred from extension
cook shopping-list "Menu/*.cook" -o shopping.yaml
```

### Ingredients Only

Just list ingredient names without quantities:

```bash
cook shopping-list "Recipe.cook" --ingredients-only
```

Output:
```
flour
eggs
milk
sugar
```

## Advanced Usage

### Weekly Meal Planning

Create a shopping list for the week:

```bash
# Week's menu
cook shopping-list \
  Monday/Pasta.cook:2 \
  Tuesday/Stir-fry.cook:2 \
  Wednesday/Pizza.cook:3 \
  Thursday/Soup.cook:4 \
  Friday/Fish.cook:2 \
  -o weekly-shopping.txt
```

### Party Planning

Scale recipes for large gatherings:

```bash
# Party for 20 people (recipes serve 4)
scale=5
cook shopping-list \
  "Appetizer.cook@$scale" \
  "Main Course.cook@$scale" \
  "Side Dish.cook@$scale" \
  "Dessert.cook@$scale"
```

### Integration with Task Apps

Export to task management apps:

```bash
# Create checklist in Markdown
cook shopping-list "Menu/*.cook" --plain | \
  awk '{print "- [ ] " $0}' > checklist.md

# Send to todo app
cook shopping-list "Menu/*.cook" --ingredients-only | \
  xargs -I {} todo add "Buy {}"
```

### Price Estimation

Combine with pricing data:

```bash
# prices.json: {"flour": 2.50, "eggs": 3.00, ...}

cook shopping-list "Menu/*.cook" -f json | \
  jq --slurpfile prices prices.json '
    .ingredients | map({
      name: .name,
      quantity: .quantity,
      price: $prices[0][.name] // 0
    }) | 
    {items: ., total: map(.price) | add}
  '
```

## Smart Shopping

### By Store

Separate lists for different stores:

```bash
# Farmer's market items
cook shopping-list "Menu/*.cook" -f json | \
  jq '.categories.produce'

# Regular grocery items
cook shopping-list "Menu/*.cook" -f json | \
  jq 'del(.categories.produce)'
```

### Batch Cooking

Plan bulk preparation:

```bash
# Make 10 portions for freezing
cook shopping-list "Freezer Meals/*.cook:10" \
  -o bulk-cooking-list.txt
```

## Tips and Tricks

### Quick Lists

```bash
# Alias for common combinations
alias weekly='cook shopping-list Weekend/*.cook'
alias party='cook shopping-list Party/*.cook:5'
```

### List Comparison

Compare shopping lists:

```bash
# What's different this week?
diff \
  <(cook shopping-list "Last Week/*.cook" --plain | sort) \
  <(cook shopping-list "This Week/*.cook" --plain | sort)
```

### Inventory Check

```bash
# Generate list, then manually mark what you have
cook shopping-list "Menu/*.cook" --plain | \
  awk '{print "[ ] " $0}' > checklist.txt
```

## Common Issues

### Unit Conversion

CookCLI automatically converts compatible units:
* 1000 ml → 1 liter
* 1000 g → 1 kg

Incompatible units are listed separately:
* "tomatoes: 3 cans, 500 g" (can't combine count with weight)

### Missing Aisle Categories

Items not in `aisle.conf` appear in an "Other" category. Run `cook doctor aisle` to find and fix these.

### Recipe Not Found

Use full paths or ensure recipes are in the current directory:

```bash
# Explicit path
cook shopping-list ~/recipes/Pizza.cook

# Change directory first
cd ~/recipes && cook shopping-list Pizza.cook
```

## See Also

* [Recipe](recipe.md) – View and scale individual recipes
* [Doctor](doctor.md) – Check aisle configuration
* [Server](server.md) – Browse recipes and create shopping lists via web interface