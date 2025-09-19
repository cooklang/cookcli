# CookCLI

Command line tools for working with Cooklang recipes.

## What is CookCLI?

CookCLI provides a suite of commands to create shopping lists,
reports and maintain recipes. We've built it to be simple and useful
for automating your cooking and shopping routine with existing
UNIX command line and scripting tools. It can also function
as a webserver for your recipes, making them browsable on
any device with a web browser.

With CookCLI, you can:

* integrate your Cooklang recipes with other tools
* script meal planning and shopping
* evaluate your recipes or menus with reports
* of course cook with your terminal open

## Getting Started

First, install CookCLI using one of the methods below. CookCLI comes with a few sample recipes to play with:

```bash
$ cook seed
$ cook recipe "Neapolitan Pizza.cook"
```

This displays the recipe in a human-readable format:

```
Neapolitan Pizza

source: https://www.stadlermade.com/how-to-pizza-dough/neapolitan/
servings: 6

Ingredients:
  semolina
  Pizza Dough              (recipe: Shared/Pizza Dough)     6 balls
  flour
  semolina
  San Marzano tomato sauce                                  5 tbsp
  basil leaves
  mozzarella cheese                                         100 grams

Cookware:
  outdoor oven
  spatula

Steps:
 1. Preheat your outdoor oven so it’s around 450/500°C (842/932°F).
     [-]
 2. Prepare your pizza toppings because from now on you wanna work fast.
    Sprinkle some semolina on your work surface.
     [semolina]
 ...
```

Create a shopping list from multiple recipes:

```bash
$ cook -v shopping-list "Neapolitan Pizza.cook" "./Breakfast/Easy Pancakes.cook""
```

Or start the web server to browse your recipes:

```bash
$ cook server --open
Listening on http://127.0.0.1:9080
Serving Web UI on http://localhost:9080
Serving recipe files from: "/Users/chefalexey/recipes"
```

<img width="600" height="359" alt="recipes" src="https://github.com/user-attachments/assets/b18ec5b9-4959-49be-9784-86f73f0e34e2" />
<img width="600" height="359" alt="recipe" src="https://github.com/user-attachments/assets/e24b7b90-7bf7-4a5d-b1e0-585995a647c6" />
<img width="600" height="359" alt="shopping" src="https://github.com/user-attachments/assets/8c9036ca-a902-4f09-901c-905c6f8325a2" />
<img width="600" height="359" alt="pantry" src="https://github.com/user-attachments/assets/92d20226-eeb4-4b64-8833-ee7d9b3578a9" />


## Installation

### Download Binary

Download the latest release for your platform from the [releases page](https://github.com/cooklang/CookCLI/releases) and add it to your PATH.

### macOS

Using Homebrew:

```bash
brew tap cooklang/tap
brew install cooklang/tap/cookcli
```

### Install with Cargo

> not working at the moment, re: https://github.com/cooklang/cookcli/issues/155#issuecomment-3239646168

If you have Rust installed:

```bash
cargo install cookcli
```

### Build from Source

You'll need Rust and Node.js installed. Then:

```bash
# Clone the repository
git clone https://github.com/cooklang/CookCLI.git
cd CookCLI

# Install frontend dependencies
npm install

# Build CSS (required for web UI)
npm run build-css

# Build the CLI with web UI
cargo build --release

# Binary will be at target/release/cook
```

#### Building without Self-Update

By default, CookCLI includes a self-update feature. To build without this feature (useful for CI/CD pipelines, package managers, or environments where auto-update is not desired):

```bash
# Build without self-update feature
cargo build --release --no-default-features

# This disables the 'self-update' feature flag while keeping all other functionality
```

Use cases for disabling self-update:
* Package manager distributions (Homebrew, apt, etc.)
* CI/CD pipelines where version control is managed externally
* Corporate environments with restricted network access
* When distributing through official release channels

### Development Setup

For development with hot-reload of CSS changes:

```bash
# Install dependencies
npm install

# In one terminal, watch CSS changes
npm run watch-css

# In another terminal, run the development server
cargo run -- server ./seed

# Or use the Makefile
make dev_server  # Builds CSS and starts server
```

## Commands

CookCLI follows the UNIX philosophy: each command does one thing well.

### `cook recipe`

Parse and display recipe files. You can view them in different formats and scale quantities.

```bash
# View a recipe
cook recipe "Pasta Carbonara.cook"

# Scale a recipe to 2x
cook recipe "Pizza.cook:2"

# Output as JSON
cook recipe "Soup.cook" -f json

# Save as Markdown
cook recipe "Cake.cook" -f markdown > cake.md
```

### `cook shopping-list`

Generate shopping lists from one or more recipes. Ingredients are automatically combined and organized by store section.

```bash
# Single recipe
cook shopping-list "Dinner.cook"

# Multiple recipes with scaling
cook shopping-list "Pasta.cook:3" "Salad.cook"

# All recipes in a directory
cook shopping-list *.cook
```

### `cook server`

Run a web server to browse your recipes from any device.

```bash
# Start on localhost
cook server

# Allow access from other devices on your network
cook server --host

# Use a different port
cook server --port 8080

# Open browser immideately
cook server --open
```

### `cook search`

Find recipes by searching through ingredients, instructions, and metadata.

```bash
# Search for recipes with chicken
cook search chicken

# Find quick recipes
cook search "30 minutes"

# Search in specific directory
cook search -b ~/recipes pasta
```

### `cook import`

Import recipes from websites and convert them to Cooklang format. Requires
`OPENAI_API_KEY` environment variable set.

```bash
# Import a recipe
cook import https://www.example.com/recipe > recipe.cook

# Import without conversion
cook import https://www.example.com/recipe --skip-conversion
```

### `cook doctor`

Check your recipe collection for issues and maintain consistency.

```bash
# Validate all recipes and display parsing errors
cook doctor validate

# Check aisle configuration for shopping lists
cook doctor aisle

# Check pantry configuration
cook doctor pantry

# Run all checks
cook doctor
```

### `cook seed`

Add sample recipes to explore Cooklang features.

```bash
# Add to current directory
cook seed

# Add to specific directory
cook seed ~/my-recipes
```

### `cook report`

Generate custom outputs using templates (experimental feature).

```bash
# Generate a recipe card
cook report -t recipe-card.md.jinja recipe.cook

# Create nutrition label
cook report -t nutrition.html.jinja recipe.cook
```

### `cook pantry`

Manage your pantry inventory, track low stock items, expiring ingredients, and find recipes you can make.

```bash
# Show items that are low or out of stock
cook pantry depleted

# Check expiring items in the next 7 days
cook pantry expiring

# Find recipes you can make with available ingredients
cook pantry recipes

# Include partial matches (recipes where most ingredients are available)
cook pantry recipes --partial --threshold 60

# Output in machine-readable formats
cook pantry -f json depleted
cook pantry -f yaml expiring --days 14
```

## Documentation

Detailed documentation for each command is available in the [docs/](docs/) directory:

* [Recipe command](docs/recipe.md) - viewing and converting recipes
* [Shopping lists](docs/shopping-list.md) - creating shopping lists
* [Server](docs/server.md) - web interface
* [Search](docs/search.md) - finding recipes
* [Import](docs/import.md) - importing from websites
* [Doctor](docs/doctor.md) - validation and maintenance
* [Seed](docs/seed.md) - example recipes
* [Report](docs/report.md) - custom outputs
* [Pantry](docs/pantry.md) - inventory management and tracking

## Configuration

CookCLI looks for configuration files in:

* `./config/` - in your recipe directory (highest priority)
* `~/.config/cooklang/` - in your home directory (fallback)
* `~/Library/Application Support/cook/` - on macOS (fallback)

Configuration files:
* `aisle.conf` - Organizes ingredients by store section
* `pantry.conf` - Tracks your ingredient inventory with quantities

### Aisle Configuration (`aisle.conf`)

Organizes ingredients by store section for shopping lists. Items not in any category will appear under "Other".

```
[produce]
tomatoes|tomato
basil|basil leaves
garlic
onions

[dairy]
milk
cheese
yogurt
butter

[pantry]
flour
sugar
pasta
rice
olive oil

[meat]
chicken
beef
pork

[bakery]
bread
rolls
```

### Pantry Configuration (`pantry.conf`)

Tracks your ingredient inventory with quantities, expiration dates, and low stock thresholds. Items in your pantry are excluded from shopping lists automatically.

```toml
[freezer]
ice_cream = "1%L"
frozen_peas = "500%g"
spinach = { bought = "05.05.2024", expire = "05.06.2024", quantity = "1%kg" }

[fridge]
milk = { expire = "10.05.2024", quantity = "2%L", low = "500%ml" }
cheese = { expire = "15.05.2024" }
butter = "250%g"

[pantry]
rice = { quantity = "5%kg", low = "1%kg" }
pasta = { quantity = "1%kg", low = "200%g" }
flour = { quantity = "5%kg", low = "1%kg" }
olive_oil = { quantity = "1%L", low = "250%ml" }
salt = "1%kg"
```

Pantry items can be specified in two formats:
- Simple: `item = "quantity"`
- Detailed: `item = { quantity = "amount", expire = "date", bought = "date", low = "threshold" }`

Attributes:
- `quantity` - Current amount in stock
- `low` - Threshold for low stock warnings (same units as quantity)
- `expire` - Expiration date
- `bought` - Purchase date

Items listed in your pantry will be automatically excluded from shopping lists, helping you track what you already have at home. Use `cook pantry depleted` to see low stock items and `cook pantry expiring` to check expiration dates.

### Using Configuration Files

```bash
# Shopping list will organize by aisle and exclude pantry items
cook shopping-list "Pasta.cook"

# Check which ingredients aren't categorized
cook doctor aisle

# Use specific config directory
cook shopping-list "Recipe.cook" --aisle ./my-config/aisle.conf

# Example: Recipe calls for salt, pepper, chicken, tomatoes
# With pantry.conf containing salt and rice in your inventory:
# Shopping list will only show: pepper, chicken, tomatoes
```

## Tips

### Logging

CookCLI has different level of logging. You can pass `-v` to show info messages, `-vv` for debug and `-vvv` for trace. Use it if you want to submit bug report because it will help us to better understand what's going on.

### Scaling Recipes

Use the `:` notation to scale any recipe:

```bash
cook recipe "Pizza.cook:2"              # Double
cook shopping-list "Pasta.cook:0.5"     # Half
```

### Combining with UNIX Tools

CookCLI works great with pipes and standard tools:

```bash
# Find all recipes with chicken and create a shopping list
cook search eggs | xargs cook shopping-list

# Convert all recipes to Markdown
for r in *.cook; do
  cook recipe "$r" -f markdown > "docs/${r%.cook}.md"
done
```

## Give us a star

Why not? It will help more people discover this tool and Cooklang.

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Areas where we'd love help:

* Bug fixes and testing
* Documentation improvements
* New features
* Recipe collections
* Translations
* UI/UX improvements

## License

MIT License. See [LICENSE](LICENSE) for details.

Some source files include code from [cooklang-chef](https://github.com/Zheoni/cooklang-chef), also under MIT license.

## Links

* [Cooklang Specification](https://cooklang.org/docs/spec) - the recipe markup language
* [Cooklang Apps](https://cooklang.org/app/) - iOS and Android apps

## Support

* [Issue Tracker](https://github.com/cooklang/CookCLI/issues) - report bugs
* [Twitter](https://twitter.com/cooklang_org) - updates and news
* [Playground](https://cooklang.github.io/cooklang-rs/)
* [Discord server](https://discord.gg/fUVVvUzEEK), ask for help or say hello fellow cooks
* [Spec discussions](https://github.com/cooklang/spec/discussions), suggest a new idea or give your opinion on future development
* [Awesome Cooklang Recipes](https://github.com/cooklang/awesome-cooklang-recipes), find inspiration or share your recipes with the community.
