# Report Command

The `report` command generates custom reports from recipes using minijinja templates. It's a powerful tool for creating recipe cards, nutrition labels, meal plans, or any custom format you need.

⚠️ **Note**: The report command is currently a prototype feature and will evolve in future versions.

## Basic Usage

```bash
cook report -t template.jinja recipe.cook
```

This processes the recipe through the template and outputs the result.

## How It Works

The report command:
1. Parses the recipe file
2. Applies any scaling
3. Loads aisle and pantry configurations (if provided)
4. Passes recipe data to the Jinja2 template
5. Outputs the rendered result

## Template Variables

Templates receive comprehensive recipe data:

### Recipe Object

```jinja2
{{ recipe.title }}           # Recipe title
{{ recipe.servings }}         # Number of servings
{{ recipe.time }}            # Total time
{{ recipe.description }}      # Recipe description
```

### Ingredients

```jinja2
{% for ingredient in recipe.ingredients %}
  {{ ingredient.name }}       # Ingredient name
  {{ ingredient.quantity }}   # Amount
  {{ ingredient.unit }}       # Unit of measure
  {{ ingredient.notes }}      # Preparation notes
{% endfor %}
```

### Steps

```jinja2
{% for step in recipe.steps %}
  {{ step.number }}          # Step number
  {{ step.instruction }}     # Instruction text
  {{ step.ingredients }}     # Ingredients used
  {{ step.time }}           # Time for step
{% endfor %}
```

### Metadata

```jinja2
{{ recipe.metadata.author }}
{{ recipe.metadata.tags }}
{{ recipe.metadata.cuisine }}
{{ recipe.metadata.source }}
```

## Using All Configurations Together

```bash
cook report \
  -t reports/cost.md.jinja \
  "Breakfast/Easy Pancakes.cook:2" \
  -d ./db \
  -a ./config/aisle.conf \
  -p ./config/pantry.conf
```

Outputs:

```markdown
# Cost Report

* eggs: $1.50
* flour: $0.38
* milk: $0.50
* sea salt: $0.00
* olive oil: $0.01

Total: $2.39
```

This provides the template with:
* Scaled recipe (2x)
* Nutritional and cost data from datastore
* Aisle categorization for shopping
* Pantry inventory for filtering (excludes items you already have)

## Example Templates

### Simple Recipe Card

Create `recipe-card.jinja`:

```jinja2
**Servings:** {{ metadata.servings }}

## Ingredients
{%- for ingredient in ingredients %}
- {{ ingredient.quantity }} {{ ingredient.unit }} {{ ingredient.name }}
{%- endfor %}

## Instructions
{% for section in sections %}
{% for content in section %}
{{ content }}
{%- endfor %}
{%- endfor %}

```

Use it:

```bash
cook report -t reports/recipe-card.md.jinja "Neapolitan Pizza.cook"
```

Checkout more reports [here](https://github.com/cooklang/cooklang-reports/tree/main/test/data/reports).

## Scaling Recipes

Scale recipes before processing:

```bash
# Double the recipe
cook report -t template.jinja "Cake.cook:2"

# Scale to 10 servings
cook report -t template.jinja "Dinner.cook:10"
```

## Configuration Options

### Datastore

Include additional data from a datastore:

```bash
cook report -t nutrition.jinja recipe.cook -d ./db
```

The datastore can contain:
* Nutritional information
* Cost data
* Dietary classifications
* Custom metadata

### Aisle Configuration

Categorize ingredients by store section:

```bash
cook report -t shopping.jinja recipe.cook -a ./config/aisle.conf
```

Template can access:
```jinja2
## Organized by Store Aisle

{%- for aisle, items in aisled(ingredients) | items %}

### {{ aisle | titleize }}
{%- for ingredient in items %}
- [ ] {{ ingredient.name | titleize }}: {{ ingredient.quantity }}
{%- endfor %}
{%- endfor %}
```

### Pantry Configuration

Filter out pantry items using your inventory:

```bash
cook report -t list.jinja recipe.cook -p ./config/pantry.conf
```

The pantry.conf file tracks your inventory with quantities and dates:

```toml
[freezer]
frozen_peas = "500%g"
spinach = { quantity = "1%kg", expire = "05.06.2024" }

[fridge]
milk = { quantity = "2%L", expire = "10.05.2024" }

[pantry]
flour = "5%kg"
salt = "1%kg"
```

Template can check pantry items:
```jinja2
## Items to Buy (Not in Pantry)

{%- for (aisle, items) in aisled(excluding_pantry(ingredients)) | items %}

### {{ aisle | titleize }}
{%- for ingredient in items %}
- [ ] {{ ingredient.name | titleize }}: {{ ingredient.quantity }}
{%- endfor %}
{%- endfor %}

---

## Already Have in Pantry

{%- for ingredient in from_pantry(ingredients) %}
- ✓ {{ ingredient.name | titleize }}: {{ ingredient.quantity }}
{%- endfor %}
```

## Output Options

### Save to File

```bash
cook report -t card.jinja recipe.cook > recipe-card.md
```

## Advanced Templates

### Conditional Content

```jinja2
{% if recipe.metadata.vegetarian %}
  🌱 Vegetarian
{% endif %}

{% if recipe.time < 30 %}
  ⚡ Quick Recipe
{% endif %}

{% if recipe.metadata.difficulty == "easy" %}
  👍 Beginner Friendly
{% endif %}
```

### Formatted Output

```jinja2
{# Format quantities nicely #}
{% for ing in ingredients %}
  {{ "%-20s" | format(ing.name) }} {{ "%8.2f %s" | format(ing.quantity, ing.unit) }}
{% endfor %}

{# Table format #}
| Ingredient | Amount | Unit |
|------------|--------|------|
{% for ing in recipe.ingredients %}
| {{ ing.name }} | {{ ing.quantity }} | {{ ing.unit }} |
{% endfor %}
```

### Calculations

```jinja2
{# Calculate totals #}
{% set total_time = recipe.prep_time + recipe.cook_time %}
Total time: {{ total_time }} minutes

{# Per-serving calculations #}
{% for ing in recipe.ingredients %}
  Per serving: {{ (ing.quantity / recipe.servings) | round(2) }} {{ ing.unit }}
{% endfor %}

{# Shopping estimates #}
Estimated cost: ${{ recipe.ingredients | length * 2.50 }}
```

## Practical Examples

### Meal Planning

Create `weekly-plan.jinja`:

```jinja2
## {{ recipe.metadata.day }} - {{ recipe.title }}

**Prep:** {{ recipe.metadata.prep_time }}
**Cook:** {{ recipe.metadata.cook_time }}

### Shopping needed:
{% for ing in recipe.ingredients %}
- {{ ing.name }}
{% endfor %}

### Notes:
{{ recipe.metadata.notes }}
---
```

Generate weekly plan:

```bash
for day in monday tuesday wednesday; do
  cook report -t weekly-plan.jinja "$day.cook" >> weekly-plan.md
done
```

### Recipe Book

Create `book-page.jinja`:

```latex
\section{{{ recipe.title }}}
\subsection{Information}
\begin{itemize}
  \item Servings: {{ recipe.servings }}
  \item Time: {{ recipe.time }}
  \item Difficulty: {{ recipe.metadata.difficulty | default("Medium") }}
\end{itemize}

\subsection{Ingredients}
\begin{itemize}
{% for ing in recipe.ingredients %}
  \item {{ ing.quantity }} {{ ing.unit }} {{ ing.name }}
{% endfor %}
\end{itemize}

\subsection{Instructions}
\begin{enumerate}
{% for step in recipe.steps %}
  \item {{ step.instruction }}
{% endfor %}
\end{enumerate}
```

### Index Generation

Create `index.jinja`:

```markdown
# Recipe Index

## {{ recipe.title }}
- **File:** {{ recipe.filename }}
- **Servings:** {{ recipe.servings }}
- **Time:** {{ recipe.time }}
- **Tags:** {{ recipe.metadata.tags | join(", ") }}
- **Ingredients:** {{ recipe.ingredients | length }} items

---
```

Generate index:

```bash
for recipe in *.cook; do
  cook report -t index.jinja "$recipe" >> index.md
done
```

### Reusable Components

Create `base.jinja`:

```jinja2
{# Macro for ingredient list #}
{% macro ingredient_list(ingredients) %}
{% for ing in ingredients %}
- {{ ing.quantity }} {{ ing.unit }} {{ ing.name }}
{% endfor %}
{% endmacro %}

{# Macro for time display #}
{% macro time_display(minutes) %}
{% if minutes < 60 %}
  {{ minutes }} minutes
{% else %}
  {{ (minutes / 60) | round(1) }} hours
{% endif %}
{% endmacro %}
```

Use in other templates:

```jinja2
{% import 'base.jinja' as base %}

{{ base.time_display(recipe.time) }}
{{ base.ingredient_list(recipe.ingredients) }}
```

## Integration Examples

### PDF Generation

```bash
# Generate Markdown
cook report -t recipe.md.jinja recipe.cook > recipe.md

# Convert to PDF with pandoc
pandoc recipe.md -o recipe.pdf

# Or direct to PDF with HTML template
cook report -t recipe.html.jinja recipe.cook | wkhtmltopdf - recipe.pdf
```

### Email Newsletter

```bash
# Generate HTML email
cook report -t email-recipe.html.jinja "Recipe of the Week.cook" > email.html

# Send with mail command
cat email.html | mail -a "Content-Type: text/html" -s "Recipe of the Week" list@example.com
```

### Social Media

Create `social.jinja`:

```jinja2
🍽️ {{ recipe.title }}
⏱️ {{ recipe.time }} | 👥 Serves {{ recipe.servings }}

Ingredients: {{ recipe.ingredients | map(attribute='name') | join(', ') }}

Get the full recipe at: example.com/recipes/{{ recipe.slug }}

#cooking #{{ recipe.metadata.cuisine }} #homemade
```

## Tips and Tricks

### Default Values

```jinja2
{{ recipe.metadata.author | default("Anonymous") }}
{{ recipe.servings | default(4) }}
{{ recipe.metadata.difficulty | default("Medium") }}
```

### Filters

```jinja2
{{ recipe.title | upper }}           # UPPERCASE
{{ recipe.title | lower }}           # lowercase
{{ recipe.title | title }}           # Title Case
{{ ingredient.name | replace('_', ' ') }}  # Replace characters
{{ recipe.time | round }}            # Round numbers
```

### Loops with Conditions

```jinja2
{# Only list main ingredients #}
{% for ing in recipe.ingredients if ing.quantity > 0 %}
  {{ ing.name }}
{% endfor %}

{# Group by category #}
{% for category, items in recipe.ingredients | groupby('category') %}
  {{ category }}:
  {% for item in items %}
    - {{ item.name }}
  {% endfor %}
{% endfor %}
```

## Troubleshooting

### Template Not Found

```bash
# Use absolute path
cook report -t /full/path/to/template.jinja recipe.cook

# Or ensure template is in current directory
ls *.jinja
```

### Variable Errors

If a variable doesn't exist:

```jinja2
{# Safe access #}
{{ recipe.metadata.note if recipe.metadata.note else "" }}

{# Or use default #}
{{ recipe.metadata.note | default("") }}
```
