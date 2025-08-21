# Seed Command

The `seed` command populates a directory with example Cooklang recipes. It's perfect for getting started, learning the syntax, or setting up a demo.

## Basic Usage

```bash
cook seed
```

This creates a collection of example recipes in the current directory.

## What Gets Created

The seed command creates:

* **Example recipes** – Various cuisines and complexity levels
* **Organized folders** – Structured by meal type
* **Configuration files** – Including `aisle.conf` for shopping lists
* **README** – Documentation about the recipes

```
.
├── Breakfast/
│   ├── Easy Pancakes.cook
│   └── Mexican Style Burrito.cook
├── Dinners/
│   ├── Neapolitan Pizza.cook
│   ├── Pasta Carbonara.cook
│   └── Roast Chicken.cook
├── Shared/
│   ├── Pizza Dough.cook
│   ├── Tomato Sauce.cook
│   └── Guacamole.cook
├── config/
│   └── aisle.conf
└── README.md
```

## Seeding Options

### Current Directory

```bash
cook seed
# Creates recipes in current directory
```

### Specific Directory

```bash
cook seed ~/my-recipes
# Creates recipes in ~/my-recipes

# Or
cook seed recipes/examples
# Creates recipes in recipes/examples
```

### Creating Directory

If the directory doesn't exist, it will be created:

```bash
cook seed ~/new-cookbook
# Creates ~/new-cookbook and adds recipes
```

## Example Recipes

The seed collection includes diverse recipes demonstrating Cooklang features:

### Basic Recipes

Simple recipes for learning:

```cooklang
>> title: Easy Pancakes
>> servings: 4

Mix @flour{2%cups} with @milk{2%cups}.
Add @eggs{2} and whisk until smooth.
Cook on #pan for ~{2%minutes} per side.
```

### Advanced Features

Recipes showcasing advanced features:

* **Timers** – `~{30%minutes}`
* **Equipment** – `#oven`, `#pot{large}`
* **References** – `@Pizza Dough.cook`
* **Metadata** – Servings, time, tags
* **Notes** – Comments and variations

### Recipe Categories

* **Breakfast** – Quick morning meals
* **Dinners** – Main courses
* **Desserts** – Sweet treats
* **Shared** – Components used in multiple recipes
* **Snacks** – Quick bites

## Learning from Examples

### Understanding Syntax

Study the seed recipes to learn:

```bash
# View a simple recipe
cook recipe "Easy Pancakes"

# See how scaling works
cook recipe "Neapolitan Pizza:2"

# Check ingredient formatting
cook recipe "Pasta Carbonara" -f json | jq '.ingredients'
```

### Recipe Structure

Examine different patterns:

```bash
# Simple linear recipe
cat "Easy Pancakes.cook"

# Complex with sub-recipes
cat "Neapolitan Pizza.cook"

# Component recipe
cat "Shared/Pizza Dough.cook"
```

## Practical Uses

### Quick Start

For new users:

```bash
# Get started with Cooklang
mkdir my-cookbook && cd my-cookbook
cook seed
cook server --open
```

### Demo Environment

For presentations or workshops:

```bash
# Create demo directory
cook seed /tmp/cooklang-demo
cd /tmp/cooklang-demo

# Show features
cook recipe "Neapolitan Pizza"
cook shopping-list *.cook
cook server
```

### Testing and Development

For testing CookCLI features:

```bash
# Test environment
cook seed /tmp/test-recipes
cd /tmp/test-recipes

# Test validation
cook doctor validate

# Test shopping lists
cook shopping-list Breakfast/*.cook

# Test search
cook search tomato
```

### Template Library

Use as templates for your recipes:

```bash
# Create templates directory
cook seed ~/recipe-templates

# Copy and modify
cp ~/recipe-templates/Dinners/Pasta.cook ~/recipes/my-pasta.cook
vim ~/recipes/my-pasta.cook
```

## Customization

### After Seeding

Customize the seed recipes:

```bash
# Add your recipes
cook seed
vim "My Special Recipe.cook"

# Modify examples
vim "Neapolitan Pizza.cook"
# Change ingredients to your preference

# Remove unwanted examples
rm -rf Breakfast/
```

### Mixing with Existing Recipes

```bash
# Add examples to existing collection
cd ~/my-recipes
mkdir examples
cook seed examples/

# Keep examples separate
recipes/
├── my-recipes/
│   └── ...
└── examples/     # Seed recipes here
    └── ...
```

### Creating Your Own Seed

Build a custom seed collection:

```bash
# Create your template collection
mkdir my-seed
cp favorite-*.cook my-seed/
cp config/aisle.conf my-seed/

# Use as template
cp -r my-seed/* new-cookbook/
```

## Integration Ideas

### Recipe Development

Use seed recipes as a starting point:

```bash
# Start with seed recipe
cook seed temp
cp temp/"Pasta Carbonara.cook" .

# Modify to create variation
vim "Pasta Carbonara.cook"
# Change to "Vegetarian Carbonara"
```

### Learning Exercises

Create learning materials:

```bash
# Workshop materials
cook seed workshop/exercises
cook seed workshop/solutions

# Student assignments
for student in alice bob charlie; do
  cook seed "students/$student"
done
```

### Automated Testing

Use in test suites:

```bash
#!/bin/bash
# test-suite.sh

# Setup
TESTDIR=$(mktemp -d)
cook seed "$TESTDIR"
cd "$TESTDIR"

# Run tests
echo "Testing recipe parsing..."
for recipe in **/*.cook; do
  cook recipe "$recipe" > /dev/null || echo "Failed: $recipe"
done

echo "Testing shopping lists..."
cook shopping-list **/*.cook > /dev/null

echo "Testing validation..."
cook doctor validate --strict

# Cleanup
rm -rf "$TESTDIR"
```

## Tips and Tricks

### Quick Exploration

```bash
# See what's available
cook seed /tmp/explore && ls -la /tmp/explore

# Browse all recipes
cook seed /tmp/browse && cd /tmp/browse && cook server --open
```

### Reset Recipes

```bash
# Clean slate
rm -rf *.cook config/
cook seed
```

### Version Control

```bash
# Track modifications to seed recipes
cook seed
git init
git add .
git commit -m "Initial seed recipes"

# Now modify and track changes
vim "Pizza.cook"
git diff
```

## Common Patterns

### Try Before You Buy

Test CookCLI features without affecting your recipes:

```bash
# Temporary playground
cd $(mktemp -d)
cook seed

# Experiment freely
cook recipe "*.cook"
cook shopping-list "*.cook"
cook doctor validate
```

### Documentation Examples

Generate documentation with real examples:

```bash
# Create examples for docs
cook seed docs/examples

# Reference in documentation
echo "See examples in docs/examples/" >> README.md
```

### Onboarding Script

```bash
#!/bin/bash
# setup-cooklang.sh

echo "Welcome to Cooklang!"
echo "Creating your cookbook..."

mkdir -p ~/cookbook
cd ~/cookbook
cook seed

echo "Setup complete!"
echo "Try these commands:"
echo "  cook recipe 'Neapolitan Pizza'"
echo "  cook shopping-list *.cook"
echo "  cook server --open"
```

## Troubleshooting

### Directory Already Has Files

The seed command won't overwrite existing files:

```bash
# Safe to run in existing directory
cook seed  # Only adds missing files
```

### Permission Issues

```bash
# If permission denied
sudo cook seed /protected/directory

# Or create directory first
mkdir ~/recipes
cook seed ~/recipes
```

### Partial Seeding

If seeding is interrupted:

```bash
# Just run again
cook seed
# It will complete missing files
```

## See Also

* [Recipe](recipe.md) – View the seeded recipes
* [Shopping List](shopping-list.md) – Create lists from seed recipes
* [Server](server.md) – Browse seed recipes via web