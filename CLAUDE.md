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
- `update`: Self-update the CookCLI binary to the latest version (can be disabled with --no-self-update feature)

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

# UI Testing Documentation

This document describes the comprehensive UI testing setup for CookCLI using Playwright.

## Overview

The testing suite provides end-to-end (E2E) testing for the CookCLI web interface, covering:
- Navigation and routing
- Recipe display and scaling
- Shopping list functionality
- Search capabilities
- Pantry management
- User preferences
- Accessibility compliance
- Performance metrics

## Setup

### Installation

```bash
# Install dependencies
npm install

# Install Playwright browsers
npx playwright install
```

### Running Tests

```bash
# Run all tests
npm test

# Run tests with UI mode (interactive)
npm run test:ui

# Run tests in debug mode
npm run test:debug

# Run tests with browser visible
npm run test:headed

# Run specific browser tests
npm run test:chrome
npm run test:firefox
npm run test:webkit

# Run mobile tests
npm run test:mobile

# Show test report
npm run test:report

# Generate tests interactively
npm run test:codegen
```

## Test Structure

### Directory Layout

```
tests/
├── e2e/                    # End-to-end tests
│   ├── navigation.spec.ts     # Navigation and routing tests
│   ├── recipe-display.spec.ts # Recipe rendering tests
│   ├── recipe-scaling.spec.ts # Recipe scaling functionality
│   ├── search.spec.ts         # Search functionality
│   ├── shopping-list.spec.ts  # Shopping list management
│   ├── preferences.spec.ts    # User preferences
│   ├── pantry.spec.ts        # Pantry management
│   ├── accessibility.spec.ts  # WCAG compliance tests
│   └── performance.spec.ts    # Performance metrics
└── fixtures/              # Test utilities
    └── test-helpers.ts    # Reusable helper functions
```

### Test Helpers

The `test-helpers.ts` file provides reusable utilities:

- **TestHelpers**: Common navigation and interaction methods
- **RecipePage**: Recipe-specific page object model
- **ShoppingListPage**: Shopping list page object model

## Test Coverage

### Navigation Tests (`navigation.spec.ts`)
- Home page display
- Recipe navigation
- Breadcrumb navigation
- Directory browsing
- Navigation menu consistency
- Browser history handling

### Recipe Display (`recipe-display.spec.ts`)
- Recipe title and description
- Ingredients list
- Cooking steps
- Ingredient/cookware/timer highlighting
- Recipe metadata
- Responsive layout

### Recipe Scaling (`recipe-scaling.spec.ts`)
- Scale input functionality
- URL parameter scaling
- Decimal scaling support
- Scaling persistence
- Shopping list integration

### Search (`search.spec.ts`)
- Search input functionality
- Search results display
- No results handling
- Special character support
- Case-insensitive search
- Search persistence

### Shopping List (`shopping-list.spec.ts`)
- Empty state display
- Adding ingredients
- Item completion toggling
- List clearing
- Ingredient aggregation
- Aisle organization
- Session persistence

### Preferences (`preferences.spec.ts`)
- Preference display
- Pantry configuration
- Aisle configuration
- File upload handling
- Configuration validation
- Settings persistence

### Pantry Management (`pantry.spec.ts`)
- Pantry navigation
- Item display
- Adding/editing/removing items
- Shopping list filtering
- Recipe integration
- Import/export functionality

### Accessibility (`accessibility.spec.ts`)
- WCAG 2.0 AA compliance
- Keyboard navigation
- Screen reader support
- ARIA labels
- Color contrast
- Focus indicators
- Form labels

### Performance (`performance.spec.ts`)
- Page load times
- Search performance
- Scaling responsiveness
- Memory usage
- Asset caching
- Cumulative Layout Shift (CLS)

## Configuration

### Playwright Configuration (`playwright.config.ts`)

Key settings:
- **Base URL**: `http://localhost:9080`
- **Web Server**: Automatically starts dev server
- **Browsers**: Chrome, Firefox, Safari, Mobile
- **Parallel Execution**: Enabled
- **Retries**: 2 on CI, 0 locally
- **Artifacts**: Screenshots, videos, traces on failure

### CI/CD Integration

GitHub Actions workflow (`ui-tests.yml`):
- Runs on push/PR to main branch
- Matrix testing across OS and browsers
- Artifact upload for test results
- Parallel job execution

## Writing New Tests

### Basic Test Structure

```typescript
import { test, expect } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

test.describe('Feature Name', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    await helpers.navigateTo('/');
  });

  test('should do something', async ({ page }) => {
    // Test implementation
    await expect(page.locator('selector')).toBeVisible();
  });
});
```

### Using Test Helpers

```typescript
// Navigate to a page
await helpers.navigateTo('/recipes');

// Search for a recipe
await helpers.searchRecipe('pasta');

// Scale a recipe
await helpers.scaleRecipe(2);

// Add to shopping list
await helpers.addToShoppingList();
```

## Best Practices

### Test Isolation
- Each test should be independent
- Use `beforeEach` for setup
- Clean up after tests when necessary

### Selectors
- Prefer semantic selectors (roles, labels)
- Use data attributes for test-specific targeting
- Avoid brittle CSS selectors

### Assertions
- Use explicit waits (`waitForLoadState`)
- Check visibility before interaction
- Verify both positive and negative cases

### Performance
- Run tests in parallel when possible
- Use page objects for reusability
- Minimize test data setup

## Debugging

### Visual Debugging

```bash
# Run with UI mode
npm run test:ui

# Run with browser visible
npm run test:headed

# Debug specific test
npm run test:debug
```

### Trace Viewer

Traces are automatically captured on failure:

```bash
# View trace
npx playwright show-trace trace.zip
```

### Screenshots and Videos

Available in `test-results/` directory after failures.

## Continuous Integration

Tests run automatically on:
- Push to main branch
- Pull requests
- Manual workflow dispatch

Results available as GitHub Actions artifacts.

## Troubleshooting

### Common Issues

1. **Server not starting**: Ensure `make dev_server` works locally
2. **Browser installation**: Run `npx playwright install`
3. **Port conflicts**: Check port 9080 is available
4. **Slow tests**: Increase timeouts in config

### Environment Variables

- `CI`: Set in CI environment for different behavior
- `DEBUG`: Enable Playwright debug output

## Future Improvements

- [ ] Visual regression testing
- [ ] API mocking for edge cases
- [ ] Load testing with multiple concurrent users
- [ ] Internationalization testing
- [ ] Cross-browser compatibility matrix
