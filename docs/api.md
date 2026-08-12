# Server API

HTTP endpoints for building integrations against this CookCLI server.

Start the server with [`cook server`](server.md); every endpoint below is served by it. The same reference is available from a running server at `/api-docs`, where the base URL reflects the host you reached it on.

## Before you start

- **Base URL:** `http://localhost:9080/api`
- **Authentication:** None. Anyone who can reach the server can read and modify your recipes — think twice before using `--host` on an untrusted network.
- **CORS:** All origins are allowed, for the methods GET, POST, PUT and DELETE.
- **Request size limit:** 1 MB.
- **Content type:** JSON in and out, except where noted — raw recipe text is `text/plain`.

## Errors

Every failure returns the same shape, with the status code carrying the meaning:

```json
{ "error": "Recipe not found: Nope.cook" }
```

- `400` — malformed input: an invalid path, a bad query parameter, or a recipe that failed to parse.
- `404` — the recipe, menu, or pantry section does not exist, or no pantry file is configured.
- `500` — the server could not read or write a file.

## Contents

- [Recipes](#recipes)
- [Menus](#menus)
- [Shopping List](#shopping-list)
- [Pantry](#pantry)
- [Search & Stats](#search--stats)
- [Realtime](#realtime)
- [Sync](#sync)

## Recipes

Browse, read, write and delete `.cook` files under the server's recipe directory. Paths are relative to that directory and may include subdirectories.

### `GET /api/recipes`

List every recipe as a directory tree

Returns the recipe tree rooted at the server's base path. Every node — root, directory, and file alike — carries the same four keys: `children`, `name`, `path`, `recipe`. `recipe` is `null` for a directory and non-null for a file; that is the discriminator, not the key's presence. Menus (`.menu`) appear in the same tree as recipes.

Response:

```json
{
  "children": {
    "Breakfast": {
      "children": {
        "Easy Pancakes": {
          "children": {},
          "name": "Easy Pancakes",
          "path": "/absolute/path/to/seed/Breakfast/Easy Pancakes.cook",
          "recipe": {
            "metadata": {
              "author": "CookCLI Team",
              "servings": 2,
              "description": "Simple crepes that are perfect for a lazy weekend breakfast."
            },
            "source": {
              "path": "/absolute/path/to/seed/Breakfast/Easy Pancakes.cook",
              "source_type": "Path"
            }
          }
        }
      },
      "name": "Breakfast",
      "path": "/absolute/path/to/seed/Breakfast",
      "recipe": null
    }
  },
  "name": "seed",
  "path": "/absolute/path/to/seed",
  "recipe": null
}
```

### `GET /api/recipes/*path`

Read one parsed recipe

Parses the recipe and returns its ingredients, cookware, timers and steps. `grouped_ingredients` aggregates repeated ingredients and indexes back into `ingredients`. `inline_quantities` is also present alongside them at the top level of `recipe`. The `image` field is a URL under `/api/static/` when the recipe has a title image, otherwise null.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `path` | path | `string` | yes | Recipe path relative to the recipe directory, e.g. `Breakfast/Easy Pancakes.cook`. The `.cook` extension is optional — the server tries the bare path first, then `.cook`, then `.menu`. |
| `scale` | query | `number` | no | Scaling factor applied during parsing. Defaults to 1. A non-numeric value returns a plain-text 400 ("Failed to deserialize query string: ...") from axum's query deserializer, not the page's usual JSON error envelope. |

Response:

```json
{
  "image": "/api/static/Breakfast/Easy Pancakes.jpg",
  "scale": 2.0,
  "recipe": {
    "metadata": {
      "map": {
        "author": "CookCLI Team",
        "servings": 4,
        "description": "Simple crepes that are perfect for a lazy weekend breakfast."
      }
    },
    "ingredients": [
      {
        "name": "eggs",
        "alias": null,
        "note": null,
        "modifiers": "",
        "quantity": {
          "scalable": true,
          "unit": null,
          "value": { "type": "number", "value": { "type": "regular", "value": 6.0 } }
        },
        "reference": null,
        "relation": {
          "reference_target": null,
          "relation": {
            "defined_in_step": true,
            "referenced_from": [],
            "type": "definition"
          }
        }
      }
    ],
    "grouped_ingredients": [
      {
        "index": 0,
        "quantities": [
          {
            "scalable": true,
            "unit": null,
            "value": { "type": "number", "value": { "type": "regular", "value": 6.0 } }
          }
        ]
      }
    ],
    "cookware": [],
    "timers": [],
    "sections": [
      {
        "content": [
          {
            "type": "step",
            "value": {
              "items": [
                { "type": "text", "value": "Crack the " },
                { "type": "ingredient", "index": 0 },
                { "type": "text", "value": " into a blender." }
              ]
            }
          }
        ]
      }
    ]
  }
}
```

### `GET /api/recipes/raw/*path`

Read the unparsed Cooklang source

Returns the file's text verbatim with content type `text/plain`, including YAML frontmatter. The `.cook` and `.menu` extensions are optional in the path — the server tries the bare path first, then `.cook`, then `.menu`.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `path` | path | `string` | yes | Recipe path relative to the recipe directory. |

Response:

```text
---
servings: 2
tags: breakfast, quick
author: CookCLI Team
---

Crack the @eggs{3} into a blender, then add the @flour{125%g},
@milk{250%ml} and @sea salt{pinch}, and blitz until smooth.
```

### `PUT /api/recipes/*path`

Create or overwrite a recipe

The request body is the raw Cooklang source as `text/plain` — not JSON. Writes are atomic (temp file plus rename). If the file does not exist yet, it is created with a `.cook` extension — but the response's `path` echoes the request path verbatim and does not report that resolved filename. The parent directory must already exist: writing into a directory that is not there returns a 500 whose message talks about permissions even when the real cause is the missing directory.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `path` | path | `string` | yes | Recipe path relative to the recipe directory. |

Request body:

```text
---
title: New Recipe
---

Mix the @flour{200%g} and @water{120%ml}.
```

Response:

```json
{
  "path": "Breakfast/New Recipe",
  "status": "success"
}
```

### `DELETE /api/recipes/*path`

Delete a recipe file

Permanently removes the file from disk. There is no undo and no trash.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `path` | path | `string` | yes | Recipe path relative to the recipe directory. The `.cook` extension is optional — the same bare → `.cook` → `.menu` resolution as the raw endpoint applies here too. |

Response:

```json
{
  "status": "success",
  "path": "Breakfast/Old Recipe.cook"
}
```

### `GET /api/static/*path`

Fetch a recipe asset

Serves files straight from the recipe directory — this is where recipe images live. The `image` field returned by `GET /api/recipes/*path` is already a URL into this route.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `path` | path | `string` | yes | Asset path relative to the recipe directory, e.g. `Breakfast/Easy Pancakes.jpg`. |

## Menus

`.menu` files group recipes into meal plans. These endpoints return a menu's structure — days, meals, and the recipes and loose ingredients in each.

### `GET /api/menus`

List every menu

Walks the recipe tree and returns only `.menu` files. Order is not stable — the tree walk is over a hash-ordered map, and consecutive calls can return the same menus in a different order. Do not depend on it.

Response:

```json
[
  { "name": "2 Day Plan", "path": "2 Day Plan.menu" },
  { "name": "Weekly Plan", "path": "Weekly Plan.menu" }
]
```

### `GET /api/menus/*path`

Read one menu

Sections correspond to days; a `date` is extracted when the section name contains one in parentheses, e.g. `Day 1 (2026-03-04)` — the seed menus don't use that convention, so `date` is null below. A meal's `time` is likewise extracted from its header, e.g. `Breakfast (08:30):` yields `"type": "Breakfast", "time": "08:30"`; none of the seed menus set a time either, hence null throughout. Meal items are tagged by `kind`: `recipe_reference` points at another file; `ingredient` is a loose item written directly in the menu. Plain connecting text in the menu (e.g. "with") is dropped — only structured references and ingredients are returned. Returns 400 if the path is not a menu file, 404 if it does not exist. The response below is trimmed to the first of this menu's two `sections`; the second follows the same shape. A `recipe_reference`'s `scale` is a ready-to-use multiplier for the referenced recipe, resolved from the menu's `{...}` notation per the Cooklang spec: a bare `{2}` is a raw multiplier, `{3%servings}` targets servings against the referenced recipe's own `servings` metadata, any other unit targets its `yield`, and `{}` means 1. The example below shows `5.0` because the menu asks for `{10%servings}` and `Easy Pancakes` declares `servings: 2`. The `?scale` query multiplies these, so `?scale=2` yields `10.0`. `POST /api/shopping_list/add_menu` resolves references identically, so the two endpoints always agree.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `path` | path | `string` | yes | Menu path relative to the recipe directory, e.g. `2 Day Plan.menu`. |
| `scale` | query | `number` | no | Scaling factor applied to the whole menu. Defaults to 1. |

Response:

```json
{
  "name": "2 Day Plan",
  "path": "2 Day Plan.menu",
  "metadata": { "servings": "2" },
  "sections": [
    {
      "name": "Day 1",
      "date": null,
      "meals": [
        {
          "type": "Breakfast",
          "time": null,
          "items": [
            {
              "kind": "recipe_reference",
              "name": "./Breakfast/Easy Pancakes",
              "path": "./Breakfast/Easy Pancakes.cook",
              "scale": 5.0
            },
            { "kind": "ingredient", "name": "maple syrup", "quantity": "2", "unit": "tbsp" },
            { "kind": "ingredient", "name": "coffee", "quantity": "1", "unit": "c" }
          ]
        },
        {
          "type": "Lunch",
          "time": null,
          "items": [
            {
              "kind": "recipe_reference",
              "name": "./lamb-chops",
              "path": "./lamb-chops.cook",
              "scale": 1.0
            },
            { "kind": "ingredient", "name": "bread", "quantity": "2", "unit": "slices" },
            { "kind": "ingredient", "name": "butter", "quantity": "1", "unit": "tbsp" }
          ]
        },
        {
          "type": "Dinner",
          "time": null,
          "items": [
            {
              "kind": "recipe_reference",
              "name": "./Neapolitan Pizza",
              "path": "./Neapolitan Pizza.cook",
              "scale": 1.0
            },
            { "kind": "ingredient", "name": "soy sauce", "quantity": "1", "unit": "tbsp" }
          ]
        }
      ]
    }
  ]
}
```

## Shopping List

Two distinct things live here. `POST /api/shopping_list` is stateless: send recipes, get an aggregated ingredient list back. Everything else operates on the server's persistent list, stored as `.shopping-list` and `.shopping-checked` in the recipe directory. Most of the endpoints that mutate the stored list respond `200 OK` with an empty body — only `GET /api/shopping_list/items`, `GET /api/shopping_list/checked`, and the stateless `POST /api/shopping_list` return JSON. (A third GET lives under this path in the router, `/api/shopping_list/events`, but it's a Server-Sent Events stream, not JSON — see the Realtime section.)

### `POST /api/shopping_list`

Aggregate ingredients across recipes

Stateless — nothing is stored. Ingredients with the same name are combined and unit-converted, then grouped into aisle categories from `aisle.conf`; a category with no matching entries is omitted from `categories` entirely, and ingredients that match no aisle category land in `other`, sorted alphabetically. Quantities are reduced by anything in `pantry.conf`; `pantry_items` lists the ingredient names that were found there (with a nonzero or `unlim` quantity) and subtracted. `checked` echoes the server's current persistent checked state, unrelated to the recipes in this request.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `recipe` | body | `string` | yes | Recipe path. The array may hold several. |
| `scale` | body | `number` | no | Scaling factor for this recipe. Defaults to 1. |
| `included_references` | body | `string[]` | no | Which sub-recipe references to expand. Omit to include all of them. |

Request body:

```json
[
  { "recipe": "Neapolitan Pizza", "scale": 2 },
  { "recipe": "Salads/Caprese", "included_references": ["Shared/Vinaigrette"] }
]
```

Response:

```json
{
  "categories": [
    {
      "category": "fruit and veg",
      "items": [
        {
          "name": "ripe tomatoes",
          "quantities": [
            {
              "scalable": true,
              "unit": "large",
              "value": { "type": "number", "value": { "type": "regular", "value": 3.0 } }
            }
          ]
        }
      ]
    },
    {
      "category": "milk and dairy",
      "items": [
        {
          "name": "fresh mozzarella",
          "quantities": [
            {
              "scalable": false,
              "unit": "g",
              "value": { "type": "number", "value": { "type": "regular", "value": 200.0 } }
            }
          ]
        },
        {
          "name": "mozzarella cheese",
          "quantities": [
            {
              "scalable": false,
              "unit": "g",
              "value": { "type": "number", "value": { "type": "regular", "value": 200.0 } }
            }
          ]
        }
      ]
    },
    {
      "category": "tinned goods and baking",
      "items": [
        {
          "name": "tipo zero flour",
          "quantities": [
            {
              "scalable": false,
              "unit": "g",
              "value": { "type": "number", "value": { "type": "regular", "value": 1680.0 } }
            }
          ]
        },
        {
          "name": "fresh yeast",
          "quantities": [
            {
              "scalable": false,
              "unit": "g",
              "value": { "type": "number", "value": { "type": "regular", "value": 3.2 } }
            }
          ]
        }
      ]
    },
    {
      "category": "dried herbs and spices",
      "items": [
        {
          "name": "salt",
          "quantities": [
            {
              "scalable": false,
              "unit": "tsp",
              "value": {
                "type": "number",
                "value": { "type": "fraction", "value": { "whole": 0, "num": 1, "den": 8, "err": 0.0 } }
              }
            },
            {
              "scalable": false,
              "unit": "g",
              "value": { "type": "number", "value": { "type": "regular", "value": 49.2 } }
            }
          ]
        },
        {
          "name": "black pepper",
          "quantities": [
            {
              "scalable": false,
              "unit": "tsp",
              "value": { "type": "number", "value": { "type": "regular", "value": 0.0625 } }
            }
          ]
        }
      ]
    },
    {
      "category": "oils and dressings",
      "items": [
        {
          "name": "Dijon mustard",
          "quantities": [
            {
              "scalable": false,
              "unit": "tsp",
              "value": {
                "type": "number",
                "value": { "type": "fraction", "value": { "whole": 0, "num": 1, "den": 4, "err": 0.0 } }
              }
            }
          ]
        },
        {
          "name": "honey",
          "quantities": [
            {
              "scalable": false,
              "unit": "tsp",
              "value": {
                "type": "number",
                "value": { "type": "fraction", "value": { "whole": 0, "num": 1, "den": 4, "err": 0.0 } }
              }
            }
          ]
        },
        {
          "name": "red wine vinegar",
          "quantities": [
            {
              "scalable": false,
              "unit": "ml",
              "value": { "type": "number", "value": { "type": "regular", "value": 10.0 } }
            }
          ]
        }
      ]
    },
    {
      "category": "other",
      "items": [
        { "name": "basil leaves", "quantities": [] },
        {
          "name": "San Marzano tomato sauce",
          "quantities": [
            {
              "scalable": false,
              "unit": "tbsp",
              "value": { "type": "number", "value": { "type": "regular", "value": 10.0 } }
            }
          ]
        },
        { "name": "semolina", "quantities": [] }
      ]
    }
  ],
  "pantry_items": ["flour", "water", "salt", "tipo zero flour", "olive oil", "fresh basil", "black pepper"],
  "checked": []
}
```

### `GET /api/shopping_list/items`

Read the stored recipe list

Returns the recipes currently on the shopping list, not their ingredients. An entry with a `recipes` array is a menu added via `add_menu`; its nested entries carry their own resolved scale and `included_references`, independent of whatever the same recipe's standalone entry (if any) was given.

Response:

```json
[
  {
    "path": "Salads/Caprese.cook",
    "name": "Caprese",
    "scale": 2.0,
    "included_references": []
  },
  {
    "path": "2 Day Plan.menu",
    "name": "2 Day Plan",
    "scale": 1.0,
    "recipes": [
      {
        "path": "Breakfast/Easy Pancakes",
        "name": "Easy Pancakes",
        "scale": 5.0,
        "included_references": []
      },
      { "path": "lamb-chops", "name": "lamb-chops", "scale": 1.0, "included_references": [] },
      {
        "path": "Neapolitan Pizza",
        "name": "Neapolitan Pizza",
        "scale": 1.0,
        "included_references": ["Shared/Pizza Dough"]
      },
      {
        "path": "Salads/Caprese",
        "name": "Caprese",
        "scale": 1.0,
        "included_references": ["Shared/Vinaigrette"]
      },
      { "path": "Risotto", "name": "Risotto", "scale": 1.0, "included_references": [] }
    ]
  }
]
```

### `POST /api/shopping_list/add`

Add one recipe to the stored list

Responds `200 OK` with an empty body. The display name is derived from the path server-side; a client-supplied name would be discarded, so it is not accepted. The path is not checked for existence or validity: adding a recipe that doesn't exist still returns 200 and the entry lands on the list — it then makes every subsequent `POST /api/shopping_list/compact` fail with 500 until it's removed, because compact re-aggregates the whole stored list and refuses to proceed if any entry fails to parse.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `path` | body | `string` | yes | Recipe path relative to the recipe directory. |
| `scale` | body | `number` | yes | Scaling factor to store with the entry. |
| `included_references` | body | `string[]` | no | Which sub-recipe references to expand. Omit and no sub-recipes are expanded — unlike the stateless `POST /api/shopping_list`, where omitting this field means "expand all", omitting it here is not preserved through storage and reads back as an explicit empty array. Pass the reference paths explicitly if you want them expanded. |

Request body:

```json
{
  "path": "Salads/Caprese.cook",
  "scale": 2.0,
  "included_references": ["Shared/Vinaigrette"]
}
```

### `POST /api/shopping_list/add_menu`

Add every recipe in a menu

Stored as a single entry with the menu's recipes nested inside; each nested recipe's own sub-recipe references are resolved automatically (there is no `included_references` field to set here). Each nested recipe's scale is resolved from the menu reference: a bare `{2}` is a raw multiplier, `{3%servings}` targets 3 servings against the recipe's own `servings` metadata, and any other unit targets its `yield` metadata. Responds `200 OK` with an empty body; returns 404 if `path` does not exist, but does not check that it's actually a `.menu` file — pointing this at a plain recipe is accepted and stores it as if it were `POST /api/shopping_list/add` with that recipe's own references as `included_references`, which can leave the list in a state that later makes `compact` fail.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `path` | body | `string` | yes | Menu path relative to the recipe directory. |
| `scale` | body | `number` | yes | Scaling factor applied to the whole menu. |

Request body:

```json
{
  "path": "2 Day Plan.menu",
  "scale": 1.0
}
```

### `POST /api/shopping_list/remove`

Remove one recipe from the stored list

Also compacts the checked log, dropping checks for ingredients no longer referenced by any remaining recipe — best-effort; a compaction failure does not fail the remove itself. Responds `200 OK` with an empty body.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `path` | body | `string` | yes | Recipe path exactly as stored. |

Request body:

```json
{ "path": "Salads/Caprese.cook" }
```

### `POST /api/shopping_list/clear`

Empty the stored list

Removes every recipe and all checked state. Responds `200 OK` with an empty body.

### `POST /api/shopping_list/check`

Mark an ingredient as bought

Appends the name to the checked log verbatim — the server does not validate it against the current aggregated list, so any string is accepted. Use a name as returned by `POST /api/shopping_list` for it to correspond to a real ingredient. Note that `GET /api/shopping_list/checked` lowercases every name it reads back, while the aggregated list from `POST /api/shopping_list` keeps original case — a client that diffs the two sets directly will fail to match any ingredient whose name isn't already all-lowercase, e.g. checking `"Dijon mustard"` here shows up as `"dijon mustard"` from `checked`. Responds `200 OK` with an empty body.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `name` | body | `string` | yes | Aggregated ingredient name. |

Request body:

```json
{ "name": "mozzarella cheese" }
```

### `POST /api/shopping_list/uncheck`

Clear an ingredient's bought mark

Responds `200 OK` with an empty body.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `name` | body | `string` | yes | Aggregated ingredient name. |

Request body:

```json
{ "name": "mozzarella cheese" }
```

### `GET /api/shopping_list/checked`

List checked ingredient names

Returns `[]` against a fresh list — nothing is checked until `check` is called. Every name comes back lowercased, regardless of the case it was checked with or the case `POST /api/shopping_list` uses for the same ingredient in its `categories` — compare case-insensitively, or lowercase the aggregated names yourself before diffing the two. Order is not stable: the underlying set is unordered, and consecutive calls can return the same names in a different order.

Response:

```json
["tipo zero flour", "mozzarella cheese"]
```

### `POST /api/shopping_list/compact`

Drop stale checked entries

Re-aggregates the current list and removes checks for ingredients that are no longer in it. Refuses to compact (500) if any recipe fails to parse, rather than wiping checks based on a partial ingredient set. Responds `200 OK` with an empty body.

## Pantry

Reads and writes `pantry.conf`, a TOML file of what you already have at home. Quantities are typically `VALUE%UNIT`, e.g. `250%g`, but the field is just a string — `unlim` and plain counts like `12` appear untouched in the seed data. Every endpoint here returns 404 when no pantry file is configured.

### `GET /api/pantry`

Read the whole pantry

Top-level keys are section names — you choose them; `fridge`, `garden`, `pantry` and `spice rack` are just conventions used by the seed data. A bare `key = "value"` line at the top of the TOML file, outside any `[section]` header, is not an error: it is folded into a section literally named `general`, which is why it appears below even though `pantry.conf` never writes `[general]` itself. Item fields other than `name` are all optional and present only when set. The response below is trimmed to two sections of the seed pantry's five (`pantry` and `spice rack` also exist, with the same shape) to keep this example short.

Response:

```json
{
  "fridge": [
    { "name": "butter", "expire": "2026-04-15", "quantity": "250%g" },
    { "name": "eggs", "bought": "2026-03-07", "quantity": "12" },
    { "name": "milk", "quantity": "2%l" },
    { "name": "parmesan cheese", "quantity": "200%g" },
    { "name": "sour cream", "quantity": "200%ml" }
  ],
  "garden": [
    { "name": "fresh basil", "quantity": "unlim" },
    { "name": "fresh oregano", "quantity": "unlim" },
    { "name": "thyme", "quantity": "unlim" }
  ],
  "general": [
    { "name": "water", "quantity": "unlim" }
  ]
}
```

### `POST /api/pantry/add`

Add an item

Creates the section if it does not exist. The response claims success and reports a distinct item was appended even when the section already has an item of that name — but every write round-trips through a TOML serializer that keys each section's items by name, so a second item sharing a name with an existing one in the same section silently replaces it on disk rather than coexisting. Verified: adding `dup` twice with different quantities to a fresh section leaves exactly one `dup` item, with the second call's quantity.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `section` | body | `string` | yes | Section to add the item to. |
| `name` | body | `string` | yes | Item name. |
| `quantity` | body | `string` | no | Amount as `VALUE%UNIT`, or any other string such as `unlim`. |
| `bought` | body | `string` | no | Purchase date. Accepts `YYYY-MM-DD`, `DD.MM.YYYY`, `DD/MM/YYYY`, `MM/DD/YYYY`, `YYYY.MM.DD` or `DD-MM-YYYY`. |
| `expire` | body | `string` | no | Expiry date, same accepted formats as `bought`. |
| `low` | body | `string` | no | Threshold below which the item counts as running low. Compared against `quantity` only when both share the same unit; see `GET /api/pantry/depleted`. |

Request body:

```json
{
  "section": "fridge",
  "name": "yogurt",
  "quantity": "1%l",
  "low": "2%l"
}
```

Response:

```json
{
  "success": true,
  "message": "Added yogurt to fridge"
}
```

### `PUT /api/pantry/:section/:name`

Update an item

Only the fields present in the body are changed; omitted fields keep their current values. Returns 404 if the section does not exist. A name that matches nothing inside a valid section does not 404 — that case still rewrites the file (a no-op) and responds `200` with a success message naming the item that was never found. If more than one item in the section shares the target name, only the first one is updated.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `section` | path | `string` | yes | Section containing the item. |
| `name` | path | `string` | yes | Item name. |
| `quantity` | body | `string` | no | New amount. |
| `bought` | body | `string` | no | New purchase date. |
| `expire` | body | `string` | no | New expiry date. |
| `low` | body | `string` | no | New low threshold. |

Request body:

```json
{ "quantity": "500%g" }
```

Response:

```json
{
  "success": true,
  "message": "Updated butter in fridge"
}
```

### `DELETE /api/pantry/:section/:name`

Remove an item

The section is deleted too if it becomes empty. Returns 404 if the section does not exist, but — like `PUT` — responds `200` with a success message even when no item in the section actually has that name; nothing is removed and the file is rewritten unchanged.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `section` | path | `string` | yes | Section containing the item. |
| `name` | path | `string` | yes | Item name. |

Response:

```json
{
  "success": true,
  "message": "Removed butter from fridge"
}
```

### `GET /api/pantry/expiring`

List items expiring soon

Sorted most urgent first. `days_remaining` goes negative for items that have already expired, and those are always included regardless of the window — the seed pantry's `butter` (`expire = "2026-04-15"`) is already expired relative to today, so it shows up even with the default 7-day window.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `days` | query | `number` | no | Look-ahead window in days. Defaults to 7. Negative values return 400. |

Response:

```json
[
  {
    "section": "fridge",
    "name": "butter",
    "expire": "2026-04-15",
    "days_remaining": -116
  }
]
```

### `GET /api/pantry/depleted`

List items running low

An item counts as low when its `quantity` has fallen to or below its `low` threshold and the two share the same unit. Returns `[]` against the unmodified seed pantry, which has no `low` field set on anything.

Response:

```json
[
  { "section": "fridge", "name": "yogurt", "low": "2%l" }
]
```

## Search & Stats

Collection-wide queries.

### `GET /api/search`

Full-text recipe search

Matches against recipe names and content. Menus are searched alongside recipes. `q` is required: omitting it entirely returns a plain-text 400 from axum's query deserializer (`Failed to deserialize query string: missing field q`), not the page's usual JSON error envelope — the same shape as the `scale` parameter's failure mode on `GET /api/recipes/*path`. A present but empty `q=` is not rejected, though: it matches everything and returns the whole collection.

| Name | In | Type | Required | Description |
|------|----|------|----------|-------------|
| `q` | query | `string` | yes | Search term. Omitting the parameter returns 400; an empty value matches everything. |

Response:

```json
[
  { "name": "Neapolitan Pizza", "path": "Neapolitan Pizza.cook" },
  { "name": "Pizza Dough", "path": "Shared/Pizza Dough.cook" },
  { "name": "2 Day Plan", "path": "2 Day Plan.menu" },
  { "name": "Weekly Plan", "path": "Weekly Plan.menu" }
]
```

### `GET /api/stats`

Collection counts

Pantry counts are all zero when no pantry file is configured, or when it fails to read or parse. `pantry_expiring_count` uses a fixed 7-day window regardless of what `GET /api/pantry/expiring?days=` would be called with.

Response:

```json
{
  "recipe_count": 12,
  "menu_count": 2,
  "pantry_item_count": 29,
  "pantry_expiring_count": 1,
  "pantry_depleted_count": 0
}
```

### `GET /api/reload`

Reload recipes (no-op)

Kept for client compatibility. The server reads from disk on every request, so there is no cache to clear and this endpoint does nothing besides log and return this fixed response.

Response:

```json
{
  "status": "success",
  "message": "Recipes will be refreshed from disk on next request"
}
```

### `POST /api/reload`

Reload recipes (no-op)

Identical to the GET form; both verbs are registered on the same route and accepted.

Response:

```json
{
  "status": "success",
  "message": "Recipes will be refreshed from disk on next request"
}
```

## Realtime

Long-lived connections. Neither of these returns a normal JSON response.

### `GET /api/shopping_list/events`

Server-sent events for shopping list changes

Emits a `change` event whenever `.shopping-list` or `.shopping-checked` is modified on disk — including by another client or by the `cook` CLI. The event's `file` field is `"list"` or `"checked"`, naming which file changed (captured live below by editing the shopping list in a second shell while connected). It is not a snapshot of what changed, so the intended pattern is to re-fetch the list on each event rather than to apply a diff. A `ping` keep-alive comment is sent every 30 seconds. If the filesystem watcher failed to start, the stream still connects and returns 200 but never emits an event.

Response:

```text
event: change
data: {"file":"list"}

event: change
data: {"file":"checked"}
```

### `GET /api/ws/lsp`

Language server bridge (websocket)

Upgrades to a websocket (verified: a plain WebSocket handshake against this path returns `101 Switching Protocols`) that bridges to a `cook lsp` subprocess, providing diagnostics and completions to the built-in editor. Messages are Language Server Protocol messages framed with `Content-Length` headers exactly as LSP over stdio would be — see the LSP specification for the format. Not a REST endpoint and not usable with a plain HTTP client.

## Sync

Sign in to CookCloud and sync recipes across devices. These four endpoints exist only when CookCLI is built with the `sync` feature, which is on by default — the badge on each entry marks that. Because the router is read as source text (`include_str!`), they are documented unconditionally rather than silently disappearing from this page in a build that lacks the feature. Authentication uses an OAuth device-code flow: start a login, show the user the code, then poll status until it completes.

### `GET /api/sync/status`

Current sync and login state *(requires a build with the `sync` feature)*

`pending_login` is non-null while a device-code login is in progress; poll this endpoint to detect completion. `expires_in_secs` counts down to when the pending login expires. Captured live against a fresh server with no session.

Response:

```json
{
  "logged_in": false,
  "email": null,
  "syncing": false,
  "pending_login": null
}
```

### `POST /api/sync/login`

Start a device-code login *(requires a build with the `sync` feature)*

Illustrative response — derived field-by-field from `LoginResponse` in `src/server/handlers/sync.rs`, not captured from a completed cook.md exchange (that needs real network and credentials, which this reference was not authenticated against). Show `user_code` to the user and send them to `verification_uri`, or open `verification_uri_complete`, which pre-fills the code. Takes no request body. Returns 400 if already logged in, 409 if a login is already in progress, and 502 if cook.md is unreachable — the 502 path was verified live, by calling this endpoint with no network reachable: it returned `502` with `{"error": "cook.md unreachable: ..."}`.

Response:

```json
{
  "user_code": "ABCD-EFGH",
  "verification_uri": "https://cook.md/device",
  "verification_uri_complete": "https://cook.md/device?code=ABCD-EFGH",
  "expires_in_secs": 900
}
```

### `POST /api/sync/cancel_login`

Abandon a pending login *(requires a build with the `sync` feature)*

`cancelled` is false when there was no login in progress — captured live against a fresh server with nothing pending. Takes no request body.

Response:

```json
{ "cancelled": false }
```

### `POST /api/sync/logout`

Sign out and stop syncing *(requires a build with the `sync` feature)*

Clears the stored session and halts the background sync task. Always responds `200` with `{"ok": true}`, even when no session exists — captured live against a fresh server with no session file. Takes no request body.

Response:

```json
{ "ok": true }
```

---

This file is generated from `src/web/api_docs.rs`. Edit that, then run `UPDATE_API_DOCS=1 cargo test --test api_docs_md_test` to regenerate.
