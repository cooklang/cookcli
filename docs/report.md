# Report Command

Generate custom reports from recipes using templates.

> **Note**: The report command is currently a prototype feature.

## Usage

```
cook report [OPTIONS] --template <TEMPLATE> <RECIPE>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<RECIPE>` | Recipe to process: a path to a `.cook` file or a bare recipe name, resolved against the base path. Supports scaling with `:N` syntax (e.g., `recipe.cook:2`). |

## Options

| Option | Description |
|--------|-------------|
| `-t, --template <FILE>` | Path to the Jinja2 template file (required) |
| `-d, --datastore <DIR>` | Path to datastore directory with additional recipe data (nutrition, costs) |
| `-a, --aisle <FILE>` | Path to aisle configuration file |
| `-p, --pantry <FILE>` | Path to pantry configuration file |
| `-b, --base-path <PATH>` | Base path the `<RECIPE>` argument and any recipe references are resolved against (default: current directory) |

## Template Variables

Templates receive recipe data including:

```jinja2
{{ metadata.servings }}

{% for ingredient in ingredients %}
  {{ ingredient.name }}: {{ ingredient.quantity }}
{% endfor %}

{% for section in sections %}
  {% for content in section %}
    {{ content }}
  {% endfor %}
{% endfor %}
```

Aisle and pantry helpers are available when configs are provided:

```jinja2
{# Group by aisle #}
{% for aisle, items in aisled(ingredients) | items %}
  {{ aisle }}: {% for i in items %}{{ i.name }}{% endfor %}
{% endfor %}

{# Exclude pantry items #}
{% for aisle, items in aisled(excluding_pantry(ingredients)) | items %}
  {{ aisle }}: {% for i in items %}{{ i.name }}{% endfor %}
{% endfor %}
```

## Examples

```bash
# Generate a recipe card
cook report -t reports/recipe-card.md.jinja "Neapolitan Pizza"

# With scaling and all configs
cook report -t reports/cost.md.jinja "Easy Pancakes:2" \
  -d ./db -a ./config/aisle.conf -p ./config/pantry.conf

# Output to file
cook report -t card.jinja recipe.cook > recipe-card.md
```

More template examples: [cooklang-reports](https://github.com/cooklang/cooklang-reports/tree/main/test/data/reports)
