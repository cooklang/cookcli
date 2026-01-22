# Shopping List Command

The `shopping-list` command creates organized shopping lists from one or more recipes. It automatically combines ingredients, converts units (some day!), and groups items by store section.

## Basic Usage

```bash
cook shopping-list "Pasta.cook" "Salad.cook"
```

This creates a combined shopping list with all ingredients from both recipes.

## Creating Shopping Lists

### Single Recipe

```bash
cook shopping-list "Breakfast/Easy Pancakes" "Neapolitan Pizza"
```

Output:
```
[dried herbs and spices]
salt                     24.6 g
sea salt                 1 pinch

[milk and dairy]
eggs                     3

[oils and dressings]
oil
olive oil                1 drizzle

[other]
San Marzano tomato sauce 5 tbsp
basil leaves
mozzarella cheese        100 grams
semolina
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

## Menu Files

Create shopping lists from [`.menu` files](/docs/use-cases/meal-planning/) that organize multiple recipes:

```bash
# Generate shopping list from weekly menu
cook shopping-list "2 Day Plan.menu"

# Scale entire menu for more people
cook shopping-list "2 Day Plan.menu:2"

# Combine menu with individual recipes
cook shopping-list "2 Day Plan.menu" "Extra Snacks.cook"
```

Menu files can contain recipe references with their own scaling, and the menu-level scaling multiplies with individual recipe scales.

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

## Pantry Configuration

Track your ingredient inventory and automatically exclude items you already have from shopping lists using `pantry.conf`:

```toml
# Default location: ./config/pantry.conf
[freezer]
ice_cream = "1%L"
frozen_peas = "500%g"
spinach = { bought = "05.05.2024", expire = "05.06.2024", quantity = "1%kg" }

[fridge]
milk = { expire = "10.05.2024", quantity = "2%L" }
cheese = { expire = "15.05.2024" }
butter = "250%g"

[pantry]
rice = "5%kg"
pasta = "1%kg"
flour = "5%kg"
salt = "1%kg"
olive_oil = "1%L"
```

### How It Works

Items listed in your pantry are automatically excluded from shopping lists:

```bash
# Recipe calls for: flour, eggs, milk, sugar
# Pantry has: flour (5kg), milk (2L)
# Shopping list shows only: eggs, sugar
cook shopping-list "Cake.cook"
```

Enable logging to see what's excluded with pantry:

```bash
cook -v shopping-list "Cake.cook"
```

### Pantry Item Formats

Two ways to specify items:

1. **Simple format**: `item = "quantity"`
   ```toml
   rice = "5%kg"
   ```

2. **Detailed format**: Track expiration and purchase dates
   ```toml
   milk = { quantity = "2%L", expire = "10.05.2024", bought = "05.05.2024" }
   ```

### Using Custom Pantry Configuration

```bash
cook shopping-list "Recipe.cook" --pantry ~/my-pantry.conf
```

## Recipe References

Shopping lists handle recipe references (includes) automatically:

```cooklang
# Main.cook
Include @./Pizza Dough{}

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
cook shopping-list Menu/*.cook -o list.txt

# Save as JSON
cook shopping-list Menu/*.cook -f json -o list.json

# Format inferred from extension
cook shopping-list Menu/*.cook -o shopping.yaml
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
  "Appetizer.cook:$scale" \
  "Main Course.cook:$scale" \
  "Side Dish.cook:$scale" \
  "Dessert.cook:$scale"
```


## Smart Shopping

### Batch Cooking

Plan bulk preparation:

```bash
# Make 10 portions for freezing
cook shopping-list "Freezer Meals/*.cook:10" \
  -o bulk-cooking-list.txt
```

## Common Issues

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
