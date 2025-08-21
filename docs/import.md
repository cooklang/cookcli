# Import Command

The `import` command fetches recipes from websites and automatically converts them to Cooklang format. It supports hundreds of popular recipe websites and extracts ingredients, instructions, and metadata intelligently.

## Basic Usage

```bash
cook import https://www.example.com/recipe/delicious-pasta
```

This downloads the recipe and outputs it in Cooklang format.

## Supported Websites

The importer works with most recipe websites that use standard recipe markup, including:

* AllRecipes
* BBC Good Food
* Bon Appétit
* Serious Eats
* Food Network
* NYT Cooking
* Simply Recipes
* And hundreds more...

Any site using Recipe Schema.org markup should work automatically.

## Importing Recipes

### Basic Import

```bash
cook import https://www.allrecipes.com/recipe/23600/worlds-best-lasagna/
```

Output (Cooklang format):
```cooklang
>> title: World's Best Lasagna
>> source: https://www.allrecipes.com/recipe/23600/worlds-best-lasagna/
>> servings: 12
>> time: 3 hours 15 minutes

Preheat oven to ~{375°F}.

Cook @lasagna noodles{12} according to package directions.

Brown @ground beef{1%lb} with @onion{1%medium} and @garlic{2%cloves}...
```

### Save to File

Redirect output to save:

```bash
cook import https://example.com/recipe > lasagna.cook

# Or with shell redirect
cook import [URL] > "Pasta Carbonara.cook"
```

### Import Without Conversion

Get the raw extracted data without converting to Cooklang:

```bash
cook import https://example.com/recipe --skip-conversion
```

Output:
```
World's Best Lasagna

[Ingredients]
1 pound ground beef
1 onion, chopped
2 cloves garlic, minced
...

[Instructions]
1. Preheat oven to 375°F.
2. Cook lasagna noodles according to package directions.
3. Brown ground beef with onion and garlic...
```

## Batch Importing

### Multiple Recipes

Import several recipes at once:

```bash
# Save URLs in a file
cat > recipes.txt << EOF
https://www.example.com/recipe1
https://www.example.com/recipe2
https://www.example.com/recipe3
EOF

# Import all
while read url; do
  name=$(echo "$url" | sed 's/.*\///' | sed 's/-/ /g')
  cook import "$url" > "${name}.cook"
  echo "Imported: $name"
done < recipes.txt
```

### Import Collection

Import a cookbook or collection:

```bash
# Example: Import a meal plan
urls=(
  "https://example.com/monday-dinner"
  "https://example.com/tuesday-dinner"
  "https://example.com/wednesday-dinner"
)

for url in "${urls[@]}"; do
  recipe_name=$(basename "$url")
  cook import "$url" > "meal-plan/${recipe_name}.cook"
done
```

## Smart Conversion

The importer intelligently converts recipes:

### Ingredient Recognition

* Extracts quantities and units
* Identifies ingredient names
* Preserves preparation notes

```
"2 cups all-purpose flour, sifted"
→ @all-purpose flour{2%cups} -- sifted
```

### Instruction Processing

* Identifies ingredients in steps
* Detects cooking times
* Recognizes equipment

```
"Bake in the oven for 45 minutes"
→ Bake in the #oven for ~{45%minutes}
```

### Metadata Extraction

Automatically extracts:
* Title
* Servings/Yield
* Prep/Cook/Total time
* Source URL
* Author
* Tags/Categories

## Advanced Usage

### Recipe Research

Compare recipes from different sources:

```bash
# Import multiple versions of the same dish
cook import https://site1.com/carbonara > carbonara-site1.cook
cook import https://site2.com/carbonara > carbonara-site2.cook
cook import https://site3.com/carbonara > carbonara-site3.cook

# Compare ingredients
for file in carbonara-*.cook; do
  echo "=== $file ==="
  cook recipe "$file" -f json | jq '.ingredients[].name'
done
```

### Building a Collection

Create a curated cookbook:

```bash
# Italian cookbook
mkdir -p cookbook/italian

# Import recipes
cook import https://example.com/pasta-arrabiata > cookbook/italian/arrabiata.cook
cook import https://example.com/risotto > cookbook/italian/risotto.cook
cook import https://example.com/tiramisu > cookbook/italian/tiramisu.cook

# Generate index
ls cookbook/italian/*.cook > cookbook/index.txt
```

### Recipe Adaptation

Import and modify recipes:

```bash
# Import original
cook import https://example.com/recipe > original.cook

# Create variation
cp original.cook my-variation.cook

# Edit to taste
vim my-variation.cook
# Reduce sugar, add spices, etc.
```

### Quality Check

Validate imported recipes:

```bash
# Import and validate
url="https://example.com/recipe"
cook import "$url" > temp.cook

# Check for issues
if cook doctor validate temp.cook; then
  mv temp.cook "recipes/$(basename $url).cook"
  echo "Recipe imported successfully"
else
  echo "Recipe has issues, please review"
  vim temp.cook
fi
```

## Working with Different Sites

### Paywalled Sites

Some sites require authentication:

```bash
# NYT Cooking and similar sites may require login
# Copy the recipe text manually, then:
pbpaste | cook recipe - > recipe.cook
```

### Sites with Anti-Scraping

For sites that block automated access:

1. Save the webpage locally
2. Extract manually or use browser tools
3. Convert saved HTML

```bash
# Save page as HTML in browser
# Then extract recipe data
cook import file:///path/to/saved-page.html
```

### Non-Standard Sites

For sites without proper markup:

```bash
# Get raw content and convert manually
cook import https://example.com/recipe --skip-conversion > raw.txt

# Manually format
vim recipe.cook
```

## Integration Workflows

### Pinterest to Cooklang

```bash
# Extract URL from Pinterest
# Copy the actual recipe URL (not Pinterest URL)
# Then import
cook import [actual-recipe-url]
```

### Browser Bookmarklet

Create a bookmarklet for quick imports:

```javascript
javascript:void(window.open('terminal://run?command=cook%20import%20' + encodeURIComponent(window.location.href)));
```

### Recipe Management System

```bash
#!/bin/bash
# import-recipe.sh

url="$1"
title=$(cook import "$url" --skip-conversion | head -1)
filename=$(echo "$title" | tr ' ' '-' | tr '[:upper:]' '[:lower:]').cook

cook import "$url" > "recipes/$filename"
echo "Imported: $title -> $filename"

# Add to git
git add "recipes/$filename"
git commit -m "Add recipe: $title"
```

## Tips and Tricks

### Clean Filenames

```bash
# Import with clean filename
import_recipe() {
  url="$1"
  cook import "$url" > /tmp/recipe.cook
  title=$(grep "^>> title:" /tmp/recipe.cook | cut -d: -f2- | xargs)
  filename=$(echo "$title" | tr ' ' '-' | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]//g').cook
  mv /tmp/recipe.cook "$filename"
  echo "Saved as: $filename"
}
```

### Bulk Import from Blog

```bash
# Import all recipes from a food blog
# First, get all recipe URLs (site-specific)
# Then:
cat urls.txt | while read url; do
  cook import "$url" > "$(basename $url).cook"
  sleep 2  # Be nice to the server
done
```

### Recipe Converter Service

```bash
# Simple web service wrapper
while true; do
  echo "Paste recipe URL (or 'quit'):"
  read url
  [[ "$url" == "quit" ]] && break
  cook import "$url"
done
```

## Troubleshooting

### Import Fails

Common issues and solutions:

* **403 Forbidden**: Site blocks bots. Try saving page locally first.
* **No recipe found**: Site might not use standard markup. Use `--skip-conversion`.
* **Partial import**: Some sites split recipes across pages. May need manual combination.

### Formatting Issues

After import, you might need to:

* Adjust quantities for metric/imperial
* Fix ingredient names for your region
* Correct timing formats
* Add missing metadata

### Rate Limiting

When importing many recipes:

```bash
# Add delay between requests
for url in "${urls[@]}"; do
  cook import "$url" > recipe.cook
  sleep 5  # Wait 5 seconds
done
```

## Best Practices

### Verify Imports

Always review imported recipes:

1. Check ingredient quantities make sense
2. Verify cooking times
3. Ensure steps are complete
4. Add any missing equipment or techniques

### Maintain Attribution

Keep source information:

```cooklang
>> source: https://original-website.com/recipe
>> author: Original Author
>> imported: 2024-01-20
```

### Organize Imports

```bash
recipes/
├── imported/      # Original imports
├── adapted/       # Your modifications
└── tested/        # Recipes you've cooked
```

## See Also

* [Recipe](recipe.md) – View and validate imported recipes
* [Doctor](doctor.md) – Check imported recipes for issues
* [Server](server.md) – Browse your imported collection