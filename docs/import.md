# Import Command

The `import` command fetches recipes from websites and automatically converts them to Cooklang format. It supports hundreds of popular recipe websites and extracts ingredients, instructions, and metadata intelligently.

Requires `OPENAI_API_KEY` environment variable set to perform the conversion to Cooklang. Without the key you still can downlad recipe original content, but it won't be converted to Cooklang.

## Basic Usage

```bash
cook import https://www.bbcgoodfood.com/recipes/chicken-bacon-pasta
```

This downloads the recipe and outputs it in Cooklang format into stdout.

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
---
author: John Chandler
cook time: 2 hours 30 minutes
course: Dinner
cuisine: Italian Inspired, Italian
prep time: 30 minutes
servings: 12
source: "https://www.allrecipes.com/recipe/23600/worlds-best-lasagna/"
time required: 3 hours 15 minutes
---

Cook @sweet Italian sausage{1%lb}, @lean ground beef{0.75%lb}, @minced onion{0.5%cup}, and @garlic{2 cloves}, crushed in a #Dutch oven{} over medium heat until well browned...
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
---
author: John Doe
cuisine: Italian
prep_time: 30 minutes
cook_time: 45 minutes
servings: 4
---

World's Best Lasagna

This is a delicious lasagna recipe...

[Ingredients]
1 pound ground beef
1 onion, chopped
2 cloves garlic, minced
...

[Instructions]
1. Preheat oven to 375°F.
2. Cook lasagna noodles according to package directions.
3. Brown ground beef with onion and garlic...

[Images]
https://example.com/recipe-image.jpg
```

### Metadata Options

Control how metadata is included in the output:

```bash
# Default: Include as YAML frontmatter
cook import https://example.com/recipe --skip-conversion

# Output metadata as JSON
cook import https://example.com/recipe --skip-conversion --metadata json

# Output metadata as YAML section
cook import https://example.com/recipe --skip-conversion --metadata yaml

# Exclude metadata
cook import https://example.com/recipe --skip-conversion --metadata none

# Extract only metadata (no recipe content)
cook import https://example.com/recipe --metadata-only

# Extract metadata as JSON
cook import https://example.com/recipe --metadata-only --metadata json
```

### Metadata Extraction

Automatically extracts:
* Title
* Description
* Images
* Servings/Yield
* Prep/Cook/Total time
* Source URL
* Author
* Tags/Categories
* Cuisine
* Course
* Difficulty
* Ratings
* Nutrition information
* And more depending on the source

## Working with Different Sites

### Paywalled Sites (TODO)

Some sites require authentication:

```bash
# NYT Cooking and similar sites may require login
# Copy the recipe text manually, then:
pbpaste | cook recipe - > recipe.cook
```

### Sites with Anti-Scraping (TODO)

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
---
source: https://original-website.com/recipe
author: Original Author
imported: 2024-01-20
---
```

## See Also

* [Recipe](recipe.md) – View and validate imported recipes
* [Doctor](doctor.md) – Check imported recipes for issues
* [Server](server.md) – Browse your imported collection
