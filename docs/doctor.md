# Doctor Command

The `doctor` command helps maintain a healthy recipe collection by checking for syntax errors, validating references, and ensuring proper organization. Think of it as a health check for your recipes.

## Basic Usage

```bash
cook doctor
```

This runs all available checks on your recipe collection and reports any issues found.

## Available Checks

### Recipe Validation

Check all recipes for syntax errors and warnings:

```bash
cook doctor validate
```

This command:
* Detects syntax errors that prevent parsing
* Reports warnings about potential issues
* Validates recipe references (when one recipe includes another)
* Checks for invalid units or quantities
* Identifies deprecated syntax

### Aisle Configuration

Check for ingredients missing from your aisle configuration:

```bash
cook doctor aisle
```

This helps maintain complete shopping list categorization by finding ingredients that aren't assigned to any store section.

### Strict Mode

Use strict mode in CI/CD pipelines:

```bash
cook doctor validate --strict
# Exits with error code 1 if any issues found
```

Great for:
* Pre-commit hooks
* GitHub Actions
* Quality gates

### Validate Specific Directory

```bash
cook doctor validate -b ~/recipes/italian
```

### Custom Base Path

```bash
cook doctor aisle -b ~/my-recipes
```

## Common Issues and Fixes

### Syntax Errors

#### Unknown Timer Units

```cooklang
❌ Wrong: ~{30 mins}
✓ Correct: ~{30%minutes}
```

#### Invalid Quantities

```cooklang
❌ Wrong: @flour{a handful}
✓ Correct: @flour{100%g}
✓ Or: @flour{} -- about a handful
```

#### Missing Units

```cooklang
❌ Wrong: @salt{2}
✓ Correct: @salt{2%tsp}
✓ Or: @salt{2%pinches}
```

### Warnings

#### Deprecated Syntax

```cooklang
⚠️  Deprecated:
>> servings: 4
>> time: 30 minutes

✓ Modern:
---
servings: 4
time: 30 minutes
---
```

#### Unsupported Metadata

```cooklang
⚠️  Warning: Unknown metadata key 'serves'
✓ Use: 'servings' instead
```

### Reference Errors

#### Missing Recipe References

```cooklang
# Main.cook
Include @Pizza Dough.cook  ❌ File doesn't exist

# Fix: Create Pizza Dough.cook or correct the reference
```

#### Circular References

```cooklang
# A.cook includes B.cook
# B.cook includes A.cook
❌ Circular dependency detected
```

## Setting Up CI/CD

### GitHub Actions

Create `.github/workflows/validate-recipes.yml`:

```yaml
name: Validate Recipes

on:
  push:
    paths:
      - '**.cook'
  pull_request:
    paths:
      - '**.cook'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install CookCLI
        run: |
          curl -L https://github.com/cooklang/cookcli/releases/latest/download/cook-x86_64-unknown-linux-gnu.tar.gz | tar xz
          sudo mv cook /usr/local/bin/
      
      - name: Validate recipes
        run: cook doctor validate --strict
```

### Batch Fixes

Fix common issues across all recipes:

```bash
# Fix timer units
find . -name "*.cook" -exec sed -i 's/~{\([0-9]*\) mins}/~{\1%minutes}/g' {} \;

# Update metadata format
for f in *.cook; do
  # Convert >> metadata to YAML frontmatter
  # (script would go here)
done
```

## Aisle Configuration Management

### Creating Aisle Configuration

Generate from existing recipes:

```bash
# Extract all ingredients
cook doctor aisle | grep "^  -" | cut -d'-' -f2- > ingredients.txt

# Organize into aisle.conf
cat > config/aisle.conf << EOF
[produce]
tomatoes
lettuce
onions

[dairy]
milk
cheese
yogurt

[pantry]
flour
sugar
pasta
EOF
```

### Multi-Store Configuration

Different stores have different layouts:

```bash
# walmart-aisle.conf
[aisle-1-produce]
vegetables
fruits

# whole-foods-aisle.conf
[organic-produce]
vegetables
fruits
```

Use appropriate configuration:

```bash
cook shopping-list *.cook --aisle walmart-aisle.conf
```
