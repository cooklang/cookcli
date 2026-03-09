# HomeAssistant Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add CookCLI API endpoints for HA integration, then build a HACS custom component in `../homeassistant-cookcli` that exposes Calendar, Todo, and Sensor entities.

**Architecture:** CookCLI server gets new REST endpoints (`/api/stats`, `/api/menus`, `/api/pantry/expiring`, `/api/pantry/depleted`). A Python HACS component polls these endpoints via DataUpdateCoordinator every 5 minutes, exposing HA Calendar (from `.menu` files), Todo (shopping list), and Sensor entities.

**Tech Stack:** Rust/Axum (CookCLI API), Python 3.12+ (HA component), aiohttp (HTTP client), Home Assistant Core APIs (CalendarEntity, TodoListEntity, SensorEntity, DataUpdateCoordinator)

**Design doc:** `docs/plans/2026-03-04-homeassistant-integration-design.md`

---

## Part 1: CookCLI API Endpoints (Rust)

### Task 1: Add `GET /api/stats` endpoint

**Files:**
- Create: `src/server/handlers/stats.rs`
- Modify: `src/server/handlers/mod.rs` (add module + re-export)
- Modify: `src/server/mod.rs:341-383` (add route)

**Step 1: Create the stats handler**

Create `src/server/handlers/stats.rs`:

```rust
use std::sync::Arc;

use axum::{extract::State, Json};
use http::StatusCode;
use serde::Serialize;

use crate::server::AppState;

#[derive(Serialize)]
pub struct StatsResponse {
    pub recipe_count: usize,
    pub menu_count: usize,
}

pub async fn stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let tree = cooklang_find::build_tree(&state.base_path)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    let mut recipe_count = 0;
    let mut menu_count = 0;

    count_entries(&tree, &mut recipe_count, &mut menu_count);

    Ok(Json(StatsResponse {
        recipe_count,
        menu_count,
    }))
}

fn count_entries(tree: &cooklang_find::RecipeTree, recipes: &mut usize, menus: &mut usize) {
    if let Some(ref entry) = tree.recipe {
        if entry.is_menu() {
            *menus += 1;
        } else {
            *recipes += 1;
        }
    }
    for child in tree.children.values() {
        count_entries(child, recipes, menus);
    }
}
```

**Step 2: Register the handler in mod.rs**

In `src/server/handlers/mod.rs`, add:
```rust
pub mod stats;
pub use stats::stats;
```

**Step 3: Add the route**

In `src/server/mod.rs`, inside the `api()` function, add:
```rust
.route("/stats", get(handlers::stats))
```

**Step 4: Build and test manually**

Run: `cargo build`
Expected: Compiles without errors.

Then test: `cargo run -- server ./seed` and in another terminal: `curl http://localhost:9080/api/stats`
Expected: `{"recipe_count":N,"menu_count":N}` with correct counts for seed directory.

**Step 5: Commit**

```bash
git add src/server/handlers/stats.rs src/server/handlers/mod.rs src/server/mod.rs
git commit -m "feat: add GET /api/stats endpoint for HA integration"
```

---

### Task 2: Add `GET /api/menus` endpoint

**Files:**
- Modify: `src/server/handlers/stats.rs` (add menus handler here, rename file to `stats.rs` keeps it simple; or create new file)
- Modify: `src/server/handlers/mod.rs`
- Modify: `src/server/mod.rs:341-383`

**Step 1: Create menus handler**

Create `src/server/handlers/menus.rs`:

```rust
use std::sync::Arc;

use axum::{extract::State, Json};
use http::StatusCode;
use serde::Serialize;

use crate::server::AppState;

#[derive(Serialize)]
pub struct MenuListItem {
    pub name: String,
    pub path: String,
}

pub async fn list_menus(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<MenuListItem>>, (StatusCode, Json<serde_json::Value>)> {
    let tree = cooklang_find::build_tree(&state.base_path)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    let mut menus = Vec::new();
    collect_menus(&tree, &mut menus, "");

    Ok(Json(menus))
}

fn collect_menus(tree: &cooklang_find::RecipeTree, menus: &mut Vec<MenuListItem>, prefix: &str) {
    if let Some(ref entry) = tree.recipe {
        if entry.is_menu() {
            let name = entry.name().clone().unwrap_or_default();
            let path = if prefix.is_empty() {
                entry.path().file_name().unwrap_or("").to_string()
            } else {
                format!("{}/{}", prefix, entry.path().file_name().unwrap_or(""))
            };
            menus.push(MenuListItem { name, path });
        }
    }
    for (dir_name, child) in &tree.children {
        let child_prefix = if prefix.is_empty() {
            dir_name.clone()
        } else {
            format!("{}/{}", prefix, dir_name)
        };
        collect_menus(child, menus, &child_prefix);
    }
}
```

**Step 2: Register in mod.rs and add route**

In `src/server/handlers/mod.rs`:
```rust
pub mod menus;
pub use menus::list_menus;
```

In `src/server/mod.rs` api() function:
```rust
.route("/menus", get(handlers::list_menus))
```

**Step 3: Build and test**

Run: `cargo build && cargo run -- server ./seed`
Test: `curl http://localhost:9080/api/menus`
Expected: JSON array containing `"2 Day Plan"` menu from seed directory.

**Step 4: Commit**

```bash
git add src/server/handlers/menus.rs src/server/handlers/mod.rs src/server/mod.rs
git commit -m "feat: add GET /api/menus endpoint listing menu files"
```

---

### Task 3: Add `GET /api/menus/*path` endpoint (parsed menu)

This is the most complex API endpoint. It extracts and restructures the parsing logic from `menu_page_handler()` in `src/server/ui.rs:1041-1271`.

**Files:**
- Modify: `src/server/handlers/menus.rs` (add get_menu handler + parsing structs)
- Modify: `src/server/mod.rs`

**Step 1: Add response structs and parsing logic to menus.rs**

Add these structs and the handler to `src/server/handlers/menus.rs`:

```rust
use axum::extract::Path;
use regex::Regex;

#[derive(Serialize)]
pub struct MenuResponse {
    pub name: String,
    pub path: String,
    pub metadata: serde_json::Value,
    pub sections: Vec<MenuApiSection>,
}

#[derive(Serialize)]
pub struct MenuApiSection {
    pub name: Option<String>,
    pub date: Option<String>,
    pub meals: Vec<MenuMeal>,
}

#[derive(Serialize)]
pub struct MenuMeal {
    #[serde(rename = "type")]
    pub meal_type: String,
    pub time: Option<String>,
    pub items: Vec<MenuMealItem>,
}

#[derive(Serialize)]
#[serde(tag = "kind")]
pub enum MenuMealItem {
    #[serde(rename = "recipe_reference")]
    RecipeReference {
        name: String,
        path: Option<String>,
        scale: Option<f64>,
    },
    #[serde(rename = "ingredient")]
    Ingredient {
        name: String,
        quantity: Option<String>,
        unit: Option<String>,
    },
}

pub async fn get_menu(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<MenuResponse>, (StatusCode, Json<serde_json::Value>)> {
    // 1. Resolve the menu file path (reuse logic from recipes handler)
    // 2. Parse the .menu file using the cooklang parser
    // 3. Extract sections, applying date regex to section names
    // 4. Within each section, group items by meal type headers
    // 5. Return structured JSON

    // Regex for extracting date from section name: == Day 1 (2026-03-04) ==
    let date_re = Regex::new(r"\((\d{4}-\d{2}-\d{2})\)").unwrap();
    // Regex for extracting time from meal header: Breakfast (08:30):
    let time_re = Regex::new(r"\((\d{2}:\d{2})\)").unwrap();

    // Parse the menu file using the same approach as menu_page_handler in ui.rs
    // ... (reuse the recipe entry resolution and parsing logic)
}
```

The handler needs to:
1. Find the menu file via `cooklang_find` (same pattern as `src/server/handlers/recipes.rs:149-159`)
2. Parse with cooklang parser (same as `ui.rs:1055-1065`)
3. Walk sections and items, building `MenuApiSection` structs
4. Extract dates from section names via regex `\((\d{4}-\d{2}-\d{2})\)`
5. Group items within each section by meal type headers (lines matching `SomeText:` pattern)
6. Extract times from meal headers via regex `\((\d{2}:\d{2})\)`

Reference `src/server/ui.rs:1073-1200` for the existing menu parsing logic to adapt.

**Step 2: Add route**

In `src/server/mod.rs` api() function (must be before the `/recipes/*path` route):
```rust
.route("/menus/*path", get(handlers::get_menu))
```

Register in `src/server/handlers/mod.rs`:
```rust
pub use menus::{list_menus, get_menu};
```

**Step 3: Add regex dependency if not already present**

Check `Cargo.toml` for `regex` crate. If missing, add it.

**Step 4: Build and test**

Run: `cargo build && cargo run -- server ./seed`
Test: `curl 'http://localhost:9080/api/menus/2%20Day%20Plan.menu'`
Expected: JSON with sections, each containing meals with recipe references and ingredients. Sections that include a date in parentheses should have the `date` field populated.

**Step 5: Commit**

```bash
git add src/server/handlers/menus.rs src/server/mod.rs src/server/handlers/mod.rs
git commit -m "feat: add GET /api/menus/*path endpoint with parsed menu structure"
```

---

### Task 4: Add `GET /api/pantry/expiring` endpoint

**Files:**
- Modify: `src/server/handlers/pantry.rs`
- Modify: `src/server/handlers/mod.rs`
- Modify: `src/server/mod.rs`

**Step 1: Add expiring handler**

Add to `src/server/handlers/pantry.rs`:

```rust
use chrono::{Local, NaiveDate};

#[derive(Debug, Deserialize)]
pub struct ExpiringQuery {
    pub days: Option<i64>,
}

#[derive(Serialize)]
pub struct ExpiringItemResponse {
    pub section: String,
    pub name: String,
    pub expire: String,
    pub days_remaining: i64,
}

pub async fn get_expiring(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ExpiringQuery>,
) -> Result<Json<Vec<ExpiringItemResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let days = query.days.unwrap_or(7);

    let pantry_path = state.pantry_path.as_ref().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No pantry configuration found"})),
        )
    })?;

    let content = std::fs::read_to_string(pantry_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    let result = cooklang::pantry::parse_lenient(&content);
    let pantry_conf = result.output().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to parse pantry"})),
        )
    })?;

    let today = Local::now().date_naive();
    let threshold = today + chrono::Duration::days(days);

    let mut items = Vec::new();
    for (section, section_items) in &pantry_conf.sections {
        for item in section_items {
            if let Some(expire_str) = item.expire() {
                if let Ok(date) = NaiveDate::parse_from_str(expire_str, "%Y-%m-%d") {
                    if date <= threshold {
                        items.push(ExpiringItemResponse {
                            section: section.clone(),
                            name: item.name().to_string(),
                            expire: expire_str.to_string(),
                            days_remaining: (date - today).num_days(),
                        });
                    }
                }
            }
        }
    }

    Ok(Json(items))
}
```

Reference `src/pantry.rs:286-399` for date parsing patterns -- may need to support multiple date formats beyond `%Y-%m-%d`.

**Step 2: Register and add route**

In `src/server/handlers/mod.rs`:
```rust
pub use pantry::{..., get_expiring};
```

In `src/server/mod.rs` api() function:
```rust
.route("/pantry/expiring", get(handlers::get_expiring))
```

Note: This route must be registered BEFORE `.route("/pantry/:section/:name", ...)` to avoid conflicts.

**Step 3: Build and test**

Run: `cargo build && cargo run -- server ./seed`
Test: `curl 'http://localhost:9080/api/pantry/expiring?days=30'`
Expected: JSON array of expiring items (may be empty if seed pantry has no expiration dates).

**Step 4: Commit**

```bash
git add src/server/handlers/pantry.rs src/server/handlers/mod.rs src/server/mod.rs
git commit -m "feat: add GET /api/pantry/expiring endpoint"
```

---

### Task 5: Add `GET /api/pantry/depleted` endpoint

**Files:**
- Modify: `src/server/handlers/pantry.rs`
- Modify: `src/server/handlers/mod.rs`
- Modify: `src/server/mod.rs`

**Step 1: Add depleted handler**

Add to `src/server/handlers/pantry.rs`:

```rust
#[derive(Serialize)]
pub struct DepletedItemResponse {
    pub section: String,
    pub name: String,
    pub low: Option<String>,
}

pub async fn get_depleted(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<DepletedItemResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let pantry_path = state.pantry_path.as_ref().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No pantry configuration found"})),
        )
    })?;

    let content = std::fs::read_to_string(pantry_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    let result = cooklang::pantry::parse_lenient(&content);
    let pantry_conf = result.output().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to parse pantry"})),
        )
    })?;

    let mut items = Vec::new();
    for (section, section_items) in &pantry_conf.sections {
        for item in section_items {
            if item.is_low() {
                items.push(DepletedItemResponse {
                    section: section.clone(),
                    name: item.name().to_string(),
                    low: item.low().map(|s| s.to_string()),
                });
            }
        }
    }

    Ok(Json(items))
}
```

Reference `src/pantry.rs:176-284` for the `is_low()` method and low-stock detection logic.

**Step 2: Register and add route**

In `src/server/handlers/mod.rs`:
```rust
pub use pantry::{..., get_depleted};
```

In `src/server/mod.rs`:
```rust
.route("/pantry/depleted", get(handlers::get_depleted))
```

Place before the `/pantry/:section/:name` routes.

**Step 3: Build and test**

Run: `cargo build && cargo run -- server ./seed`
Test: `curl http://localhost:9080/api/pantry/depleted`

**Step 4: Commit**

```bash
git add src/server/handlers/pantry.rs src/server/handlers/mod.rs src/server/mod.rs
git commit -m "feat: add GET /api/pantry/depleted endpoint"
```

---

### Task 6: Run `cargo fmt` and `cargo clippy`

**Step 1: Format and lint**

Run: `cargo fmt && cargo clippy`
Fix any warnings.

**Step 2: Run tests**

Run: `cargo test`
All tests should pass.

**Step 3: Commit if there were fixes**

```bash
git add -A
git commit -m "style: apply cargo fmt and clippy fixes"
```

---

## Part 2: HACS Custom Component (Python)

All files in this part are created in `/Users/alexeydubovskoy/Cooklang/homeassistant-cookcli/`.

### Task 7: Scaffold the HACS component

**Files:**
- Create: `custom_components/cookcli/__init__.py`
- Create: `custom_components/cookcli/manifest.json`
- Create: `custom_components/cookcli/const.py`
- Create: `custom_components/cookcli/strings.json`
- Create: `custom_components/cookcli/translations/en.json`
- Create: `hacs.json`
- Create: `README.md`

**Step 1: Create directory structure**

```bash
mkdir -p ../homeassistant-cookcli/custom_components/cookcli/translations
```

**Step 2: Create manifest.json**

Create `custom_components/cookcli/manifest.json`:
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

**Step 3: Create const.py**

Create `custom_components/cookcli/const.py`:
```python
DOMAIN = "cookcli"
DEFAULT_URL = "http://localhost:9080"
DEFAULT_SCAN_INTERVAL = 300  # 5 minutes

CONF_ACTIVE_MENU = "active_menu"

DEFAULT_MEAL_TIMES = {
    "breakfast": "07:00",
    "lunch": "12:00",
    "dinner": "18:00",
    "snack": "15:00",
}
```

**Step 4: Create __init__.py**

Create `custom_components/cookcli/__init__.py`:
```python
"""CookCLI integration for Home Assistant."""
from __future__ import annotations

from homeassistant.config_entries import ConfigEntry
from homeassistant.const import CONF_URL, Platform
from homeassistant.core import HomeAssistant

from .const import DOMAIN
from .coordinator import CookCLICoordinator

PLATFORMS = [Platform.CALENDAR, Platform.TODO, Platform.SENSOR]


async def async_setup_entry(hass: HomeAssistant, entry: ConfigEntry) -> bool:
    """Set up CookCLI from a config entry."""
    coordinator = CookCLICoordinator(hass, entry)
    await coordinator.async_config_entry_first_refresh()

    hass.data.setdefault(DOMAIN, {})[entry.entry_id] = coordinator

    await hass.config_entries.async_forward_entry_setups(entry, PLATFORMS)

    return True


async def async_unload_entry(hass: HomeAssistant, entry: ConfigEntry) -> bool:
    """Unload a config entry."""
    unload_ok = await hass.config_entries.async_unload_platforms(entry, PLATFORMS)
    if unload_ok:
        hass.data[DOMAIN].pop(entry.entry_id)
    return unload_ok
```

**Step 5: Create strings.json and translations/en.json**

Create `custom_components/cookcli/strings.json`:
```json
{
  "config": {
    "step": {
      "user": {
        "title": "Connect to CookCLI",
        "data": {
          "url": "Server URL",
          "active_menu": "Active Menu Plan (optional)"
        }
      }
    },
    "error": {
      "cannot_connect": "Unable to connect to CookCLI server",
      "unknown": "Unexpected error occurred"
    }
  }
}
```

Copy the same content to `custom_components/cookcli/translations/en.json`.

**Step 6: Create hacs.json**

Create `hacs.json` at repo root:
```json
{
  "name": "CookCLI",
  "render_readme": true
}
```

**Step 7: Commit**

```bash
cd ../homeassistant-cookcli
git add -A
git commit -m "feat: scaffold HACS custom component structure"
```

---

### Task 8: Create the API client

**Files:**
- Create: `custom_components/cookcli/api.py`

**Step 1: Create api.py**

Create `custom_components/cookcli/api.py`:
```python
"""CookCLI REST API client."""
from __future__ import annotations

import aiohttp
import async_timeout


class CookCLIApiError(Exception):
    """Base exception for CookCLI API errors."""


class CookCLIConnectionError(CookCLIApiError):
    """Connection error."""


class CookCLIApi:
    """Client for CookCLI REST API."""

    def __init__(self, url: str, session: aiohttp.ClientSession) -> None:
        self._url = url.rstrip("/")
        self._session = session

    async def _get(self, path: str, params: dict | None = None) -> dict | list:
        """Make a GET request."""
        try:
            async with async_timeout.timeout(10):
                resp = await self._session.get(
                    f"{self._url}{path}", params=params
                )
                resp.raise_for_status()
                return await resp.json()
        except aiohttp.ClientError as err:
            raise CookCLIConnectionError(
                f"Error communicating with CookCLI: {err}"
            ) from err

    async def _post(self, path: str, data: dict | None = None) -> None:
        """Make a POST request."""
        try:
            async with async_timeout.timeout(10):
                resp = await self._session.post(
                    f"{self._url}{path}", json=data
                )
                resp.raise_for_status()
        except aiohttp.ClientError as err:
            raise CookCLIConnectionError(
                f"Error communicating with CookCLI: {err}"
            ) from err

    async def async_get_stats(self) -> dict:
        return await self._get("/api/stats")

    async def async_get_menus(self) -> list[dict]:
        return await self._get("/api/menus")

    async def async_get_menu(self, path: str) -> dict:
        return await self._get(f"/api/menus/{path}")

    async def async_get_shopping_list_items(self) -> list[dict]:
        return await self._get("/api/shopping_list/items")

    async def async_add_to_shopping_list(
        self, path: str, name: str, scale: float = 1.0
    ) -> None:
        await self._post(
            "/api/shopping_list/add",
            {"path": path, "name": name, "scale": scale},
        )

    async def async_remove_from_shopping_list(self, path: str) -> None:
        await self._post("/api/shopping_list/remove", {"path": path})

    async def async_clear_shopping_list(self) -> None:
        await self._post("/api/shopping_list/clear")

    async def async_get_pantry_expiring(self, days: int = 7) -> list[dict]:
        return await self._get("/api/pantry/expiring", params={"days": days})

    async def async_get_pantry_depleted(self) -> list[dict]:
        return await self._get("/api/pantry/depleted")

    async def async_search(self, query: str) -> list[dict]:
        return await self._get("/api/search", params={"q": query})
```

**Step 2: Commit**

```bash
git add custom_components/cookcli/api.py
git commit -m "feat: add CookCLI REST API client"
```

---

### Task 9: Create the DataUpdateCoordinator

**Files:**
- Create: `custom_components/cookcli/coordinator.py`

**Step 1: Create coordinator.py**

Create `custom_components/cookcli/coordinator.py`:
```python
"""DataUpdateCoordinator for CookCLI."""
from __future__ import annotations

import logging
from datetime import timedelta
from typing import Any

from homeassistant.config_entries import ConfigEntry
from homeassistant.const import CONF_URL
from homeassistant.core import HomeAssistant
from homeassistant.helpers.aiohttp_client import async_get_clientsession
from homeassistant.helpers.update_coordinator import DataUpdateCoordinator, UpdateFailed

from .api import CookCLIApi, CookCLIApiError
from .const import CONF_ACTIVE_MENU, DEFAULT_SCAN_INTERVAL, DOMAIN

_LOGGER = logging.getLogger(__name__)


class CookCLICoordinator(DataUpdateCoordinator[dict[str, Any]]):
    """Coordinator for polling CookCLI API."""

    def __init__(self, hass: HomeAssistant, entry: ConfigEntry) -> None:
        super().__init__(
            hass,
            _LOGGER,
            name=DOMAIN,
            update_interval=timedelta(seconds=DEFAULT_SCAN_INTERVAL),
        )
        session = async_get_clientsession(hass)
        self.api = CookCLIApi(entry.data[CONF_URL], session)
        self.active_menu: str | None = entry.data.get(CONF_ACTIVE_MENU)

    async def _async_update_data(self) -> dict[str, Any]:
        """Fetch data from CookCLI."""
        try:
            stats = await self.api.async_get_stats()
            shopping_items = await self.api.async_get_shopping_list_items()
            pantry_expiring = await self.api.async_get_pantry_expiring(days=7)
            pantry_depleted = await self.api.async_get_pantry_depleted()

            menu = None
            if self.active_menu:
                menu = await self.api.async_get_menu(self.active_menu)

            return {
                "stats": stats,
                "shopping_items": shopping_items,
                "pantry_expiring": pantry_expiring,
                "pantry_depleted": pantry_depleted,
                "menu": menu,
            }
        except CookCLIApiError as err:
            raise UpdateFailed(f"Error fetching CookCLI data: {err}") from err
```

**Step 2: Commit**

```bash
git add custom_components/cookcli/coordinator.py
git commit -m "feat: add DataUpdateCoordinator for CookCLI polling"
```

---

### Task 10: Create the config flow

**Files:**
- Create: `custom_components/cookcli/config_flow.py`

**Step 1: Create config_flow.py**

Create `custom_components/cookcli/config_flow.py`:
```python
"""Config flow for CookCLI integration."""
from __future__ import annotations

from typing import Any

import aiohttp
import voluptuous as vol

from homeassistant.config_entries import ConfigFlow, ConfigFlowResult
from homeassistant.const import CONF_URL
from homeassistant.helpers.aiohttp_client import async_get_clientsession

from .api import CookCLIApi, CookCLIConnectionError
from .const import CONF_ACTIVE_MENU, DEFAULT_URL, DOMAIN


class CookCLIConfigFlow(ConfigFlow, domain=DOMAIN):
    """Handle a config flow for CookCLI."""

    VERSION = 1

    async def async_step_user(
        self, user_input: dict[str, Any] | None = None
    ) -> ConfigFlowResult:
        """Handle the initial step."""
        errors: dict[str, str] = {}

        if user_input is not None:
            url = user_input[CONF_URL]
            session = async_get_clientsession(self.hass)
            api = CookCLIApi(url, session)

            try:
                await api.async_get_stats()
            except CookCLIConnectionError:
                errors["base"] = "cannot_connect"
            except Exception:  # noqa: BLE001
                errors["base"] = "unknown"
            else:
                await self.async_set_unique_id(url)
                self._abort_if_unique_id_configured()
                return self.async_create_entry(
                    title="CookCLI",
                    data=user_input,
                )

        return self.async_show_form(
            step_id="user",
            data_schema=vol.Schema(
                {
                    vol.Required(CONF_URL, default=DEFAULT_URL): str,
                    vol.Optional(CONF_ACTIVE_MENU): str,
                }
            ),
            errors=errors,
        )
```

**Step 2: Commit**

```bash
git add custom_components/cookcli/config_flow.py
git commit -m "feat: add config flow for CookCLI setup"
```

---

### Task 11: Create sensor entities

**Files:**
- Create: `custom_components/cookcli/sensor.py`

**Step 1: Create sensor.py**

Create `custom_components/cookcli/sensor.py`:
```python
"""Sensor entities for CookCLI."""
from __future__ import annotations

from homeassistant.components.sensor import SensorEntity
from homeassistant.config_entries import ConfigEntry
from homeassistant.core import HomeAssistant
from homeassistant.helpers.entity_platform import AddEntitiesCallback
from homeassistant.helpers.update_coordinator import CoordinatorEntity

from .const import DOMAIN
from .coordinator import CookCLICoordinator


async def async_setup_entry(
    hass: HomeAssistant,
    entry: ConfigEntry,
    async_add_entities: AddEntitiesCallback,
) -> None:
    """Set up CookCLI sensors."""
    coordinator: CookCLICoordinator = hass.data[DOMAIN][entry.entry_id]

    entities = [
        CookCLIRecipeCountSensor(coordinator, entry),
        CookCLIPantryExpiringSensor(coordinator, entry),
        CookCLIPantryDepletedSensor(coordinator, entry),
    ]

    async_add_entities(entities)


class CookCLIRecipeCountSensor(CoordinatorEntity, SensorEntity):
    """Sensor showing total recipe count."""

    _attr_icon = "mdi:book-open-variant"

    def __init__(self, coordinator: CookCLICoordinator, entry: ConfigEntry) -> None:
        super().__init__(coordinator)
        self._attr_unique_id = f"{entry.entry_id}_recipe_count"
        self._attr_name = "CookCLI Recipes"

    @property
    def native_value(self) -> int | None:
        stats = self.coordinator.data.get("stats")
        if stats:
            return stats.get("recipe_count")
        return None

    @property
    def extra_state_attributes(self) -> dict:
        stats = self.coordinator.data.get("stats", {})
        return {"menu_count": stats.get("menu_count", 0)}


class CookCLIPantryExpiringSensor(CoordinatorEntity, SensorEntity):
    """Sensor showing number of expiring pantry items."""

    _attr_icon = "mdi:clock-alert-outline"

    def __init__(self, coordinator: CookCLICoordinator, entry: ConfigEntry) -> None:
        super().__init__(coordinator)
        self._attr_unique_id = f"{entry.entry_id}_pantry_expiring"
        self._attr_name = "CookCLI Pantry Expiring"

    @property
    def native_value(self) -> int:
        items = self.coordinator.data.get("pantry_expiring", [])
        return len(items)

    @property
    def extra_state_attributes(self) -> dict:
        return {"items": self.coordinator.data.get("pantry_expiring", [])}


class CookCLIPantryDepletedSensor(CoordinatorEntity, SensorEntity):
    """Sensor showing number of depleted pantry items."""

    _attr_icon = "mdi:basket-off-outline"

    def __init__(self, coordinator: CookCLICoordinator, entry: ConfigEntry) -> None:
        super().__init__(coordinator)
        self._attr_unique_id = f"{entry.entry_id}_pantry_depleted"
        self._attr_name = "CookCLI Pantry Depleted"

    @property
    def native_value(self) -> int:
        items = self.coordinator.data.get("pantry_depleted", [])
        return len(items)

    @property
    def extra_state_attributes(self) -> dict:
        return {"items": self.coordinator.data.get("pantry_depleted", [])}
```

**Step 2: Commit**

```bash
git add custom_components/cookcli/sensor.py
git commit -m "feat: add recipe count and pantry sensor entities"
```

---

### Task 12: Create Todo entity (shopping list)

**Files:**
- Create: `custom_components/cookcli/todo.py`

**Step 1: Create todo.py**

Create `custom_components/cookcli/todo.py`:
```python
"""Todo entity for CookCLI shopping list."""
from __future__ import annotations

from homeassistant.components.todo import (
    TodoItem,
    TodoItemStatus,
    TodoListEntity,
    TodoListEntityFeature,
)
from homeassistant.config_entries import ConfigEntry
from homeassistant.core import HomeAssistant
from homeassistant.helpers.entity_platform import AddEntitiesCallback
from homeassistant.helpers.update_coordinator import CoordinatorEntity

from .const import DOMAIN
from .coordinator import CookCLICoordinator


async def async_setup_entry(
    hass: HomeAssistant,
    entry: ConfigEntry,
    async_add_entities: AddEntitiesCallback,
) -> None:
    """Set up CookCLI todo entities."""
    coordinator: CookCLICoordinator = hass.data[DOMAIN][entry.entry_id]
    async_add_entities([CookCLIShoppingListTodo(coordinator, entry)])


class CookCLIShoppingListTodo(CoordinatorEntity, TodoListEntity):
    """Shopping list as a Todo entity."""

    _attr_supported_features = (
        TodoListEntityFeature.DELETE_TODO_ITEM
    )
    _attr_icon = "mdi:cart-outline"

    def __init__(self, coordinator: CookCLICoordinator, entry: ConfigEntry) -> None:
        super().__init__(coordinator)
        self._attr_unique_id = f"{entry.entry_id}_shopping_list"
        self._attr_name = "CookCLI Shopping List"

    @property
    def todo_items(self) -> list[TodoItem]:
        items = self.coordinator.data.get("shopping_items", [])
        return [
            TodoItem(
                uid=item["path"],
                summary=f"{item['name']} (x{item['scale']})" if item.get("scale", 1) != 1 else item["name"],
                status=TodoItemStatus.NEEDS_ACTION,
            )
            for item in items
        ]

    async def async_delete_todo_items(self, uids: list[str]) -> None:
        """Remove items from shopping list."""
        for uid in uids:
            await self.coordinator.api.async_remove_from_shopping_list(uid)
        await self.coordinator.async_request_refresh()
```

**Step 2: Commit**

```bash
git add custom_components/cookcli/todo.py
git commit -m "feat: add shopping list Todo entity"
```

---

### Task 13: Create Calendar entity (menu meal plan)

**Files:**
- Create: `custom_components/cookcli/calendar.py`

**Step 1: Create calendar.py**

Create `custom_components/cookcli/calendar.py`:
```python
"""Calendar entity for CookCLI menu meal plans."""
from __future__ import annotations

import re
from datetime import datetime, timedelta
from typing import Any

from homeassistant.components.calendar import CalendarEntity, CalendarEvent
from homeassistant.config_entries import ConfigEntry
from homeassistant.core import HomeAssistant
from homeassistant.helpers.entity_platform import AddEntitiesCallback
from homeassistant.helpers.update_coordinator import CoordinatorEntity

from .const import DEFAULT_MEAL_TIMES, DOMAIN
from .coordinator import CookCLICoordinator

DATE_RE = re.compile(r"\((\d{4}-\d{2}-\d{2})\)")
TIME_RE = re.compile(r"\((\d{2}:\d{2})\)")


async def async_setup_entry(
    hass: HomeAssistant,
    entry: ConfigEntry,
    async_add_entities: AddEntitiesCallback,
) -> None:
    """Set up CookCLI calendar entities."""
    coordinator: CookCLICoordinator = hass.data[DOMAIN][entry.entry_id]

    if coordinator.active_menu:
        async_add_entities([CookCLIMealPlanCalendar(coordinator, entry)])


class CookCLIMealPlanCalendar(CoordinatorEntity, CalendarEntity):
    """Meal plan calendar from a .menu file."""

    _attr_icon = "mdi:silverware-fork-knife"

    def __init__(self, coordinator: CookCLICoordinator, entry: ConfigEntry) -> None:
        super().__init__(coordinator)
        self._attr_unique_id = f"{entry.entry_id}_meal_plan"
        self._attr_name = "CookCLI Meal Plan"
        self._events: list[CalendarEvent] = []

    @property
    def event(self) -> CalendarEvent | None:
        """Return the next upcoming event."""
        now = datetime.now()
        events = self._parse_events()
        future = [e for e in events if e.end > now]
        return future[0] if future else None

    async def async_get_events(
        self,
        hass: HomeAssistant,
        start_date: datetime,
        end_date: datetime,
    ) -> list[CalendarEvent]:
        """Return events in a date range."""
        events = self._parse_events()
        return [
            e for e in events
            if e.start < end_date and e.end > start_date
        ]

    def _parse_events(self) -> list[CalendarEvent]:
        """Parse menu data into calendar events."""
        menu = self.coordinator.data.get("menu")
        if not menu:
            return []

        events = []
        for section in menu.get("sections", []):
            section_name = section.get("name") or ""
            date_match = DATE_RE.search(section_name)
            if not date_match:
                continue

            date_str = date_match.group(1)

            for meal in section.get("meals", []):
                meal_type = meal.get("type", "Meal")
                time_str = meal.get("time")

                if not time_str:
                    # Fall back to defaults
                    time_str = DEFAULT_MEAL_TIMES.get(
                        meal_type.lower(), None
                    )

                # Build item descriptions
                descriptions = []
                for item in meal.get("items", []):
                    if item.get("kind") == "recipe_reference":
                        descriptions.append(item["name"])
                    elif item.get("kind") == "ingredient":
                        descriptions.append(item["name"])
                description = ", ".join(descriptions)

                if time_str:
                    start = datetime.fromisoformat(f"{date_str}T{time_str}")
                    end = start + timedelta(hours=1)
                else:
                    # All-day event
                    start = datetime.fromisoformat(f"{date_str}T00:00")
                    end = start + timedelta(days=1)

                events.append(CalendarEvent(
                    summary=meal_type,
                    description=description,
                    start=start,
                    end=end,
                ))

        return sorted(events, key=lambda e: e.start)
```

**Step 2: Commit**

```bash
git add custom_components/cookcli/calendar.py
git commit -m "feat: add meal plan Calendar entity from menu files"
```

---

### Task 14: Add services

**Files:**
- Create: `custom_components/cookcli/services.yaml`
- Modify: `custom_components/cookcli/__init__.py`

**Step 1: Create services.yaml**

Create `custom_components/cookcli/services.yaml`:
```yaml
search_recipe:
  name: Search recipes
  description: Search CookCLI recipes by keyword
  fields:
    query:
      name: Query
      description: Search term
      required: true
      selector:
        text:

add_recipe_to_shopping_list:
  name: Add recipe to shopping list
  description: Add a recipe's ingredients to the shopping list
  fields:
    recipe:
      name: Recipe
      description: Recipe file path
      required: true
      selector:
        text:
    scale:
      name: Scale
      description: Scale factor
      required: false
      default: 1
      selector:
        number:
          min: 0.5
          max: 10
          step: 0.5
          mode: box

clear_shopping_list:
  name: Clear shopping list
  description: Remove all items from the shopping list
```

**Step 2: Register services in __init__.py**

Update `custom_components/cookcli/__init__.py` to add service registration after the coordinator setup. Add these service handlers:

```python
from homeassistant.core import ServiceCall, ServiceResponse, SupportsResponse

async def async_setup_entry(hass: HomeAssistant, entry: ConfigEntry) -> bool:
    """Set up CookCLI from a config entry."""
    coordinator = CookCLICoordinator(hass, entry)
    await coordinator.async_config_entry_first_refresh()
    hass.data.setdefault(DOMAIN, {})[entry.entry_id] = coordinator
    await hass.config_entries.async_forward_entry_setups(entry, PLATFORMS)

    async def handle_search(call: ServiceCall) -> ServiceResponse:
        query = call.data["query"]
        results = await coordinator.api.async_search(query)
        return {"results": results}

    async def handle_add_recipe(call: ServiceCall) -> None:
        recipe = call.data["recipe"]
        scale = call.data.get("scale", 1)
        await coordinator.api.async_add_to_shopping_list(recipe, recipe, scale)
        await coordinator.async_request_refresh()

    async def handle_clear(call: ServiceCall) -> None:
        await coordinator.api.async_clear_shopping_list()
        await coordinator.async_request_refresh()

    hass.services.async_register(DOMAIN, "search_recipe", handle_search)
    hass.services.async_register(DOMAIN, "add_recipe_to_shopping_list", handle_add_recipe)
    hass.services.async_register(DOMAIN, "clear_shopping_list", handle_clear)

    return True
```

**Step 3: Commit**

```bash
git add custom_components/cookcli/services.yaml custom_components/cookcli/__init__.py
git commit -m "feat: add search, add-to-shopping-list, and clear services"
```

---

### Task 15: Final checks and documentation

**Step 1: Verify CookCLI builds clean**

In the CookCLI directory:
```bash
cargo fmt
cargo clippy
cargo test
```

**Step 2: Verify HA component file structure**

In `../homeassistant-cookcli/`:
```bash
find custom_components -type f | sort
```

Expected:
```
custom_components/cookcli/__init__.py
custom_components/cookcli/api.py
custom_components/cookcli/calendar.py
custom_components/cookcli/config_flow.py
custom_components/cookcli/const.py
custom_components/cookcli/coordinator.py
custom_components/cookcli/manifest.json
custom_components/cookcli/sensor.py
custom_components/cookcli/services.yaml
custom_components/cookcli/strings.json
custom_components/cookcli/todo.py
custom_components/cookcli/translations/en.json
```

**Step 3: Commit any final fixes**

```bash
git add -A
git commit -m "chore: finalize HACS component structure"
```
