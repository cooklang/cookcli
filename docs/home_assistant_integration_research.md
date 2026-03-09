# Home Assistant Integration Research: 2025 Market Analysis

**Research Date:** March 2026
**Focus:** Popular integrations, patterns, food/recipe systems, and what drives adoption

---

## Executive Summary

Home Assistant has reached **2 million active installations** (2025.5 release), with nearly **3,000 official integrations** available. The ecosystem includes both built-in integrations and community-developed custom components through HACS (Home Assistant Community Store). Success is driven by solving specific pain points, reliability, seamless updates, and dashboard visibility.

**Key Insight for CookCLI:** Food/recipe integrations like Mealie are gaining traction (2,079 active installations as of 2024.7), but no comprehensive cooking/recipe CLI integration exists yet.

---

## 1. Top Popular Home Assistant Integrations (2024-2025)

### Built-in Integrations (No Additional Setup Required)

These are bundled with Home Assistant Core:

| Integration | Purpose | IoT Class | Notes |
|---|---|---|---|
| **Automation** | Core workflow engine | Local | Drives retention - automations are core to HA value |
| **Scripts & Scenes** | Task automation & presets | Local | Essential for advanced users |
| **Template** | Dynamic entity creation | Local | Highly flexible for power users |
| **RESTful** | REST API polling | Local Polling | Generic HTTP endpoint access |
| **MQTT** | Message broker protocol | Local Push | Used by 44% of active installations |
| **Calendar** | Event scheduling | - | Google Calendar, CalDAV support |
| **Todo** | To-do list management | - | Native Mealie & Bring! support |
| **Sensor** | Numeric measurements | - | Most common entity type |
| **Binary Sensor** | On/off states | - | Window/door sensors, motion, etc. |
| **Switch** | Controllable devices | - | Lights, plugs, power control |
| **Climate** | Thermostats & HVAC | - | Temperature management |
| **Media Player** | Audio/video devices | - | Speakers, TVs, receivers |
| **Shopping List** | Built-in list management | - | Simple, native alternative to Mealie/Grocy |

### Officially Recognized Popular HACS Integrations (2024-2025)

Community-developed custom integrations distributed via HACS:

| Integration | Category | Purpose | Quality Indicator |
|---|---|---|---|
| **Alarmo** | Security | DIY alarm system with automations | Highly Recommended |
| **Frigate** | Cameras | Security camera management & recording | Highly Recommended |
| **Local Tuya** | Device Control | Control Tuya Wi-Fi devices locally | Popular |
| **ZHA Toolkit** | Zigbee | Enhanced ZHA cluster operations | Power User |
| **Powercalc** | Energy | Estimate power consumption | Data Focused |
| **Battery Notes** | Device Management | Track battery info by device | Simple/Popular |
| **Adaptive Lighting** | Lighting | Circadian rhythm color/brightness | Retention Driver |
| **Bubble Card** | UI | Mobile-optimized cards | Visual/Engagement |
| **Mushroom Cards** | UI | Simple entity cards | Visual/Engagement |
| **Browser Mod 2.0** | UI/Automation | Enhanced browser control & automations | Powerful |
| **Spook** | Automation | New actions for automations/scripts | Enhancement |
| **ESPHome** | Device Framework | DIY smart device framework | Native Support |
| **Alexa Media Player** | Device Control | Control Amazon Alexa devices | Popular |
| **iCloud3** | Device Tracking | Enhanced iCloud device location | Specialized |
| **SmartIR** | IR Control | Broadlink IR device integration | Popular |
| **Circadian Lighting** | Lighting | Color temperature management | Legacy (now Adaptive Lighting) |
| **Lutron Caseta Pro** | Lighting | Smart lighting control | Professional |
| **Sonoff LAN** | Device Control | Local network device control | Popular |
| **Matterbridge** | Protocol Bridge | Bridge non-Matter devices to Matter | Growing |
| **InfluxDB** | Data Storage | Time-series database | Analytics Focused |

### Not Yet Officially Integrated

Notable platforms without native Home Assistant integrations (as of 2025):

- **Tandoor Recipes** - No official integration, but available as add-on with API workarounds
- **Grocy** - Available as HACS custom component, not official integration
- **Bring!** - Official integration exists, but limited to todo list syncing
- **CookCLI** - No existing Home Assistant integration (opportunity!)

---

## 2. Common Patterns in Successful HA Integrations

### Pattern 1: DataUpdateCoordinator Pattern

**What it is:** Central data fetching coordinator that all entities use

**Why it matters:**
- Single coordinated API call instead of per-entity calls
- Efficient polling schedules
- Reduces network overhead
- Prevents API rate limiting

**Structure:**
```
MyIntegration/
├── __init__.py                 # Setup & coordinator instantiation
├── config_flow.py              # Configuration UI
├── coordinator.py              # DataUpdateCoordinator class
├── const.py                    # Constants & defaults
├── entity.py                   # Entity base classes
└── sensor.py, binary_sensor.py # Entity implementations
```

**Key Methods:**
- `_async_update_data()` - Single place to fetch all data
- `CoordinatorEntity` - Base class for entities using coordinator
- `should_poll = False` - Coordinator handles updates, not entities

### Pattern 2: Entity Types (Provides Functionality)

**Most Common:**
- **Sensor** - Numeric values (temperature, price, count)
- **Binary Sensor** - On/off states (motion, window, recipe ready)
- **Switch** - Controllable boolean (toggle automation, enable mode)
- **Todo** - To-do list items (shopping lists, meal plans)
- **Calendar** - Scheduled events (meal planning, reminders)
- **Select** - Dropdown selection (recipe scale, meal selection)
- **Light** - Brightness/color control
- **Climate** - Temperature/humidity control

**Pattern:** Start with sensors, expand to automations via binary sensors

### Pattern 3: Dashboard Integration

**Critical for Retention:**
- Pre-built dashboard templates for the integration
- Auto-generated cards matching integration type
- Device Database integration for smart defaults
- Mobile-responsive design

**2025 Trend:** Integrations now provide "smart cards" - context-aware UI components that auto-suggest useful visualizations (e.g., "fridge dashboard" for appliance integrations)

### Pattern 4: Automation Enablement

**What Successful Integrations Provide:**
- Service calls for automation triggers
- Event signals for condition checks
- Binary sensors for automation conditions
- Device triggers for native automations

**Example (Mealie):**
- Meal plan calendar → triggers daily notification
- Shopping list todo → automation to show when near store
- Recipe sensor → automation to enable kitchen display

### Pattern 5: Local vs Cloud

**Local-First Approach (Strongly Preferred):**
- No cloud dependency
- Works offline
- Faster response
- Private data
- Higher quality rating

**Examples:**
- Local Tuya (local network control)
- Sonoff LAN (local polling)
- ESPHome (local firmware)
- MQTT (local broker)
- Grocy/Mealie (self-hosted)

**Cloud Support (Acceptable as fallback):**
- Only if local option unavailable
- Should auto-detect local availability
- Circuit breaker pattern for failures
- Exponential backoff retry logic

### Pattern 6: Configuration Flow (User Experience)

**What Matters:**
1. **One-click setup** - Zeroconf discovery or reauth
2. **Clear options** - Wizard format, not YAML only
3. **Validation** - Real-time config validation
4. **Reconfiguration** - Easy update without reinstall
5. **Options Flow** - Runtime settings without restart

**Quality Indicator:** Integrations with config_flow > manual YAML-only config

### Pattern 7: Update Management

**HACS Solves This:**
- Automatic update notifications
- One-click updates
- No manual file management
- Dependency handling

**Result:** Higher adoption for HACS-distributed integrations

### Pattern 8: Community & Reputation

**Stickiness Drivers:**
- Active developer maintaining code
- Regular updates and bug fixes
- Community forum presence
- GitHub discussions responsive
- Clear documentation
- Example automations provided

**Anti-patterns:**
- Abandoned repos (no updates for 6+ months)
- Unresponsive maintainers
- Breaking changes without migration path
- Poor error messages

---

## 3. Food/Kitchen/Recipe Integrations in Home Assistant

### Currently Available

#### Mealie Integration (Recommended)

**Status:** Official integration since 2024.7

**GitHub:** `mealie-recipes/mealie-hacs`

**Adoption:** 2,079 active installations (2024.7 baseline)

**What it does:**
- Syncs meal plans → Calendar entities (hourly updates)
- Syncs shopping lists → Todo list entities (5-minute updates)
- Read-only view in Home Assistant dashboards
- Integrates with store location-based automations

**Entities created:**
- `calendar.meal_*` - Meal plan calendars
- `todo.mealie_*` - Shopping list entities

**Integration pattern:**
- REST API calls to self-hosted Mealie server
- DataUpdateCoordinator for polling
- Calendar & Todo entity types

**Limitations:**
- No recipe browsing in HA
- No serving size/scaling in HA
- No ingredient list display
- Read-only (can't create recipes in HA)

**Automation examples:**
```yaml
# Send daily meal reminder
- trigger: time
  at: "08:00"
  action: notify.notify
  data:
    message: "Today's meal: {{ state_attr('calendar.meal_breakfast', 'event') }}"

# Notify when approaching grocery store with items
- trigger: state
  entity_id: zone.grocery_store
  to: "in"
  action: notify.notify
  data:
    message: "Items to buy: {{ states('todo.shopping_list') }}"
```

#### Bring! Integration (Basic)

**Status:** Official integration in Home Assistant core

**Purpose:** Shared grocery shopping list app

**What it does:**
- Sync Bring! lists → Todo list entities
- Available in HA dashboards
- Mark items as done in HA

**Adoption:** Less tracked than Mealie

**Limitations:**
- Third-party cloud service (no self-host)
- Basic todo functionality only
- No recipe integration

#### Grocy Integration (Advanced Users)

**Status:** Community HACS custom component

**GitHub:** `custom-components/grocy`

**Purpose:** Complete kitchen inventory & stock management

**What it does:**
- Track inventory quantities and expiration dates
- Shopping list integration
- Chore tracking
- Product database
- REST API for automation

**Adoption:** Unknown (HACS custom component)

**Why it's powerful:**
- Stock level monitoring → automations
- Expiration alerts → notifications
- Consumption tracking → recipes reorder ingredients
- Works with Mealie for recipe-to-shopping-list

**Example automation:**
```yaml
# Refill when stock low
- trigger: state
  entity_id: sensor.milk_quantity
  below: 1
  action:
    service: todo.add_item
    target:
      entity_id: todo.shopping_list
    data:
      item: "Milk"
```

#### Tandoor Recipes (Limited)

**Status:** Available as Home Assistant add-on, not full integration

**Purpose:** Recipe management and planning

**What it does:**
- Self-hosted recipe platform
- Meal planning
- Shared recipes & community

**HA Integration:**
- Can be accessed via browser mod
- API available for custom automations
- Users report custom Python scripts to pull meal data

**GitHub Issue:** No official HA integration exists (TandoorRecipes/recipes#796)

**Limitation:** No native HA entity support (unlike Mealie/Grocy)

### The Opportunity: CookCLI as HA Integration

**Current Gap:**
- Mealie handles meal plans & shopping lists
- Grocy handles inventory
- **No integration for a versatile recipe CLI tool**

**What CookCLI Integration Could Provide:**
1. **Recipe Discovery** → Sensor with current recipe count
2. **Recipe Display** → Calendar or custom card for featured recipe
3. **Shopping List Export** → Todo entities from recipe ingredients
4. **Serving Size Control** → Automation to trigger scaling
5. **Kitchen Display** → Browser card showing current recipe steps
6. **Pantry Integration** → Complement Grocy with aisle/pantry config
7. **Search Service** → Custom service to search recipes by name/tag
8. **Statistics** → Sensor tracking recipes prepared, ingredients used

**Competitive Advantages:**
- **Local-first** - No cloud dependency (like Tandoor, unlike Bring!)
- **Lightweight CLI** - Lower resource footprint than Grocy/Mealie
- **Flexible** - Works with any recipe source (not locked to app)
- **Automation-friendly** - Native HA entity types
- **Open format** - YAML-based recipes (vs proprietary databases)
- **Composable** - Can layer with Grocy for full kitchen automation

---

## 4. Integration Methods: REST API, MQTT, WebSocket, Polling

### Local Polling (Most Common for CLI-like integrations)

**How it works:**
- Integration periodically calls local REST API
- Polls on configured interval (default 30 seconds)
- DataUpdateCoordinator manages schedule

**Pros:**
- Simple to implement
- Works with any HTTP server
- No special protocol needed
- Good for stateless operations

**Cons:**
- Latency (up to polling interval)
- Overhead if polling frequently

**Best for:** CookCLI integration (could expose local HTTP server)

**Example with CookCLI:**
```python
async def _async_update_data(self):
    """Fetch data from CookCLI server."""
    return await self.client.get_recipes()
    # Returns: {"recipes": [...], "total": N, "updated": timestamp}
```

### REST API (Full-featured)

**Official HA REST API:**
- Authenticate with long-lived access token
- Call HA services from external systems
- Update entities from CLI

**How it works:**
```bash
# From CookCLI, trigger HA automation
curl -X POST https://ha-server:8123/api/services/automation/trigger \
  -H "Authorization: Bearer TOKEN" \
  -d '{"entity_id": "automation.cook_recipe"}'
```

**Use case:** CookCLI could call Home Assistant services on events

### MQTT (Message Broker - 44% of Installs)

**How it works:**
- Lightweight pub/sub message protocol
- CookCLI publishes recipe updates
- HA MQTT integration subscribes
- Bi-directional communication

**Pros:**
- Real-time updates (no polling lag)
- Efficient for frequent updates
- Supports commands (scaling, search)
- Standard protocol

**Cons:**
- Requires MQTT broker setup
- More complex implementation
- Higher barrier for simple setups

**Example:**
```
Topics:
- cookcli/recipe/search/request → CookCLI listener
- cookcli/recipe/search/response → HA listener
- cookcli/recipe/count → HA sensor

HA receives: {"topic": "cookcli/recipe/count", "payload": "47"}
```

### WebSocket (Real-time bidirectional)

**How it works:**
- Persistent connection
- Server can push updates immediately
- Client can request data on demand

**Pros:**
- True real-time
- Bi-directional
- Lower latency than polling

**Cons:**
- More complex server implementation
- Not necessary for simple recipes

**Not recommended for:** CookCLI (overkill for typical use case)

### Local Push (Best for Events)

**How it works:**
- Integration pushes data when something changes
- HA listens on configured port
- Uses webhooks or callbacks

**Pros:**
- Immediate updates
- No polling overhead
- Event-driven architecture

**Cons:**
- More complex setup
- Requires firewall port exposure
- Network latency

**Example for CookCLI:**
```
CookCLI detects recipe change →
  HTTP POST to HA webhook →
  HA updates sensor/calendar
```

### Recommended Approach for CookCLI

**Phased implementation:**

1. **Phase 1 (MVP):** Local polling via HTTP
   - CookCLI opens HTTP server on port 9090
   - HA integration polls every 60 seconds
   - Simple to implement, works offline

2. **Phase 2 (Enhancement):** MQTT support
   - Optional for users who have MQTT broker
   - Real-time updates when recipes change
   - Service to trigger searches

3. **Phase 3 (Advanced):** WebSocket for UI
   - Real-time recipe display
   - Kitchen display system integration
   - Advanced automations

---

## 5. What Makes HA Integrations "Sticky" (Drives Ongoing Usage)

### Primary Retention Drivers

#### 1. Solves a Specific Pain Point
- **Problem:** Manual shopping list management
- **Solution:** Mealie/Grocy auto-sync
- **Result:** Users return daily because it adds value

**For CookCLI:**
- Pain point: Switching between recipe CLI and HA dashboard
- Solution: Recipe info in HA dashboard
- Usage trigger: Daily meal planning, recipe browsing

#### 2. Automation Foundation
- **Why:** Automations are the core value of Home Assistant
- **Example:** "Show me today's recipe when I wake up"
- **Integration type:** Trigger + Action combo

**Sticky pattern:** Integration that enables new automations users couldn't do before

**For CookCLI:**
- Automation: "Alert when recipe modified" → Update dashboard
- Automation: "Scale recipe for 4 people" → Show updated quantities
- Automation: "Export shopping list" → Add to Grocy pantry

#### 3. Visual/Dashboard Integration
- **Why:** Users see it every day
- **Example:** Adaptive Lighting (auto-color temp) vs hidden background service
- **Metric:** Cards on dashboard = recurring visibility

**2025 trend:** Pre-built smart cards that suggest useful data to display

**For CookCLI:**
- Suggested card: "Today's recipe" with step counter
- Suggested card: "Recipe browser" dropdown
- Suggested card: "Last cooked" meal history

#### 4. Low Friction Updates
- **Why:** Abandoned integrations are uninstalled
- **HACS solves this:** Auto-update notifications, one-click install

**Quality indicator:** Integrations distributed via HACS > manual installs

#### 5. Community & Documentation
- **Why:** Users need help, examples, best practices
- **Examples:** Blog posts, forum threads, template automations

**Sticky pattern:** Integration with active maintainer who shares usage patterns

**For CookCLI:**
- Share automation templates (meal planning examples)
- Dashboard templates showing recipe flow
- Integration with Mealie/Grocy examples

#### 6. Proactive Notifications
- **Why:** Passive visibility in dashboard isn't enough
- **Example:** "Expiration date approaching" (from Grocy)

**For CookCLI:**
- Push notification: "New recipe added"
- Notification: "Time to cook - recipe ready"
- Notification: "Recommended recipe based on pantry"

#### 7. Error Handling & Reliability
- **Why:** Broken integrations get removed
- **Pattern:** Graceful degradation, clear error messages

**Quality indicator:** Circuit breaker pattern, exponential backoff, meaningful logs

#### 8. Fast Performance
- **Why:** Slow integrations cause frustration
- **Pattern:** CoordinatorEntity prevents redundant updates

**For CookCLI:**
- Cache recipe list (don't re-parse every update)
- Lazy-load full recipe data
- Efficient JSON API responses

### Usage Metrics That Indicate Success

Based on Home Assistant analytics:

1. **Installation count** - Growing vs stable vs declining
2. **Update frequency** - Active development signal
3. **Issue resolution time** - Maintainer responsiveness
4. **Feature requests** - User demand for improvements
5. **Integration with automations** - How often used in actual HA setups

**For reference:**
- Mealie: 2,079 active installations (2024.7) - Growing
- MQTT: 44% of all active installations - Mission-critical
- ESPHome: Tens of thousands - Core ecosystem

---

## 6. Home Assistant Integration Types (Comparison)

### Type 1: Built-in Integration (Official)

**What it is:**
- Part of Home Assistant core repository
- Installed automatically
- Maintained by core team + community

**Requirements:**
- Pass integration quality scale
- Full documentation
- Comprehensive tests
- Long-term maintenance commitment

**Advantages:**
- Auto-updates with HA
- Highest quality bar
- Best documentation
- Native config UI

**Disadvantages:**
- High barrier to entry
- Slower release cycle
- Must maintain forever
- Requires PR review process

**Example:** Mealie (now official), MQTT, Calendar

**Timeline:** 6-12 months from HACS to official consideration

---

### Type 2: HACS Custom Component

**What it is:**
- Community-developed integration
- Installed via HACS UI
- Git-based distribution
- Auto-update capability

**Requirements:**
- GitHub repository
- README with docs
- Semantic versioning
- manifest.json

**Advantages:**
- Faster development
- Flexible release schedule
- Easier to change direction
- Lower maintenance burden
- Large community (100k+ repositories)

**Disadvantages:**
- Less official support
- Variable quality
- User must install HACS first
- No automatic HA updates

**Example:** Grocy, SmartIR, Alarmo, ZHA Toolkit

**Timeline:** Ready in 1-2 months, can be maintained indefinitely

**How to get into HACS:**
1. Create GitHub repo with proper structure
2. Add to HACS repository (hacs/integration)
3. PR merged → automatically available in HACS UI

---

### Type 3: Add-on (OS Specific)

**What it is:**
- Containerized service running in Home Assistant OS
- Installed via Home Assistant Add-on Store
- Manages dependencies and config

**Requirements:**
- Docker container
- add-on manifest
- documentation
- Home Assistant OS support

**Advantages:**
- Full control over environment
- Can run complex services (Grocy, Mealie servers)
- Auto-restarts on failure
- Shared file access with HA

**Disadvantages:**
- Home Assistant OS only (not HA Container, HA Core)
- Higher resource usage
- Slower startup time
- More complex testing

**Example:** Grocy add-on, Mealie add-on, Mosquitto (MQTT broker)

**Use case for CookCLI:**
- If CookCLI needs to run as service alongside HA
- If you want to manage CookCLI from HA UI
- If you want shared storage with HA database

---

### Type 4: REST Sensor / Template Integration

**What it is:**
- Minimal configuration
- No custom code needed
- Uses built-in REST or Template integrations

**How it works:**
```yaml
rest:
  - name: Recipe Count
    resource: http://localhost:9090/api/recipes/count
    value_template: "{{ value_json.total }}"
    scan_interval: 300
```

**Advantages:**
- Dead simple setup
- No HACS needed
- Configuration in YAML
- Fast to prototype

**Disadvantages:**
- Limited functionality
- No config flow UI
- No error handling
- No service calls

**Use case for CookCLI:**
- Quick proof of concept
- Users who want lightweight integration
- Simple recipe count/status display

---

### Type 5: AppDaemon Script

**What it is:**
- Python 3 code running in AppDaemon container
- Event-driven automation scripting
- More flexible than automations

**Use case:**
- Complex recipe scaling logic
- Custom shopping list generation
- AI-powered recommendations

**Not recommended for:** CookCLI (too heavy-weight for integration)

---

### Recommended Path for CookCLI

**Phase 1: HACS Custom Component**
- Fastest to market
- Good for early adoption
- Flexible for iteration
- Can upgrade to official later

**Phase 2: REST Sensor Alternative**
- For users who don't want HACS
- Simple HTTP polling option
- Lower barrier to entry

**Phase 3: Official Integration**
- Once proven user adoption
- Stable API/features
- Community feedback incorporated
- Ready for core integration

**Why not add-on?**
- CookCLI is CLI-based, not a service
- Can integrate via HTTP, no need for container
- Users likely have it running separately
- Adds complexity for little benefit

---

## 7. Common Entity Types Used in Successful Integrations

### Most Common Entity Types

#### Sensor (Numeric/String Values)

**What it measures:**
- Temperature, humidity, power, energy
- Counts (recipes, ingredients, calories)
- Strings (status, recipe name, weather)
- Percentages (battery, storage)

**Why popular:**
- Universal data container
- Supports templates
- Easy to graph/analyze
- History integration

**For CookCLI:**
```python
class RecipeCountSensor(SensorEntity):
    """Total number of recipes."""
    _attr_name = "Recipe Count"
    _attr_unique_id = "cookcli_recipe_count"

    @property
    def state(self):
        return self.coordinator.data["total_recipes"]

    @property
    def unit_of_measurement(self):
        return "recipes"
```

**Usage in HA:**
- Dashboards: Show as big number
- Templates: "You have {{ states('sensor.recipe_count') }} recipes"
- Automations: Trigger when count changes

---

#### Binary Sensor (True/False States)

**What it detects:**
- Door open/closed
- Motion detected
- Connection online/offline
- Recipe ready/not ready

**Why popular:**
- Simple automation conditions
- Visual icons (dot, status)
- Low overhead

**For CookCLI:**
```python
class RecipeModifiedBinarySensor(BinarySensorEntity):
    """True if recipe was modified since last check."""
    _attr_name = "Recipe Modified"
    _attr_unique_id = "cookcli_recipe_modified"

    @property
    def is_on(self):
        return self.coordinator.data["modified_recently"]

    @property
    def device_class(self):
        return BinarySensorDeviceClass.UPDATE
```

**Usage:**
- Automation trigger: "When recipe is modified, notify me"
- Dashboard condition: "Show alert if recipes modified"

---

#### Todo Entity (To-Do Lists)

**What it tracks:**
- Shopping list items
- Tasks to complete
- Meal plan items
- Action items

**Why popular:**
- Bidirectional (add/remove in HA)
- Marks items done
- Integrates with reminders
- Mobile app support

**For CookCLI:**
```python
class RecipeIngredientsTodo(TodoListEntity):
    """Shopping list from current recipe."""
    _attr_name = "Recipe Ingredients"
    _attr_unique_id = "cookcli_ingredients"

    async def async_create_todo_item(self, item):
        """Add ingredient."""
        self.coordinator.data["ingredients"].append(item)

    async def async_update_todo_item(self, item_id, *, is_done=None):
        """Mark ingredient as obtained."""
        # Sync back to cookcli pantry
```

**Usage:**
- Mark ingredients as obtained
- Show checklist on dashboard
- Integrate with shopping trip reminders

---

#### Calendar Entity (Scheduled Events)

**What it shows:**
- Meal plans (breakfast, lunch, dinner)
- Cooking schedules
- Ingredient delivery dates
- Recipe development schedule

**Why popular:**
- Visual timeline
- Native automation triggers
- Mobile app integration
- Easy to share/calendar

**For CookCLI:**
```python
class RecipeMealPlanCalendar(CalendarEntity):
    """Today's meal plan from recipes."""
    _attr_name = "Meal Plan"
    _attr_unique_id = "cookcli_meal_plan"

    @property
    def event(self):
        """Return current/next event."""
        return CalendarEvent(
            summary="Pasta Carbonara",
            start=datetime.now(),
            end=datetime.now() + timedelta(hours=1)
        )
```

**Usage:**
- Show today's meal
- Trigger morning notification
- Suggest recipes based on schedule

---

#### Select Entity (Dropdown/Choices)

**What it controls:**
- Serving size selector
- Meal category filter
- Cuisine type selector
- Recipe difficulty level

**Why popular:**
- Cleaner than text input
- Automation-friendly
- UI widget friendly

**For CookCLI:**
```python
class RecipeScaleSelect(SelectEntity):
    """Select serving size multiplier."""
    _attr_name = "Recipe Scale"
    _attr_unique_id = "cookcli_scale"
    _attr_options = ["0.5", "1.0", "1.5", "2.0", "3.0"]

    async def async_select_option(self, option):
        """Change recipe scale."""
        self.coordinator.scale_recipe(float(option))
```

**Usage:**
- "Scale recipe for 4 people" → select option
- Automation: "Set scale to 1.5 for guests"
- Dashboard: Dropdown to choose serving size

---

#### Switch Entity (On/Off Control)

**What it controls:**
- Enable/disable features
- Toggle cookbook modes
- Start/stop cooking timer
- Activate prep mode

**Note:** Legacy in 2025 (being deprecated for other entity types)

**For CookCLI (example):**
```python
class RecipePrepModeSwitch(SwitchEntity):
    """Toggle recipe prep mode."""
    _attr_name = "Prep Mode"
    _attr_unique_id = "cookcli_prep_mode"

    async def async_turn_on(self, **kwargs):
        """Enable prep mode."""
        # Show ingredient checklist, shopping list
```

---

#### Other Notable Entity Types

| Entity Type | Use Case | Stickiness |
|---|---|---|
| **Button** | Trigger actions (scale recipe, export list) | Medium |
| **Number** | Numeric input (servings, temperature) | Low-Medium |
| **Image** | Recipe photo, nutrition label | High (visual) |
| **Media Player** | Play cooking timer audio | Medium |
| **Alarm Control Panel** | Ingredient expiration warnings | Low |
| **Humidifier** | N/A for recipes | N/A |
| **Fan** | N/A for recipes | N/A |

---

## 8. Integration Quality Scale (Official HA Standard)

Home Assistant has an official **Integration Quality Scale** with 4 tiers:

### Bronze Tier (Minimum Requirement)

**Requirements:**
- Entity discovery
- Basic error handling
- Config flow
- Basic tests

**What integrations typically look like:** Emerging integrations

**Time to achieve:** 2-4 weeks

---

### Silver Tier (Good Standard)

**Requirements:**
- All Bronze requirements
- Comprehensive error handling
- Circuit breaker pattern
- Exponential backoff retries
- Meaningful error messages
- Type annotations
- Documentation
- Config recovery after HA restart

**What integrations typically look like:** Most popular HACS integrations

**Time to achieve:** 6-8 weeks

**Example:** Alarmo, Frigate, Local Tuya

---

### Gold Tier (High Quality)

**Requirements:**
- All Silver requirements
- Async operations
- Data validation
- Comprehensive tests (70%+ coverage)
- Clear entity naming/icons
- Metrics/statistics
- Advanced config options

**What integrations typically look like:** Nearly all official integrations

**Time to achieve:** 12+ weeks

**Example:** Most built-in integrations

---

### Platinum Tier (Excellence)

**Requirements:**
- All Gold requirements
- 90%+ test coverage
- Performance optimization
- Advanced async patterns
- Community feedback integration
- Maintainer responsiveness

**What integrations typically look like:** Flagship integrations (MQTT, Zigbee)

**Time to achieve:** 6+ months minimum

---

### Recommended Path for CookCLI Integration

**Target: Silver Tier (8-12 weeks of development)**

**Critical elements:**
1. DataUpdateCoordinator for efficient polling
2. Config flow for easy setup
3. Circuit breaker pattern for API calls
4. Proper error handling and logging
5. Type annotations throughout
6. Basic unit tests
7. Documentation with examples
8. Sensor + Todo entity types minimum

**Nice-to-have for Gold:**
1. Calendar entity for meal planning
2. Advanced automations guide
3. Integration with Mealie/Grocy
4. Performance optimization

---

## 9. CookCLI Integration Opportunity Assessment

### Market Opportunity

**Current State:**
- Mealie: 2,079 active installations (meal planning + shopping lists)
- Grocy: Unknown (HACS custom component)
- Bring!: Unknown (less integrated than Mealie)
- **CookCLI: 0 active installations (no integration exists)**

**Potential Users:**
- Everyone with HA + CookCLI installed (estimated: 10-500 based on CLI market)
- Home chefs who want recipe automation
- Families sharing recipes with Home Assistant dashboard
- Users combining multiple recipe sources

### Competitive Analysis

| Feature | Mealie | Grocy | Bring! | CookCLI |
|---|---|---|---|---|
| **Type** | Official Integration | HACS Custom | Official | Not yet available |
| **Installation Count** | 2,079 | Unknown | Unknown | 0 |
| **Self-hosted** | Yes | Yes | No | Yes (CLI) |
| **Meal Planning** | Yes | No | No | No |
| **Inventory Tracking** | No | Yes | No | No |
| **Recipe Browser in HA** | No | No | No | **Opportunity** |
| **Shopping List** | Yes | Yes | Yes | **Opportunity** |
| **Local Polling** | Yes | Yes | No | **Yes (HTTP)** |
| **YAML-based Recipes** | No | No | No | **Yes** |
| **CLI-first Design** | No | No | No | **Yes** |

### Unique Value Propositions

**vs Mealie:**
- Lightweight CLI tool (no server overhead)
- Flexible recipe format (any text/YAML)
- Composable (works with any system)
- Lower barrier to entry

**vs Grocy:**
- Recipe-focused (not inventory-heavy)
- Simpler setup
- Better for recipe browsing/discovery

**vs Bring!:**
- Self-hosted (privacy + no subscription)
- Integration with CLI tool (developers prefer)
- Extensible

**vs none (current state):**
- CLI users can now use recipes in automations
- HA users can browse recipes on dashboard
- Integration with Grocy/Mealie for full kitchen automation

### Recommended Feature Set (MVP)

**Entities to expose:**
1. **Sensor** - Recipe count, last modified, featured recipe
2. **Todo** - Shopping list from selected recipe
3. **Calendar** - Optional: Meal plan if combined with Mealie
4. **Select** - Recipe selector dropdown
5. **Button** - Export shopping list action

**Services to provide:**
1. `cookcli.search_recipe` - Find recipes by name
2. `cookcli.scale_recipe` - Adjust serving size
3. `cookcli.export_ingredients` - Create shopping list

**Automations to suggest:**
1. "Show today's recipe on kitchen display at 6pm"
2. "Notify when recipe is marked complete"
3. "Add recipe ingredients to Grocy pantry"
4. "Weekly meal plan suggestion based on ingredients"

### Implementation Timeline

**Phase 1 (4-6 weeks): MVP as HACS Custom Component**
- Local HTTP polling integration
- Sensor + Todo entities
- Basic config flow
- Target: Silver quality tier

**Phase 2 (4-6 weeks): Enhanced Features**
- Calendar entity
- Service calls for automation
- Integration examples with Mealie/Grocy
- Documentation and templates

**Phase 3 (4-8 weeks): Official Integration**
- Community feedback incorporated
- Test coverage improved
- Performance optimizations
- Submit to official HA core

---

## 10. Key Insights for CookCLI Integration Decision

### Why Home Assistant Integration Matters

1. **Discoverability:** HA users looking for recipe solutions will find CookCLI
2. **Stickiness:** Integration into daily HA automations = repeated usage
3. **Ecosystem:** Pairs well with existing HA ecosystem (Grocy, Mealie, Zigbee devices)
4. **Authority:** "Official HA integration" is trust signal
5. **Growth:** HA ecosystem growing (2M installations in 2025)

### Critical Success Factors

1. **Local-first architecture** - HTTP server, no cloud
2. **Low friction setup** - Config flow, not YAML required
3. **Dashboard visibility** - Pre-built smart cards
4. **Automation enablement** - Services for recipe selection/scaling
5. **Grocy/Mealie interop** - Complement rather than compete
6. **Active maintenance** - Regular updates, responsive to issues

### Risks & Mitigation

| Risk | Mitigation |
|---|---|
| **Users prefer Mealie/Grocy** | Position as complementary, not replacement |
| **Low initial adoption** | Market to existing CookCLI users first, then HA community |
| **Maintenance burden** | Start with HACS, don't commit to official until proven |
| **Technical complexity** | Use established patterns (DataUpdateCoordinator, config flow) |
| **Poor first impression** | Target Silver quality tier minimum before release |

### Recommended Decision

**Verdict: Worth pursuing**

**Reasoning:**
- Market exists (2K+ Mealie users, growing HA ecosystem)
- Competitive advantage (CLI-native, lightweight)
- Reasonable effort (8-12 weeks for MVP)
- Strategic fit (extends CookCLI reach into smart home market)
- Low risk (HACS distribution, can iterate without blocking)

**Recommended approach:**
1. Start with HACS custom component (fastest path)
2. Focus on 3 core entities: Sensor, Todo, Select
3. Solve recipe discovery + shopping list automation
4. Gather feedback from early adopters
5. Plan official integration once proven

---

## References & Sources

### Official Home Assistant Resources
- [Home Assistant Integrations Directory](https://www.home-assistant.io/integrations/)
- [Home Assistant Analytics Dashboard](https://analytics.home-assistant.io/integrations/)
- [Integration Quality Scale Documentation](https://developers.home-assistant.io/docs/core/integration-quality-scale/)
- [DataUpdateCoordinator Guide](https://developers.home-assistant.io/docs/integration_fetching_data/)
- [Home Assistant Developer Docs](https://developers.home-assistant.io/)

### Integration Best Practices
- [HACS Community Store](https://www.hacs.xyz/)
- [Building HA Integrations - Andrew Doering's Blog](https://andrewdoering.org/blog/2025/home-assistant-loggamera/)
- [HACS Integration Blueprint](https://github.com/jpawlowski/hacs.integration_blueprint)
- [Awesome Home Assistant - Community Curated List](https://github.com/frenck/awesome-home-assistant)

### Food/Recipe Integrations
- [Mealie Integration Documentation](https://www.home-assistant.io/integrations/mealie/)
- [Mealie Home Assistant Integration GitHub](https://github.com/mealie-recipes/mealie-hacs)
- [Grocy Custom Integration](https://github.com/custom-components/grocy)
- [Bring! Integration Documentation](https://www.home-assistant.io/integrations/bring/)
- [Tandoor Recipes HA Integration Issue](https://github.com/TandoorRecipes/recipes/issues/796)

### Community Resources
- [Home Assistant Community Forum](https://community.home-assistant.io/)
- [SmartHomeScene - Integration Reviews](https://smarthomescene.com/blog/best-hacs-integrations-for-home-assistant/)
- [XDA - HA Integration Guides](https://www.xda-developers.com/the-coolest-hacs-integrations-for-home-assistant-users/)
- [Integrating Grocy with HA - Phil Hawthorne](https://philhawthorne.com/automating-your-shopping-list-with-home-assistant-and-grocy/)

### Market Data
- [Home Assistant 2025.5 Release - 2 Million Users](https://www.home-assistant.io/blog/2025/05/07/release-20255/)
- [InfluxDB - 9 HA Integrations Guide](https://www.influxdata.com/blog/9-home-assistant-integrations-how-use-them/)
- [Awesome Home Assistant Ranked Gallery](https://github.com/legovaer/best-of-hassio)

---

## Appendix: Home Assistant Entity Type Quick Reference

### Entity Domains Available

```
alarm_control_panel   - Security system states
automation           - Automation triggers/controls
binary_sensor        - On/off sensors
button               - Action triggers
calendar             - Event scheduling
camera               - Media streaming
climate              - Temperature control
cover                - Doors, blinds, garage
device_tracker       - Location tracking
fan                  - Fan control
group                - Entity grouping
image                - Image display
input_boolean        - User toggle
input_datetime       - Date/time picker
input_number         - Numeric slider
input_select         - Dropdown selector
input_text           - Text input
light                - Lights + color/brightness
lock                 - Smart locks
media_player         - Media control
number               - Numeric sensor
person                - Person tracking
scene                - Preset states
script               - Automation script
select               - Dropdown control
sensor               - Numeric/string values
siren                - Alert sounds
switch               - On/off control
todo                 - To-do lists
update               - Update status
vacuum               - Robot vacuums
water_heater         - Water heater control
weather              - Weather data
```

### For CookCLI: Recommended Primary Entities

1. **sensor** - Recipe count, featured recipe, last modified
2. **todo** - Shopping list items, ingredient checklist
3. **select** - Recipe selector, scale multiplier
4. **calendar** - Meal plan (if integrated with scheduling)
5. **button** - Export list, trigger scaling

### Icon Reference (for frontend)

```yaml
sensor.recipe_count:
  icon: mdi:book-open-page-variant

todo.ingredients:
  icon: mdi:cart

select.recipe_selector:
  icon: mdi:food

binary_sensor.recipe_modified:
  icon: mdi:pencil
```

---

**Document Version:** 1.0
**Last Updated:** March 2026
**Status:** Research Complete - Ready for Integration Planning
