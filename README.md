# CookCLI

Command line tools for working with Cooklang recipes.

## What is CookCLI?

CookCLI provides a suite of tools to create shopping lists and maintain recipes. We've built it to be simple and useful for automating your cooking and shopping routine with existing UNIX command line and scripting tools. It can also function as a webserver for your recipes, making them browsable on any device with a web browser.

With CookCLI, you can:

* simplify your personal recipe management
* streamline your shopping routine  
* make cooking more fun

## Getting Started

First, install CookCLI using one of the methods below. Then create some sample recipes to explore:

```bash
$ cook seed
$ cook recipe "Neapolitan Pizza.cook"
```

This displays the recipe in a human-readable format:

```
Metadata:
    servings: 6

Ingredients:
    chopped tomato                3 cans
    dried oregano                 3 tbsp
    fresh basil                   18 leaves
    fresh yeast                   1.6 g
    garlic                        3 cloves
    mozzarella                    3 packs
    parma ham                     3 packs
    salt                          25 g
    tipo zero flour               820 g
    water                         530 ml

Steps:
     1. Make 6 pizza balls using tipo zero flour, water, salt and fresh yeast. Put in a fridge for 2 days.
        [fresh yeast: 1.6 g; salt: 25 g; tipo zero flour: 820 g; water: 530 ml]
     2. Set oven to max temperature and heat pizza stone for about 40 minutes.
        [–]
     3. Make some tomato sauce with chopped tomato and garlic and dried oregano. Put on a pan and leave for 15 minutes occasionally stirring.
        [chopped tomato: 3 cans; dried oregano: 3 tbsp; garlic: 3 cloves]
     4. Make pizzas putting some tomato sauce with spoon on top of flattened dough. Add fresh basil, parma ham and mozzarella.
        [fresh basil: 18 leaves; mozzarella: 3 packs; parma ham: 3 packs]
     5. Put in an oven for 4 minutes.
        [–]
```

Create a shopping list from multiple recipes:

```bash
$ cook shopping-list "Neapolitan Pizza.cook" "Caesar Salad.cook"
```

Or start the web server to browse your recipes:

```bash
$ cook server
Started server on http://127.0.0.1:9080, serving cook files from current directory
```

![server interface](https://user-images.githubusercontent.com/4168619/148116974-7010e265-5aa8-4990-a4b9-f85abe3eafb0.png)

## Installation

### macOS

Using Homebrew:

```bash
brew tap cooklang/tap
brew install cooklang/tap/cookcli
```

### Install with Cargo

If you have Rust installed:

```bash
cargo install cookcli
```

### Download Binary

Download the latest release for your platform from the [releases page](https://github.com/cooklang/CookCLI/releases) and add it to your PATH.

### Build from Source

You'll need Rust and Node.js installed. Then:

```bash
# Clone the repository
git clone https://github.com/cooklang/CookCLI.git
cd CookCLI

# Build web UI
cd ui && npm install && npm run build && cd ..

# Build the CLI
cargo build --release

# Binary will be at target/release/cook
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

Import recipes from websites and convert them to Cooklang format.

```bash
# Import a recipe
cook import https://www.example.com/recipe > recipe.cook

# Import without conversion
cook import https://www.example.com/recipe --skip-conversion
```

### `cook doctor`

Check your recipe collection for issues and maintain consistency.

```bash
# Validate all recipes
cook doctor validate

# Check aisle configuration for shopping lists
cook doctor aisle

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
cook report -t recipe-card.j2 recipe.cook

# Create nutrition label
cook report -t nutrition.j2 recipe.cook
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

## Configuration

CookCLI looks for configuration files in:

* `./config/` - in your recipe directory
* `~/.config/cooklang/` - in your home directory

The main configuration file is `aisle.conf` which organizes ingredients by store section for shopping lists:

```
[produce]
tomatoes
basil
garlic

[dairy]
milk
cheese
yogurt

[pantry]
flour
sugar
pasta
```

## Tips

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
cook search chicken | xargs cook shopping-list

# Convert all recipes to Markdown
for r in *.cook; do
  cook recipe "$r" -f markdown > "docs/${r%.cook}.md"
done

# Check which recipes are quick
cook search "" | while read r; do
  cook recipe "$r" -f json | jq -r 'select(.metadata.time <= 30) | .title'
done
```

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Areas where we'd love help:

* Bug fixes and testing
* Documentation improvements
* New features
* Recipe collections
* Translations

## License

MIT License. See [LICENSE](LICENSE) for details.

Some source files include code from [cooklang-chef](https://github.com/Zheoni/cooklang-chef), also under MIT license.

## Links

* [Cooklang Specification](https://cooklang.org/docs/spec) - the recipe markup language
* [Cooklang Apps](https://cooklang.org/app/) - iOS and Android apps
* [Community Recipes](https://github.com/cooklang/recipes) - recipe collections
* [Discussion Forum](https://github.com/cooklang/CookCLI/discussions) - questions and ideas

## Support

* [Issue Tracker](https://github.com/cooklang/CookCLI/issues) - report bugs
* [Discussions](https://github.com/cooklang/CookCLI/discussions) - ask questions
* [Twitter](https://twitter.com/cooklang_org) - updates and news