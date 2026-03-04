# CookCLI + Home Assistant Integration Research

## Why Home Assistant?

Home Assistant (HA) has 2M+ active installations and a thriving ecosystem of integrations. Kitchen/food management is an emerging category with proven demand (Mealie: 2,079+ active installations, Grocy, Bring! Shopping List). CookCLI currently has no HA integration, representing a clear market opportunity.

## Landscape: Popular HA Integrations & What Drives Them

### Top Integration Patterns

| Integration | Type | What It Does | Why It's Popular |
|---|---|---|---|
| MQTT | Built-in | Message broker for IoT | 44% adoption, real-time, universal protocol |
| Alarmo | HACS | Alarm panel replacement | Solves real pain point, beautiful UI |
| Frigate | HACS | NVR with object detection | Local AI, no cloud, privacy-first |
| Local Tuya | HACS | Cloud-free Tuya devices | Removes cloud dependency |
| Powercalc | HACS | Energy monitoring | Saves money, visible daily impact |
| Adaptive Lighting | HACS | Circadian lighting | Set-and-forget automation |
| Browser Mod | HACS | Dashboard customization | Enables custom UI patterns |
| Mushroom Cards | HACS | Modern card collection | Visual polish, daily dashboard use |

### What Makes Integrations Sticky (Retention Drivers)

1. **Solves a specific pain point** - Not "nice to have" but "I need this daily"
2. **Dashboard visibility** - Users see it every day on their HA dashboard
3. **Automation foundation** - Users build personal automations around it, creating lock-in
4. **Proactive notifications** - Push alerts, not just passive display
5. **Low friction updates** - HACS auto-update mechanism
6. **Reliability** - Graceful error handling, doesn't slow down HA
7. **Local-first** - No cloud dependency preferred by the HA community
8. **Community & docs** - Active maintainer, shared usage patterns and blueprints

### Integration Architecture Patterns

Successful integrations follow these HA patterns:

- **DataUpdateCoordinator** - Centralized polling that prevents API hammering (5-15 min intervals typical)
- **Config Flow** - UI-based setup wizard (YAML-only setup is discouraged since 2024)
- **Entity diversity** - Combine multiple entity types (sensor + todo + calendar) for richer experience
- **Services** - Expose actions that automations can call
- **Diagnostics** - Built-in troubleshooting data export

### Integration Types & Trade-offs

| Type | Dev Time | Distribution | Quality | Best For |
|---|---|---|---|---|
| REST Sensor (YAML) | 1 week | Manual | Basic | Quick prototype |
| HACS Custom Component | 1-2 months | HACS store | Good | Community distribution |
| Official HA Core | 6-12 months | Built-in | Excellent | Maximum reach |
| Add-on | 2-4 months | Supervisor | Varies | Running CookCLI server |

## Existing Food/Kitchen Integrations

### Mealie (Official since HA 2024.7)
- **What:** Self-hosted recipe manager with meal planning
- **HA entities:** Calendar (meal plans), Todo (shopping lists), Sensor (statistics)
- **Polling:** 5-minute DataUpdateCoordinator
- **Key insight:** Meal plan calendar + shopping list todo is the killer combo
- **Limitation:** Heavy (full web app), requires separate server

### Grocy (HACS Custom)
- **What:** Inventory/pantry management
- **HA entities:** Sensors (stock levels, expiring items), Binary sensors (overdue items)
- **Key insight:** Expiration tracking + low-stock alerts drive daily engagement
- **Limitation:** Complex setup, overkill for many users

### Bring! Shopping List
- **What:** Collaborative shopping list app
- **HA entities:** Todo lists (bidirectional sync)
- **Key insight:** Family sharing + phone notifications = daily use

### Tandoor Recipes
- **No official integration** - Only workarounds via API scripts
- **Gap:** Shows unmet demand for recipe integration beyond Mealie

## CookCLI's Unique Position

CookCLI is fundamentally different from Mealie/Grocy:

| Aspect | Mealie/Grocy | CookCLI |
|---|---|---|
| Architecture | Heavy web app, database | Lightweight CLI, plain files |
| Recipe storage | Database | `.cook` files (version-controllable) |
| Setup | Docker container | Single binary |
| Philosophy | All-in-one platform | UNIX composability |
| Scaling | Per-deployment | Per-recipe, dynamic |
| Meal planning | Database-driven | `.menu` files (plain text, composable) |
| Import | Web scraping | Web + Cooklang ecosystem |

**Positioning:** CookCLI is not a Mealie competitor but covers surprisingly similar ground. With `.menu` files for meal planning and pantry tracking, CookCLI already has the core features -- just in a lightweight, file-based form.

### Menu Files: CookCLI's Meal Planning

CookCLI supports `.menu` files -- plain-text meal plans that reference recipes with scaling. This is a **major differentiator** for HA integration because it provides structured meal planning data without a database.

**Menu file format:**
```
---
servings: 2
---

==Day 1==

Breakfast:
- @./Breakfast/Easy Pancakes{10%servings} with @maple syrup{2%tbsp}

Lunch:
- @./Sicilian-style Scottadito Lamb Chops{}
- @bread{2%slices}(toasted) with @butter{1%tbsp}

Dinner:
- @./Neapolitan Pizza{}
```

**Key capabilities:**
- **Sections** (`==Day 1==`) map naturally to HA calendar days
- **Meal types** (Breakfast, Lunch, Dinner) map to calendar event categories
- **Recipe references** (`@./Recipe Name{scale}`) link to full recipes with scaling
- **Inline ingredients** (`@ingredient{qty%unit}`) for simple additions
- **Menu-level scaling** cascades to all referenced recipes
- **Shopping list integration** -- aggregate all ingredients across the full plan
- **Web UI support** -- menus render in the server with clickable recipe links

## Integration Ideas

### Approach A: HACS Custom Component (Recommended for MVP)

A Python-based custom component that communicates with CookCLI's REST API.

**Prerequisites:** CookCLI server running (either as HA add-on or standalone).

**Entities:**
- `sensor.cookcli_recipe_count` - Total recipes
- `sensor.cookcli_featured_recipe` - Random/daily recipe suggestion
- `todo.cookcli_shopping_list` - Shopping list as HA Todo entity (bidirectional)
- `calendar.cookcli_meal_plan` - Meal plan from `.menu` files mapped to HA calendar
- `select.cookcli_recipe_picker` - Dropdown to browse/select recipes
- `sensor.cookcli_pantry_expiring` - Items expiring soon
- `binary_sensor.cookcli_pantry_low_stock` - Low stock alert

**Services:**
- `cookcli.search_recipe` - Search recipes by keyword
- `cookcli.scale_recipe` - Get scaled ingredient list
- `cookcli.add_to_shopping_list` - Add recipe ingredients to shopping list
- `cookcli.clear_shopping_list` - Clear shopping list

**Automations users could build:**
- "At 6pm, show today's meal from the active menu plan on kitchen dashboard"
- "When pantry item expires in 3 days, send notification"
- "When I say 'Hey Google, what's for dinner', read tonight's dinner from the menu"
- "Every Sunday, generate shopping list from this week's menu and send to phone"
- "When menu plan changes, update the HA calendar automatically"
- "Notify family members of today's meals each morning"

**Pros:** Fastest to build, HACS distribution, community expects this pattern
**Cons:** Requires CookCLI server running, Python (not Rust)

### Approach B: HA Add-on + Custom Component

Package CookCLI server as an HA Supervisor add-on, paired with a custom component.

**Add-on:** Runs `cook server` inside HA's Docker-based add-on system, mounting recipe directory as a volume.

**Custom Component:** Same as Approach A, but auto-discovers the add-on (no manual URL config).

**Pros:** One-click install, no external server needed, self-contained
**Cons:** Requires HA Supervisor (not all installs), Docker packaging, more maintenance

### Approach C: MQTT Bridge

A lightweight bridge that publishes CookCLI data to MQTT topics.

**Topics:**
```
cookcli/recipes/count
cookcli/recipes/featured
cookcli/pantry/expiring
cookcli/pantry/low_stock
cookcli/shopping_list/items
```

**Pros:** Real-time updates, works with any MQTT consumer, very HA-native
**Cons:** Requires MQTT broker, more moving parts, overkill for recipe data that changes infrequently

### Approach D: REST Sensor (Zero-Code Quick Start)

Users configure HA's built-in REST sensor to poll CookCLI API directly. No custom code needed.

```yaml
# Example HA configuration.yaml
sensor:
  - platform: rest
    resource: http://localhost:9080/api/recipes
    name: CookCLI Recipes
    value_template: "{{ value_json | length }}"
    scan_interval: 300

rest_command:
  cookcli_add_shopping:
    url: http://localhost:9080/api/shopping_list/add
    method: POST
    content_type: application/json
    payload: '{"recipe": "{{ recipe }}", "scale": {{ scale | default(1) }}}'
```

**Pros:** Zero development, users can start today, documentation-only effort
**Cons:** Limited functionality, no config flow, no HACS distribution, manual setup

## Recommended Strategy

### Phase 1: Documentation + REST Sensor Templates (1-2 weeks)
- Write a guide showing users how to connect CookCLI to HA using REST sensors
- Provide copy-paste YAML configurations for common use cases
- Zero code changes to CookCLI needed
- Validates demand before investing in custom component

### Phase 2: HA Add-on (2-4 weeks)
- Package CookCLI as an HA Supervisor add-on
- Users install from HACS or custom repository
- Mounts recipe directory, runs `cook server`
- Makes CookCLI accessible within HA network

### Phase 3: HACS Custom Component (4-8 weeks)
- Python custom component with config flow
- DataUpdateCoordinator polling CookCLI API
- Entities: sensor, todo (shopping list), select (recipe picker)
- Services for search, scale, shopping list management
- Auto-discovery of add-on if installed

### Phase 4: Official HA Integration (6-12 months)
- Submit to HA core after community validation
- Meet HA quality scale requirements (Silver tier minimum)
- Full test coverage, documentation, translations

## Key CookCLI API Gaps for HA Integration

Current API endpoints that work well:
- `GET /api/recipes` - List recipes
- `GET /api/recipes/*path?scale=N` - Get scaled recipe
- `GET /api/search?q=term` - Search
- `POST /api/shopping_list/add` - Add to shopping list
- `GET /api/pantry` - Pantry items

Missing/needed for good HA integration:
- **`GET /api/stats`** - Recipe count, last modified, total ingredients (for sensors)
- **`GET /api/recipes/random`** - Random recipe suggestion (for daily featured recipe)
- **`GET /api/menus`** - List all menu files
- **`GET /api/menus/*path`** - Get parsed menu with sections, meal types, and resolved recipe references
- **`GET /api/menus/*path/shopping_list`** - Aggregated shopping list for an entire menu plan
- **`GET /api/pantry/expiring?days=N`** - Expiring items endpoint (exists in CLI, not in API)
- **`GET /api/pantry/depleted`** - Out-of-stock items (exists in CLI, not in API)
- **Webhook/SSE support** - Push notifications when recipes/menus change (nice-to-have)

## Automation Blueprint Ideas

HA Blueprints are shareable automation templates. These would drive adoption:

1. **"What's for Dinner?"** - Read today's dinner from the active `.menu` plan and notify at configurable time
2. **"Pantry Watchdog"** - Alert when items expire within N days
3. **"Sunday Meal Prep"** - Generate shopping list from the active menu plan every Sunday
4. **"Kitchen Timer"** - Parse timer steps from recipe, create HA timer entities
5. **"Smart Grocery Run"** - When leaving home (zone trigger), send menu shopping list to phone
6. **"Voice Recipe"** - Read recipe steps via TTS on smart speaker
7. **"Morning Menu Brief"** - Announce today's meals from the menu plan each morning
8. **"Menu Rotation"** - Cycle through multiple `.menu` files on a weekly/monthly schedule

## Dashboard Card Ideas

Custom Lovelace cards that would make CookCLI visible daily:

1. **Recipe of the Day** - Card showing featured recipe with image, ingredients count, cook time
2. **Pantry Status** - Gauge/badge showing stock levels, expiring items highlighted
3. **Shopping List** - Interactive checklist synced with CookCLI
4. **Quick Cook** - Buttons for favorite recipes that open full recipe view
5. **Meal Calendar** - Week view from active `.menu` file, showing meals per day with recipe links

## Competitive Analysis Summary

| Feature | Mealie+HA | Grocy+HA | CookCLI+HA (proposed) |
|---|---|---|---|
| Recipe browsing | Yes | No | Yes |
| Shopping list sync | Yes (todo) | No | Yes (todo) |
| Pantry tracking | No | Yes | Yes |
| Meal planning | Yes (calendar) | Yes | Yes (`.menu` files as calendar) |
| Recipe scaling | Limited | No | Yes (native strength) |
| Ingredient aggregation | Basic | No | Yes (native strength) |
| Recipe import | Yes | No | Yes |
| File-based recipes | No | No | Yes (unique) |
| Setup complexity | Medium | High | Low |
| Resource usage | High | High | Low |

## Menu Files as the HA Integration Centerpiece

The `.menu` file format maps remarkably well to HA's entity model. This is CookCLI's strongest differentiator vs Mealie/Grocy for HA integration.

### Natural Entity Mapping

```
.menu file                    HA Entity
─────────────────────────────────────────────
==Day 1==                  →  Calendar event date
Breakfast: / Lunch: / ...  →  Calendar event category/title
- @./Recipe{}              →  Calendar event description + link
- @ingredient{qty%unit}    →  Todo list item (shopping)
Full menu ingredients      →  Todo entity (aggregated shopping list)
Active menu file           →  Select entity (menu picker)
```

### How It Works

1. User creates/edits `.menu` files (plain text, git-friendly)
2. CookCLI server parses menus with section structure and recipe references
3. HA integration polls CookCLI API, maps sections to calendar days
4. Shopping list generated from the full menu becomes an HA Todo entity
5. Automations trigger based on calendar events ("dinner starting in 1 hour")

### Why This Beats Mealie's Calendar

- **Plain text** - Edit meal plans in any text editor, version control with git
- **Recipe scaling built-in** - `@./Recipe{2%servings}` right in the plan
- **Composable** - Mix recipe references with ad-hoc ingredients
- **Multiple plans** - Switch between "Weeknight Quick", "Holiday Feast", "Meal Prep Week"
- **No database** - No migration headaches, no backup complexity

### Menu API Endpoints Needed

To fully support the calendar integration, CookCLI's API needs:

```
GET /api/menus                    → List all .menu files
GET /api/menus/{path}             → Parsed menu with sections and resolved recipes
GET /api/menus/{path}/calendar    → Menu as iCal/structured calendar events
GET /api/menus/{path}/shopping    → Aggregated shopping list for the full plan
PUT /api/menus/{path}             → Save/update menu file
```

## Conclusion

The HA integration opportunity for CookCLI is real and differentiated. The lightweight, file-based, UNIX-philosophy approach fills a gap between heavy platforms (Mealie) and no-solution (manual YAML). Starting with REST sensor documentation (Phase 1) validates demand at zero cost, while the HACS custom component (Phase 3) is the target for a proper integration.

The killer features for HA would be: **menu plans as HA Calendar** (daily visibility, the centerpiece), **shopping list as HA Todo entity** (daily use), **pantry expiration alerts** (proactive notifications), and **recipe scaling services** (unique to CookCLI). The `.menu` file format is CookCLI's secret weapon -- it provides structured meal planning data that maps perfectly to HA's calendar entity, all from plain text files.
