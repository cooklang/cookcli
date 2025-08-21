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

## Validation Check

### Basic Validation

```bash
cook doctor validate
```

Example output:
```
📄 Pasta Carbonara.cook
  ❌ Error: Unknown timer unit: mins
  ⚠️  Warning: Deprecated metadata syntax: use YAML frontmatter

📄 Pizza Dough.cook
  ❌ Error: Invalid quantity: "handful"
  ⚠️  Warning: Missing required metadata: servings

=== Recipe References ===
📄 Lasagna.cook
  ❌ Missing reference: Bechamel Sauce.cook

=== Validation Summary ===
Total recipes scanned: 25
❌ 3 error(s) found in 2 recipe(s)
⚠️  4 warning(s) found in 3 recipe(s)
```

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

## Aisle Check

### Basic Check

```bash
cook doctor aisle
```

Example output:
```
Scanned 50 recipes, found 125 unique ingredients

✓ All ingredients are present in aisle configuration
```

Or if missing ingredients:

```
Scanned 50 recipes, found 125 unique ingredients

16 ingredients not found in aisle configuration:
  - quinoa
  - tahini
  - miso paste
  - fish sauce
  - sumac
  ...

Consider adding these ingredients to your aisle.conf file.
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

### Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
echo "Validating recipes..."
cook doctor validate --strict
if [ $? -ne 0 ]; then
  echo "Recipe validation failed. Please fix errors before committing."
  exit 1
fi
```

### Makefile Integration

```makefile
.PHONY: validate
validate:
	@echo "Checking recipes..."
	@cook doctor validate --strict

.PHONY: check-aisle
check-aisle:
	@echo "Checking aisle configuration..."
	@cook doctor aisle

.PHONY: health-check
health-check: validate check-aisle
	@echo "All checks passed!"
```

## Maintaining Recipe Quality

### Regular Health Checks

Run doctor regularly:

```bash
# Add to crontab or scheduled tasks
0 9 * * 1 cd ~/recipes && cook doctor > ~/recipe-health.log
```

### Progressive Enhancement

Start with fixing errors, then warnings:

```bash
# First, fix all errors
cook doctor validate | grep "❌"

# Then address warnings
cook doctor validate | grep "⚠️"
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

## Advanced Validation

### Custom Validation Rules

Create a validation script:

```bash
#!/bin/bash
# validate-custom.sh

# Check for required metadata
for recipe in *.cook; do
  if ! grep -q "^servings:" "$recipe"; then
    echo "Missing servings: $recipe"
  fi
  if ! grep -q "^time:" "$recipe"; then
    echo "Missing time: $recipe"
  fi
done

# Check image files exist
for recipe in *.cook; do
  base=$(basename "$recipe" .cook)
  if [ ! -f "${base}.jpg" ] && [ ! -f "${base}.png" ]; then
    echo "Missing image: $recipe"
  fi
done
```

### Validation Report

Generate detailed reports:

```bash
# Create validation report
{
  echo "# Recipe Collection Health Report"
  echo "Date: $(date)"
  echo ""
  echo "## Validation Results"
  cook doctor validate
  echo ""
  echo "## Aisle Coverage"
  cook doctor aisle
  echo ""
  echo "## Statistics"
  echo "Total recipes: $(ls *.cook | wc -l)"
  echo "With errors: $(cook doctor validate | grep -c '❌')"
  echo "With warnings: $(cook doctor validate | grep -c '⚠️')"
} > health-report.md
```

## Best Practices

### Keep Recipes Valid

1. Run `cook doctor` before committing changes
2. Fix errors immediately
3. Address warnings when possible
4. Keep aisle.conf updated

### Organize for Health

```
recipes/
├── config/
│   └── aisle.conf      # Maintained and complete
├── validated/          # Recipes that pass all checks
├── drafts/            # Work in progress
└── archive/           # Old recipes
```

### Document Issues

When you can't fix an issue immediately:

```cooklang
>> known_issues: Missing quantity for salt - season to taste
>> todo: Add precise measurements after testing
```

## Troubleshooting

### Performance Issues

For large collections:

```bash
# Check specific directories
cook doctor validate -b recipes/tested

# Run checks in parallel
find . -type d -maxdepth 1 | xargs -P 4 -I {} cook doctor validate -b {}
```

### False Positives

Some warnings might be intentional:

```cooklang
# Intentionally vague quantity
@salt{} -- to taste

# Non-standard but valid unit
@love{1%handful} -- just kidding!
```

### Integration Issues

If doctor commands fail in CI:

1. Check CookCLI version
2. Verify file permissions
3. Ensure recipes are checked out
4. Check for encoding issues

## See Also

* [Recipe](recipe.md) – View and validate individual recipes
* [Shopping List](shopping-list.md) – Uses aisle configuration
* [Server](server.md) – Browse recipes with validation status