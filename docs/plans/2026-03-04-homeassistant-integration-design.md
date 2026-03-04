# CookCLI + Home Assistant Integration Design

## Overview

A HACS custom component (`homeassistant-cookcli`) in a separate repo that communicates with CookCLI's REST API. Provides Calendar (meal plans from `.menu` files), Todo (shopping list), and Sensor entities. Requires CookCLI server running.

This design covers both sides: the HA custom component (Python) and the CookCLI API additions (Rust) needed to support it.

---

## Part 1: CookCLI API Additions

### New Endpoints

#### `GET /api/stats`
Overview statistics for sensor entities.

```json
{
  "recipe_count": 42,
  "menu_count": 3,
  "pantry_item_count": 87,
  "pantry_expiring_count": 5,
  "pantry_depleted_count": 2
}
```

Implementation: Count entries from `cooklang_find::build_tree()`, parse pantry.conf for expiring/depleted counts (reuse logic from `src/pantry.rs`).

---

#### `GET /api/menus`
List all `.menu` files.

```json
[
  {
    "name": "2 Day Plan",
    "path": "2 Day Plan.menu"
  },
  {
    "name": "Weekly Meal Prep",
    "path": "plans/Weekly Meal Prep.menu"
  }
]
```

Implementation: Filter `cooklang_find` entries where `entry.is_menu() == true`.

---

#### `GET /api/menus/{path}`
Parsed menu with sections and resolved recipe references.

```json
{
  "name": "2 Day Plan",
  "path": "2 Day Plan.menu",
  "metadata": {
    "servings": "2"
  },
  "sections": [
    {
      "name": "Day 1",
      "meals": [
        {
          "type": "Breakfast",
          "items": [
            {
              "kind": "recipe_reference",
              "name": "Easy Pancakes",
              "path": "Breakfast/Easy Pancakes.cook",
              "scale": 0.1
            },
            {
              "kind": "ingredient",
              "name": "maple syrup",
              "quantity": "2",
              "unit": "tbsp"
            }
          ]
        },
        {
          "type": "Lunch",
          "items": [...]
        },
        {
          "type": "Dinner",
          "items": [...]
        }
      ]
    },
    {
      "name": "Day 2",
      "meals": [...]
    }
  ]
}
```

Implementation: Extract and restructure the parsing logic from `menu_page_handler()` in `src/server/ui.rs` into a reusable function, add JSON serialization via serde.

---

#### `GET /api/menus/{path}/shopping`
Aggregated shopping list for an entire menu plan.

```json
{
  "categories": [
    {
      "category": "Produce",
      "items": [
        { "name": "onion", "quantities": [{ "value": 2, "unit": null }] }
      ]
    }
  ],
  "pantry_items": ["salt", "pepper"]
}
```

Implementation: Resolve all recipe references in the menu, aggregate their ingredients (reuse `shopping_list` handler logic), apply aisle categorization.

---

#### `GET /api/pantry/expiring?days=7`
Items expiring within N days.

```json
[
  {
    "section": "Dairy",
    "name": "milk",
    "expire": "2026-03-08",
    "days_remaining": 4
  }
]
```

Implementation: Reuse logic from `src/pantry.rs` `expiring` subcommand, expose as API.

---

#### `GET /api/pantry/depleted`
Out-of-stock items.

```json
[
  {
    "section": "Dairy",
    "name": "butter",
    "low": "100g"
  }
]
```

Implementation: Reuse logic from `src/pantry.rs` `depleted` subcommand.

---

### Existing Endpoints (No Changes Needed)

These already work for the HA integration:
- `GET /api/recipes` - List recipes
- `GET /api/recipes/*path?scale=N` - Scaled recipe
- `GET /api/search?q=term` - Search
- `GET /api/shopping_list/items` - Current shopping list
- `POST /api/shopping_list/add` - Add to shopping list
- `POST /api/shopping_list/remove` - Remove from shopping list
- `POST /api/shopping_list/clear` - Clear shopping list
- `GET /api/pantry` - Full pantry

---

## Part 2: HACS Custom Component

### Repository: `cooklang/homeassistant-cookcli`

### File Structure

```
custom_components/cookcli/
├── __init__.py           # Integration setup, service registration
├── manifest.json         # HACS metadata
├── const.py              # Constants (DOMAIN, defaults)
├── config_flow.py        # UI setup wizard
├── coordinator.py        # DataUpdateCoordinator (5-min polling)
├── api.py                # CookCLI REST API client
├── calendar.py           # Meal plan calendar entity
├── todo.py               # Shopping list todo entity
├── sensor.py             # Stats and pantry sensors
├── services.yaml         # Service definitions
├── strings.json          # English strings
└── translations/
    └── en.json           # Translations
```

### manifest.json

```json
{
  "domain": "cookcli",
  "name": "CookCLI",
  "codeowners": ["@cooklang"],
  "config_flow": true,
  "documentation": "https://github.com/cooklang/homeassistant-cookcli",
  "issue_tracker": "https://github.com/cooklang/homeassistant-cookcli/issues",
  "integration_type": "service",
  "iot_class": "local_polling",
  "requirements": ["aiohttp>=3.8.0"],
  "version": "0.1.0",
  "homeassistant": "2024.1.0"
}
```

Key: `iot_class: "local_polling"` -- CookCLI runs on the local network, no cloud.

### Config Flow

Setup wizard asks for:
1. **CookCLI Server URL** (default: `http://localhost:9080`)
2. **Active Menu** (optional) -- dropdown of available `.menu` files, fetched from `/api/menus`

```
┌─────────────────────────────┐
│  CookCLI Integration Setup  │
│                             │
│  Server URL:                │
│  [http://localhost:9080   ] │
│                             │
│  Active Menu Plan:          │
│  [2 Day Plan           ▼ ] │
│  (optional)                 │
│                             │
│  [Submit]                   │
└─────────────────────────────┘
```

Validation: Calls `GET /api/stats` to verify connectivity.

### DataUpdateCoordinator

Single coordinator polling CookCLI every 5 minutes.

```python
class CookCLICoordinator(DataUpdateCoordinator):
    update_interval = timedelta(minutes=5)

    async def _async_update_data(self):
        stats = await self.api.get_stats()
        shopping_items = await self.api.get_shopping_list_items()
        pantry_expiring = await self.api.get_pantry_expiring(days=7)
        pantry_depleted = await self.api.get_pantry_depleted()
        menu = await self.api.get_menu(self.active_menu) if self.active_menu else None

        return {
            "stats": stats,
            "shopping_items": shopping_items,
            "pantry_expiring": pantry_expiring,
            "pantry_depleted": pantry_depleted,
            "menu": menu,
        }
```

### Entities

#### Calendar: `calendar.cookcli_meal_plan`

Maps `.menu` file sections to calendar events.

**Mapping logic:**
- Section names can optionally contain a date in parentheses: `== Day 1 (2026-03-04) ==`
- Date extracted via regex: `\((\d{4}-\d{2}-\d{2})\)` from the section name
- Sections with a date are mapped to calendar events on that date
- Each meal type within a dated section (Breakfast, Lunch, Dinner, Snack, etc.) becomes a calendar event
- Sections without a date (e.g., `==Extras==`, `==Day 1==`) are ignored for calendar -- still included in shopping lists and recipe browsing
- Dates can be non-consecutive (skip days)
- Event summary = meal type, description = list of recipes/ingredients

**Menu file example:**
```
---
servings: 2
---

== Day 1 (2026-03-04) ==
Breakfast (08:30):
- @./Breakfast/Easy Pancakes{10%servings}

Dinner:
- @./Neapolitan Pizza{}

== Day 3 (2026-03-06) ==
Lunch (12:30):
- @./Sicilian-style Scottadito Lamb Chops{}

Snack:
- @crackers{1%box} with @hummus{1%cup}

== Extras ==
- @soy sauce{1%tbsp}
```

**Extraction rules:**
- Section date: regex `\((\d{4}-\d{2}-\d{2})\)` on section name
- Meal time: regex `\((\d{2}:\d{2})\)` on meal type header
- If no time in parentheses, fall back to defaults: Breakfast 07:00, Lunch 12:00, Dinner 18:00, Snack 15:00, unknown = all-day event

**Calendar mapping:**
```
Section "Day 1 (2026-03-04)"   →  date: 2026-03-04
  Breakfast (08:30):           →  CalendarEvent(summary="Breakfast",
    - Easy Pancakes                description="Easy Pancakes",
                                   start=2026-03-04 08:30, end=09:30)
  Dinner:                      →  CalendarEvent(summary="Dinner",
    - Neapolitan Pizza             description="Neapolitan Pizza",
                                   start=2026-03-04 18:00, end=19:00)
                                   ↑ no time specified, uses default

Section "Day 3 (2026-03-06)"   →  date: 2026-03-06
  Lunch (12:30):               →  CalendarEvent(summary="Lunch",
    - Scottadito Lamb Chops        description="Sicilian-style Scottadito Lamb Chops",
                                   start=2026-03-06 12:30, end=13:30)
  Snack:                       →  CalendarEvent(summary="Snack",
    - crackers, hummus             description="crackers, hummus",
                                   start=2026-03-06 15:00, end=15:30)

Section "Extras"               →  no date, ignored for calendar, still in shopping list
```

**Read-only initially.** Menu files are plain text -- editing via HA calendar UI would require a write-back API that preserves Cooklang syntax. This can be added later.

---

#### Todo: `todo.cookcli_shopping_list`

Bidirectional sync with CookCLI's shopping list.

**Entity features:**
- `CREATE_TODO_ITEM` -- Add recipe to shopping list via `POST /api/shopping_list/add`
- `DELETE_TODO_ITEM` -- Remove via `POST /api/shopping_list/remove`

**Todo items:** Each recipe in the shopping list becomes a TodoItem:
```python
TodoItem(
    uid=item["path"],
    summary=f"{item['name']} (x{item['scale']})",
    status=TodoItemStatus.NEEDS_ACTION,
)
```

**Clear all:** Exposed as a service call `cookcli.clear_shopping_list`.

---

#### Sensors

| Entity ID | Value | Attributes |
|---|---|---|
| `sensor.cookcli_recipe_count` | `42` | `menu_count: 3` |
| `sensor.cookcli_pantry_expiring` | `5` | `items: [{name, section, expire, days_remaining}]` |
| `sensor.cookcli_pantry_depleted` | `2` | `items: [{name, section, low}]` |
| `sensor.cookcli_todays_meal` | `"Neapolitan Pizza"` | `meal_type: "Dinner"`, `recipe_path: "..."` |

`sensor.cookcli_todays_meal` derives from the active menu's calendar mapping -- shows the next upcoming meal.

---

### Services

#### `cookcli.search_recipe`

```yaml
search_recipe:
  description: Search recipes by keyword
  fields:
    query:
      description: Search term
      required: true
      selector:
        text:
```

Returns results as a response variable (HA 2024.7+ `response_variable` feature).

#### `cookcli.add_menu_to_shopping_list`

```yaml
add_menu_to_shopping_list:
  description: Add all ingredients from a menu plan to the shopping list
  fields:
    menu:
      description: Menu file path
      required: true
      selector:
        text:
    scale:
      description: Scale factor
      required: false
      default: 1
      selector:
        number:
          min: 0.5
          max: 10
          step: 0.5
```

Calls `GET /api/menus/{path}` to resolve all recipes, then `POST /api/shopping_list/add` for each.

#### `cookcli.add_recipe_to_shopping_list`

```yaml
add_recipe_to_shopping_list:
  description: Add a single recipe's ingredients to the shopping list
  fields:
    recipe:
      description: Recipe path
      required: true
      selector:
        text:
    scale:
      description: Scale factor
      required: false
      default: 1
      selector:
        number:
          min: 0.5
          max: 10
          step: 0.5
```

#### `cookcli.clear_shopping_list`

```yaml
clear_shopping_list:
  description: Clear all items from the shopping list
```

---

### Automation Examples

Users can build these with the integration:

**Morning menu brief:**
```yaml
automation:
  - alias: "Morning Meal Brief"
    trigger:
      - platform: time
        at: "07:00:00"
    action:
      - service: notify.mobile_app
        data:
          title: "Today's Meals"
          message: "{{ states('sensor.cookcli_todays_meal') }}"
```

**Pantry expiration alert:**
```yaml
automation:
  - alias: "Pantry Expiration Warning"
    trigger:
      - platform: numeric_state
        entity_id: sensor.cookcli_pantry_expiring
        above: 0
    action:
      - service: notify.mobile_app
        data:
          title: "Pantry Alert"
          message: >
            {{ state_attr('sensor.cookcli_pantry_expiring', 'items') | length }}
            items expiring soon
```

**Weekly shopping list from menu:**
```yaml
automation:
  - alias: "Sunday Meal Prep Shopping"
    trigger:
      - platform: time
        at: "09:00:00"
    condition:
      - condition: time
        weekday: [sun]
    action:
      - service: cookcli.add_menu_to_shopping_list
        data:
          menu: "Weekly Meal Prep.menu"
          scale: 1
```

---

## Data Flow

```
┌─────────────────┐     HTTP/JSON      ┌──────────────────┐
│  Home Assistant  │◄──────────────────►│  CookCLI Server  │
│                  │   5-min polling    │  (localhost:9080) │
│  ┌────────────┐  │                    │                  │
│  │Coordinator │──┼── GET /api/stats   │  .cook files     │
│  │ (5 min)    │──┼── GET /api/menus/* │  .menu files     │
│  │            │──┼── GET /api/sl/items│  pantry.conf     │
│  │            │──┼── GET /api/pantry/*│  aisle.conf      │
│  └─────┬──────┘  │                    └──────────────────┘
│        │         │
│  ┌─────▼──────┐  │
│  │ Calendar   │  │  .menu sections → calendar events
│  │ Todo       │  │  shopping list ↔ todo items
│  │ Sensors    │  │  stats, pantry alerts
│  └────────────┘  │
└─────────────────┘
```

---

## What's NOT in MVP

- **Calendar write-back** -- Editing menu files from HA calendar UI (complex: need to preserve Cooklang syntax)
- **Recipe browsing entity** -- A Select entity for recipe picker (useful but not essential)
- **MQTT support** -- Overkill for data that changes infrequently
- **HA Add-on** -- Packaging CookCLI as an add-on is a separate effort
- **Webhook/push** -- HA polls; push notifications can come later
- **Dashboard cards** -- Custom Lovelace cards are a separate project
- **Multi-instance** -- Supporting multiple CookCLI servers (one is enough for MVP)

---

## Implementation Order

1. **CookCLI API endpoints** -- Add `/api/stats`, `/api/menus`, `/api/menus/{path}`, `/api/menus/{path}/shopping`, `/api/pantry/expiring`, `/api/pantry/depleted`
2. **HA component scaffold** -- Separate repo, manifest, config flow, coordinator, API client
3. **Sensors** -- Recipe count, pantry expiring, pantry depleted (simplest entities)
4. **Todo** -- Shopping list as HA Todo (bidirectional)
5. **Calendar** -- Menu-to-calendar mapping (most complex)
6. **Services** -- Search, add to shopping list, clear shopping list
7. **Documentation & HACS submission**
