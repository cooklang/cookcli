# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

CookCLI is a command-line interface for managing Cooklang recipes. It's written in Rust and includes a web server with server-side rendered HTML using Askama templates and Tailwind CSS. The project follows UNIX philosophy where each command does one thing well.

## Build and Development Commands

### Building
```bash
# Full release build (includes CSS compilation)
make release

# Development build
cargo build
make build  # Includes CSS compilation

# Build CSS only
make css
npm run build-css

# Build specific workspace member
cargo build -p cookcli

# Run without building
cargo run -- [command] [args]
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo run -- [command]
RUST_LOG=trace cargo run -- [command]  # More verbose
```

### Web UI Development
```bash
# Start development server with CSS built
make dev_server

# Start development with CSS watch mode
make css-watch  # In one terminal
cargo run -- server ./seed  # In another terminal

# Install dependencies (first time setup)
npm install

# Build CSS for production
npm run build-css

# Watch CSS changes during development
npm run watch-css
```

### Linting and Formatting
```bash
cargo fmt         # Format code
cargo clippy      # Lint code
```

## Architecture

### Workspace Structure
This is a Cargo workspace with multiple crates:
- `cookcli` (this crate) - The CLI application
- `../cooklang-rs` - Core Cooklang parser and models
- `../cooklang-find` - Recipe discovery and search
- `../cooklang-reports` - Template-based report generation
- `../cooklang-import` - Recipe import from websites (external dependency)

### Command Architecture
Each command is a module in `src/` with a consistent pattern:
1. `XxxArgs` struct in `src/args.rs` or module file - Clap argument definitions
2. `pub fn run(ctx: &Context, args: XxxArgs) -> Result<()>` - Entry point
3. Commands receive a `Context` with base_path, aisle, and pantry configuration

### Context System
The `Context` struct (in `src/main.rs`) provides:
- `base_path`: Current working directory for recipes
- `aisle()`: Returns path to aisle.conf (local `./config/` or global `~/.config/cook/`)
- `pantry()`: Returns path to pantry.conf (same search pattern)

Configuration search order:
1. `./config/[aisle.conf|pantry.conf]` - Local to recipe directory
2. `~/Library/Application Support/cook/` (macOS) or `~/.config/cook/` (Linux)

### Command Modules

#### Core Commands
- `recipe`: Parse and display recipes (supports multiple output formats)
- `shopping_list`: Generate shopping lists with ingredient aggregation
- `server`: Web server using Axum + Askama templates with Tailwind CSS
- `search`: Full-text recipe search
- `import`: Import from websites via cooklang-import
- `doctor`: Validate recipes and check configurations
- `seed`: Initialize with example recipes
- `report`: Generate custom outputs using Jinja2 templates

#### Utility Modules
- `util/`: Shared utilities for parsing, conversion, and output formatting
  - Recipe parsing and scaling
  - Output format conversions (human, JSON, YAML, Markdown, Cooklang)
  - Shopping list generation with aisle/pantry support

### Web Server
- Backend: Axum web framework in `src/server/`
- Frontend: Server-side rendered HTML using Askama templates
- Templates: Located in `templates/` directory
- Styling: Tailwind CSS with custom components
- API handlers in `src/server/handlers/`
- Static files served from `static/` directory
- Shopping list stored as tab-delimited files in `/tmp/`

### Recipe Processing Pipeline
1. Recipe discovery via `cooklang-find` (handles paths and search)
2. Parsing via `cooklang` parser (lenient mode for validation)
3. Scaling and conversion in `util/` modules
4. Output formatting based on requested format

### Error Handling
- Uses `anyhow::Result` throughout for error propagation
- User-friendly error messages with context
- Special handling for recipe validation (warnings vs errors)

## Important Implementation Details

### Recipe Scaling
- Scaling notation: `recipe.cook:2` (uses colon)
- Handled in `util::split_recipe_name_and_scaling_factor()`
- Applied during parsing before output

### Shopping List Aggregation
- Ingredients with same name are automatically combined
- Unit conversion handled by cooklang crate
- Aisle categorization via aisle.conf
- Pantry filtering via pantry.conf (TOML format with quantities and dates)

### Doctor Command
- `validate`: Checks syntax, deprecated features, and recipe references
- `aisle`: Finds ingredients not in aisle configuration
- Exit codes: Normal (0) or strict mode (1) for CI/CD

### Report Command
- Uses cooklang-reports with Jinja2 templates
- Config builder pattern for scale, datastore, aisle, pantry
- Falls back to context configurations if not specified

### File Formats
- `.cook` files use Cooklang markup
- Supports YAML frontmatter or `>>` metadata (deprecated)
- Output formats: human, json, yaml, markdown, cooklang

## Frontend Architecture

### Template System
The web UI uses server-side rendering with Askama templates:
- Templates located in `templates/` directory
- Base template (`base.html`) provides common layout
- Page-specific templates extend the base template
- Template data structures defined in `src/server/templates.rs`

### Styling
- Tailwind CSS for utility-first styling
- Custom components defined in `static/css/input.css`
- Compiled CSS output in `static/css/output.css`
- Configuration in `tailwind.config.js`

### Key Templates
- `base.html` - Common layout with navigation and search
- `recipes.html` - Recipe listing with directory navigation
- `recipe.html` - Individual recipe display with scaling
- `shopping_list.html` - Shopping list management
- `preferences.html` - User preferences and settings

### Frontend Features
- **Recipe Browsing**: Directory-based navigation with breadcrumbs
- **Recipe Display**: Ingredients, steps, metadata with colorful badges
- **Shopping List**: Persistent storage in `/tmp/shopping_list.txt`
- **Recipe Scaling**: Dynamic scaling with URL parameters
- **Search**: Real-time recipe search with dropdown results
- **Responsive Design**: Mobile-friendly layout with Tailwind

### CSS Component Classes
Custom Tailwind components for consistent styling:
- `.recipe-card` - Recipe cards with gradient top border
- `.ingredient-badge` - Orange gradient badges for ingredients
- `.cookware-badge` - Green gradient badges for cookware
- `.timer-badge` - Red gradient badges for timers
- `.metadata-pill` - Clean outline badges for metadata
- `.nav-pill` - Navigation items with hover effects
- `.step-number` - Circular step numbers with gradient

## Testing Approach

Currently no automated tests (as noted in CONTRIBUTING.md). Manual testing approach:
1. Use `cook seed` to create test recipes
2. Test each command with various options
3. Validate output formats
4. Check error handling with invalid inputs

## Release Process

Uses semantic commit messages for automated releases:
- `feat:` - New features
- `fix:` - Bug fixes  
- `docs:` - Documentation changes
- `chore:` - Maintenance tasks

## Common Development Tasks

### Adding a New Command
1. Add variant to `Command` enum in `src/args.rs`
2. Create module in `src/` with `XxxArgs` struct and `run()` function
3. Add case in `main.rs` match statement
4. Update help text and documentation

### Modifying Output Formats
Output formatting is centralized in `src/util/` modules. Each format has its own module with consistent interface.

### Working with Templates
1. Create new template in `templates/` directory
2. Define data structure in `src/server/templates.rs`
3. Implement handler in `src/server/ui.rs`
4. Add route in the UI router

### Modifying Styles
1. Edit component classes in `static/css/input.css`
2. Run `make css` or `npm run build-css` to compile
3. For development, use `npm run watch-css` for auto-rebuild
4. Custom colors and utilities can be added to `tailwind.config.js`

### Frontend Development Workflow
1. Install dependencies: `npm install`
2. Start CSS watch: `npm run watch-css`
3. Run server: `cargo run -- server ./seed`
4. Make changes to templates or styles
5. Refresh browser to see changes (templates are recompiled on each request in dev mode)

### Debugging Recipe Parsing
Use `RUST_LOG=trace` to see detailed parsing information including:
- File discovery paths
- Configuration loading
- Recipe reference resolution
- Static file serving paths
