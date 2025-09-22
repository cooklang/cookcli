# Search Command

The `search` command helps you find recipes quickly by searching through titles, ingredients, instructions, and metadata. It's perfect for answering "what can I cook with what I have?" or finding that recipe you remember but can't locate.

## Basic Usage

```bash
cook search chicken
```

This searches all recipes for the word "chicken" and returns matching recipes sorted by relevance.

## Search Basics

### Single Term Search

```bash
cook search tomato
```

Output:
```
Pasta with Tomato Sauce.cook
Tomato Soup.cook
Caprese Salad.cook
Pizza Margherita.cook
```

### Multiple Terms

All terms must match (AND logic):

```bash
cook search chicken rice
# Finds recipes containing both "chicken" AND "rice"
```

### Phrase Search

Use quotes for exact phrases:

```bash
cook search "olive oil"
# Matches the exact phrase "olive oil"

cook search "slow cooker"
# Finds recipes mentioning "slow cooker" together
```

## What Gets Searched

The search looks through:

* **Recipe titles** – From filename or title metadata
* **Ingredients** – All ingredient names and quantities  
* **Instructions** – The complete cooking steps
* **Metadata** – Tags, categories, cuisine, etc.
* **Notes** – Any notes or comments in the recipe

## Search Options

### Specify Directory

Search in a specific directory:

```bash
# Search in specific folder
cook search -b ~/recipes chicken

# Search in Italian recipes only
cook search -b ~/recipes/italian pasta
```

### Case Sensitivity

Searches are case-insensitive by default:

```bash
cook search Chicken  # Same as...
cook search chicken  # ...and...
cook search CHICKEN  # ...this
```

## Practical Examples

### Finding Recipes by Ingredient

What can I make with what I have?

```bash
# What can I make with chicken?
cook search chicken

# Recipes with both chicken and mushrooms
cook search chicken mushrooms

# Vegetarian recipes (assuming you tag them)
cook search vegetarian

# Quick recipes
cook search "30 minutes"
cook search quick
```

### Dietary Restrictions

```bash
# Gluten-free recipes (if tagged)
cook search gluten-free

# Vegan recipes
cook search vegan

# Dairy-free
cook search dairy-free
```

### By Cooking Method

```bash
cook search grilled
cook search "slow cooker"
cook search "instant pot"
cook search baked
cook search "no cook"
```

### By Cuisine

```bash
cook search italian
cook search mexican
cook search thai
cook search indian
```

## Advanced Usage

### Search and Cook Workflow

Find a recipe and immediately view it:

```bash
# Find and select
recipe=$(cook search chicken | head -1)
cook recipe "$recipe"

# Interactive selection with fzf
cook search pasta | fzf | xargs cook recipe
```

### Search and Shopping List

Create shopping list from search results:

```bash
# Shopping list for all chicken recipes
cook search chicken | xargs cook shopping-list

# First 3 pasta recipes
cook search pasta | head -3 | xargs cook shopping-list
```

### Inventory-Based Cooking

```bash
# What can I make with ingredients I have?
ingredients="chicken tomato basil"
for ingredient in $ingredients; do
  cook search $ingredient
done | sort | uniq -c | sort -rn | head

# This shows recipes ranked by how many of your ingredients they use
```

## Search Patterns

### Exclusion Patterns

While the search command doesn't support negative queries directly, you can filter results:

```bash
# Chicken recipes without rice
cook search chicken | grep -v -i rice

# Pasta without tomato
cook search pasta | grep -v -i tomato
```

### Complex Queries

Combine with shell tools for complex searches:

```bash
# Recipes with chicken OR beef
{ cook search chicken; cook search beef; } | sort | uniq

# Quick recipes (under 30 min) with chicken
cook search chicken | while read recipe; do
  cook recipe "$recipe" -f json | \
    jq -r 'select(.metadata.time <= 30) | input_filename'
done

# Recipes matching multiple criteria
comm -12 \
  <(cook search italian | sort) \
  <(cook search vegetarian | sort)
```

## Integration Examples

### Menu Planning

```bash
# Weekly menu planner
days="monday tuesday wednesday thursday friday"
for day in $days; do
  echo "$day:"
  cook search $day | head -1
done

# Random meal selector
cook search dinner | shuf -n 1
```

### Recipe Discovery

```bash
# Recipe of the day
cook search "" | shuf -n 1 | xargs cook recipe

# Explore new cuisines
cuisines=(thai indian moroccan korean)
cuisine=${cuisines[$RANDOM % ${#cuisines[@]}]}
echo "Try something $cuisine today:"
cook search $cuisine | head -3
```

### Export Search Results

```bash
# Create a cookbook of chicken recipes
cook search chicken | while read recipe; do
  cook recipe "$recipe" -f markdown
done > chicken-cookbook.md

# Generate recipe index
cook search "" | sort > recipe-index.txt
```

## Tips and Tricks

### Fuzzy Search Workflow

Use with fuzzy finders for interactive selection:

```bash
# With fzf
cook search "" | fzf --preview 'cook recipe {}' | xargs cook recipe

# With dmenu (Linux)
cook search "" | dmenu | xargs cook recipe

# With rofi (Linux)
cook search "" | rofi -dmenu | xargs cook recipe
```


## Optimizing Search

### Organizing for Better Search

Structure your recipes for better searchability:

```cooklang
---
title: Quick Chicken Stir-Fry
tags: quick, weeknight, asian, chicken
time: 20 minutes
cuisine: Chinese
---
```

## Troubleshooting

### No Results Found

* Check spelling
* Try simpler terms
* Use single words instead of phrases
* Verify you're in the right directory

### Too Many Results

* Add more search terms
* Use more specific terms
* Search in subdirectories
* Filter results with grep

### Performance

For very large collections:
* Organize recipes into folders
* Search specific directories
* Consider using external search tools like `ripgrep`
