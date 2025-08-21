# Report Command

The `report` command generates custom reports from recipes using Jinja2 templates. It's a powerful tool for creating recipe cards, nutrition labels, meal plans, or any custom format you need.

⚠️ **Note**: The report command is currently a prototype feature and will evolve in future versions.

## Basic Usage

```bash
cook report -t template.j2 recipe.cook
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
  -t comprehensive.j2 \
  recipe.cook:2 \
  -d ./datastore \
  -a ./config/aisle.conf \
  -p ./config/pantry.conf
```

This provides the template with:
* Scaled recipe (2x)
* Nutritional and cost data from datastore
* Aisle categorization for shopping
* Pantry status for filtering

## Example Templates

### Simple Recipe Card

Create `recipe-card.j2`:

```jinja2
# {{ recipe.title }}

**Servings:** {{ recipe.servings }}
**Time:** {{ recipe.time }}

## Ingredients
{% for ingredient in recipe.ingredients %}
- {{ ingredient.quantity }} {{ ingredient.unit }} {{ ingredient.name }}
{% endfor %}

## Instructions
{% for step in recipe.steps %}
{{ step.number }}. {{ step.instruction }}
{% endfor %}
```

Use it:

```bash
cook report -t recipe-card.j2 "Pasta.cook"
```

### Shopping List Card

Create `shopping-card.j2`:

```jinja2
SHOPPING LIST FOR: {{ recipe.title | upper }}
=====================================

{# Group by aisle if available #}
{% if recipe.ingredients[0].aisle is defined %}
  {% for aisle, items in recipe.ingredients | groupby('aisle') %}
{{ aisle | upper }}
    {% for ingredient in items if not ingredient.in_pantry %}
[ ] {{ ingredient.name }} - {{ ingredient.quantity }} {{ ingredient.unit }}
    {% endfor %}
  {% endfor %}
{% else %}
  {# Simple list if no aisle data #}
  {% for ingredient in recipe.ingredients if not ingredient.in_pantry %}
[ ] {{ ingredient.name }} - {{ ingredient.quantity }} {{ ingredient.unit }}
  {% endfor %}
{% endif %}

Total items to buy: {{ recipe.ingredients | rejectattr('in_pantry') | list | length }}
Pantry items used: {{ recipe.ingredients | selectattr('in_pantry') | list | length }}
```

### Nutrition Label

Create `nutrition.j2`:

```jinja2
Nutrition Facts
---------------
Serving Size: 1 serving
Servings: {{ recipe.servings }}

{% if recipe.nutrition %}
Calories: {{ recipe.nutrition.calories }}
Total Fat: {{ recipe.nutrition.fat }}g
Protein: {{ recipe.nutrition.protein }}g
Carbs: {{ recipe.nutrition.carbs }}g
{% else %}
Nutrition data not available
{% endif %}
```

### HTML Recipe Page

Create `recipe-page.j2`:

```html
<!DOCTYPE html>
<html>
<head>
    <title>{{ recipe.title }}</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; }
        .ingredient { margin: 5px 0; }
        .step { margin: 15px 0; }
    </style>
</head>
<body>
    <h1>{{ recipe.title }}</h1>
    <p>Serves {{ recipe.servings }} | {{ recipe.time }}</p>
    
    <h2>Ingredients</h2>
    <ul>
    {% for ing in recipe.ingredients %}
        <li class="ingredient">
            {{ ing.quantity }} {{ ing.unit }} {{ ing.name }}
            {% if ing.notes %}({{ ing.notes }}){% endif %}
        </li>
    {% endfor %}
    </ul>
    
    <h2>Instructions</h2>
    <ol>
    {% for step in recipe.steps %}
        <li class="step">{{ step.instruction }}</li>
    {% endfor %}
    </ol>
</body>
</html>
```

## Scaling Recipes

Scale recipes before processing:

```bash
# Double the recipe
cook report -t template.j2 "Cake.cook:2"

# Scale to 10 servings
cook report -t template.j2 "Dinner.cook:10"
```

## Configuration Options

### Datastore

Include additional data from a datastore:

```bash
cook report -t nutrition.j2 recipe.cook -d ./datastore
```

The datastore can contain:
* Nutritional information
* Cost data
* Dietary classifications
* Custom metadata

### Aisle Configuration

Categorize ingredients by store section:

```bash
cook report -t shopping.j2 recipe.cook -a ./config/aisle.conf
```

Template can access:
```jinja2
{% for ingredient in recipe.ingredients %}
  {{ ingredient.name }} - Aisle: {{ ingredient.aisle }}
{% endfor %}
```

### Pantry Configuration

Filter out pantry items:

```bash
cook report -t list.j2 recipe.cook -p ./config/pantry.conf
```

Template can check pantry items:
```jinja2
{# Show only non-pantry items #}
{% for ingredient in recipe.ingredients if not ingredient.in_pantry %}
  {{ ingredient.name }}: {{ ingredient.quantity }}
{% endfor %}

{# Or separate them #}
Need to buy:
{% for ing in recipe.ingredients if not ing.in_pantry %}
  - {{ ing.name }}
{% endfor %}

From pantry:
{% for ing in recipe.ingredients if ing.in_pantry %}
  - {{ ing.name }}
{% endfor %}
```

## Output Options

### Save to File

```bash
cook report -t card.j2 recipe.cook > recipe-card.md
```

### Multiple Recipes

Process multiple recipes:

```bash
for recipe in *.cook; do
  cook report -t card.j2 "$recipe" > "cards/$(basename $recipe .cook).md"
done
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
{% for ing in recipe.ingredients %}
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

Create `weekly-plan.j2`:

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
  cook report -t weekly-plan.j2 "$day.cook" >> weekly-plan.md
done
```

### Recipe Book

Create `book-page.j2`:

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

Create `index.j2`:

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
  cook report -t index.j2 "$recipe" >> index.md
done
```

## Template Development

### Testing Templates

```bash
# Test with simple output
cook report -t test.j2 recipe.cook

# Debug available variables
cat > debug.j2 << 'EOF'
Available variables:
{{ recipe | tojson(indent=2) }}
EOF

cook report -t debug.j2 recipe.cook
```

### Template Library

Organize templates:

```
templates/
├── cards/
│   ├── simple.j2
│   ├── detailed.j2
│   └── photo.j2
├── formats/
│   ├── markdown.j2
│   ├── html.j2
│   └── latex.j2
└── special/
    ├── nutrition.j2
    ├── cost.j2
    └── planning.j2
```

### Reusable Components

Create `base.j2`:

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
{% import 'base.j2' as base %}

{{ base.time_display(recipe.time) }}
{{ base.ingredient_list(recipe.ingredients) }}
```

## Integration Examples

### PDF Generation

```bash
# Generate Markdown
cook report -t recipe.j2 recipe.cook > recipe.md

# Convert to PDF with pandoc
pandoc recipe.md -o recipe.pdf

# Or direct to PDF with HTML template
cook report -t recipe-html.j2 recipe.cook | wkhtmltopdf - recipe.pdf
```

### Email Newsletter

```bash
# Generate HTML email
cook report -t email-recipe.j2 "Recipe of the Week.cook" > email.html

# Send with mail command
cat email.html | mail -a "Content-Type: text/html" -s "Recipe of the Week" list@example.com
```

### Social Media

Create `social.j2`:

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
cook report -t /full/path/to/template.j2 recipe.cook

# Or ensure template is in current directory
ls *.j2
```

### Variable Errors

If a variable doesn't exist:

```jinja2
{# Safe access #}
{{ recipe.metadata.note if recipe.metadata.note else "" }}

{# Or use default #}
{{ recipe.metadata.note | default("") }}
```

### Encoding Issues

For special characters:

```jinja2
{{ recipe.title | escape }}  {# HTML escape #}
{{ recipe.title | urlencode }}  {# URL encoding #}
```

## See Also

* [Recipe](recipe.md) – View recipe data structure
* [Shopping List](shopping-list.md) – Generate shopping lists
* [Server](server.md) – Web-based recipe viewing