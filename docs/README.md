The Cook CLI command line program provides a suite of tools to create shopping lists, maintain recipes, and manage your cooking workflow. We've built it to be simple and useful for automating your cooking and shopping routine with existing UNIX command line and scripting tools.

## Available Commands

* **[recipe](recipe.md)** – Parse and display recipe files in various formats
* **[shopping-list](shopping-list.md)** – Generate shopping lists from multiple recipes
* **[server](server.md)** – Run a web server to browse your recipe collection
* **[search](search.md)** – Search through your recipes by ingredient or text
* **[import](import.md)** – Import recipes from websites and convert to Cooklang
* **[doctor](doctor.md)** – Validate recipes and check for issues
* **[seed](seed.md)** – Initialize a directory with example recipes
* **[report](report.md)** – Generate custom reports using templates

## Installation

### macOS (Homebrew)
```bash
brew install cooklang/tap/cookcli
```

### Build from Source
```bash
git clone https://github.com/cooklang/cookcli.git
cd cookcli
cargo build --release
```

## Global Options

CookCLI supports several global options that apply to all commands:

### Base Path
Specify the directory containing your recipes (defaults to current directory):

```bash
# Use recipes from a specific directory
cook --base-path ~/my-recipes recipe "Pizza.cook"

# Short form
cook -b ~/my-recipes shopping-list "Pasta.cook"
```

### Logging Verbosity
Control the amount of debug information displayed:

```bash
# Normal output (default)
cook recipe "Pizza.cook"

# Info level logging (-v)
cook -v recipe "Pizza.cook"

# Debug level logging (-vv)
cook -vv shopping-list "Pasta.cook"

# Trace level logging (-vvv) - most verbose
cook -vvv doctor validate
```

The logging levels are:
* No flag: Normal output only
* `-v`: Info messages
* `-vv`: Debug messages (helpful for troubleshooting)
* `-vvv`: Trace messages (detailed parsing and processing information)

## Quick Start

Start by creating some sample recipes to explore:

```bash
# Add sample recipes to current directory
cook seed

# View a recipe
cook recipe "Neapolitan Pizza.cook"

# Create a shopping list
cook shopping-list "Neapolitan Pizza.cook" "Caesar Salad.cook"

# Start the web server
cook server

# Use recipes from another directory with debug logging
cook -b ~/recipes -vv server
```

## Philosophy

Everything in CookCLI follows these principles:

* **Everything is a file** – No databases, no lock-in. Your recipes are plain text files you control.
* **Human-readable** – All recipe files are readable without any special tools.
* **Composable** – Each command does one thing well and can be combined with other UNIX tools.
* **Offline-first** – Everything works without an internet connection.

## Recipe Files

Recipes are stored as `.cook` files using the Cooklang markup language. Here's a simple example:

```cooklang
---
title: Simple Pasta
time: 20 minutes
servings: 2
---

Bring @water{2%liters} to a boil in a large #pot.

Add @pasta{200%g} and cook for ~{10%minutes}.

Drain and mix with @olive oil{2%tbsp} and @parmesan{50%g}.
```

For a complete reference on the Cooklang syntax, see the [language specification](https://cooklang.org/docs/spec).

## Configuration

CookCLI looks for configuration files in the following locations:

* `./config/` – Configuration in your recipe directory
* `~/.config/cooklang/` – User configuration
* `/etc/cooklang/` – System-wide configuration

### Aisle Configuration

The `aisle.conf` file helps organize shopping lists by store section:

```
[produce]
tomatoes
basil
garlic

[dairy]
mozzarella
parmesan
```

## Tips and Tricks

### Scaling Recipes

Scale any recipe using the `:` notation:

```bash
# Double a recipe
cook recipe "Pizza.cook:2"

# Scale to 3x for shopping list
cook shopping-list "Pasta.cook:3"
```

### Combining with UNIX Tools

CookCLI works great with standard UNIX tools:

```bash
# Find all recipes with chicken
cook search chicken | head -5

# Create shopping list for all pasta recipes
cook shopping-list $(ls *Pasta*.cook)

# Export recipe to JSON for processing
cook recipe "Pizza.cook" -f json | jq '.ingredients'
```

### Quick Recipe Validation

Check all your recipes for errors:

```bash
cook doctor validate

# In CI/CD pipelines
cook doctor validate --strict
```

## Getting Help

Each command has built-in help:

```bash
cook --help
cook recipe --help
cook shopping-list --help
```

## License

CookCLI is open source software licensed under the [MIT License](https://github.com/cooklang/cookcli/blob/main/LICENSE).
