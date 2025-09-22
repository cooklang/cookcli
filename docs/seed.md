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
---
title: Easy Pancakes
servings: 4
---

Mix @flour{2%cups} with @milk{2%cups}.

Add @eggs{2} and whisk until smooth.

Cook on #pan for ~{2%minutes} per side.
```

### Advanced Features

Recipes showcasing advanced features:

* **Timers** – `~{30%minutes}`
* **Equipment** – `#oven`, `#pot{large}`
* **References** – `@./Pizza Dough.cook{}`
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
cook recipe "Breakfast/Easy Pancakes"

# See how scaling works
cook recipe "Neapolitan Pizza:2"

# Check ingredient formatting
cook recipe "Sicilian-style Scottadito Lamb Chops" -f json | jq '.ingredients'
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
