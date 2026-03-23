# CookCLI

CookCLI is a free, open-source command-line tool for working with [Cooklang](https://cooklang.org/docs/spec/) recipe files. It parses `.cook` files, generates combined shopping lists from multiple recipes, runs a local web server to browse your collection, imports recipes from websites, and scales servings — all from the terminal.

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| [recipe](recipe.md) | `r` | Parse, validate and display recipe files |
| [shopping-list](shopping-list.md) | `sl` | Generate a combined shopping list from recipes |
| [server](server.md) | `s` | Start a web server to browse recipes |
| [search](search.md) | `f` | Search through your recipe collection |
| [import](import.md) | `i` | Import recipes from websites |
| [report](report.md) | `rp` | Generate custom reports using templates |
| [doctor](doctor.md) | | Analyze recipes for issues |
| [pantry](pantry.md) | `p` | Manage and analyze pantry inventory |
| [seed](seed.md) | | Initialize with example recipes |
| [lsp](lsp.md) | | Start the Language Server Protocol server |
| [update](update.md) | `u` | Update CookCLI to the latest version |

## Installation

### Download Binary

Download the latest release for your platform from the [releases page](https://github.com/cooklang/CookCLI/releases) and add it to your PATH.

### macOS/Linux

Using Homebrew:

```bash
brew install cookcli
```

### Install with Cargo

If you have Rust installed:

```bash
cargo install cookcli
```

## Global Options

| Option | Description |
|--------|-------------|
| `-v, --verbose...` | Increase verbosity (`-v` info, `-vv` debug, `-vvv` trace) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

## Quick Start

```bash
cook seed                                     # Create example recipes
cook recipe "Neapolitan Pizza"                # View a recipe
cook shopping-list "Neapolitan Pizza" "Easy Pancakes"  # Shopping list
cook server --open                            # Browse in browser
```

For the Cooklang markup language, see the [language specification](https://cooklang.org/docs/spec).
