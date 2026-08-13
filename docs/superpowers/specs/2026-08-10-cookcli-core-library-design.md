# cookcli-core: CookCLI commands as a library

**Date:** 2026-08-10
**Status:** Approved, ready for planning

## Problem

CookCLI's commands are reachable only by running the binary. Three codebases
independently wrap the same underlying crates:

- **CookCLI** — `src/`, commands shaped as `run(&Context, XxxArgs) -> Result<()>`
  that take clap structs and `println!`.
- **`editor/packages/cooklang-native`** — a NAPI-RS addon with its own `parse`,
  `generateShoppingList`, `renderReport`, `findRecipe`, plus a `shopping_list.rs`
  wrapper over `cooklang::shopping_list`.
- **`cook.md/cookbot/crates/tui/src/cooklang/`** — its own `parser.rs`,
  `render.rs`, `shopping_list.rs` (~860 lines).

The immediate goal is to let the Cooklang editor call CookCLI's command logic
during AI sessions, so the AI operates on the same recipe/shopping-list/pantry
semantics the CLI does instead of a parallel reimplementation that drifts.

CookCLI already has a `[lib]` target, but its API is CLI-shaped: nothing returns
data, warnings are logged and discarded, and the public modules are the command
modules themselves.

## Decisions

| # | Decision | Choice |
|---|---|---|
| 1 | Integration surface | Typed Rust crate API (not MCP; MCP remains a possible later layer over it) |
| 2 | Command scope | `recipe`, `shopping_list`, `search`, `doctor`, `pantry`, `report` |
| 3 | Types at the boundary | Hybrid — re-export `cooklang` types for recipes, cookcli-owned DTOs for composed results |
| 4 | Crate structure | Workspace split: `cookcli-core` + `cookcli` |
| 5 | Errors | `thiserror` enum, `#[non_exhaustive]`, every result carries diagnostics |
| 6 | Input sources | Source enums (path or in-memory content) plus an opt-in `discover()` helper |
| 7 | Project scope | Split into two specs; this document is Spec 1 |

### 1. Integration surface

A typed Rust crate API. Commands become
`fn(&Context, Request) -> Result<Outcome<T>, CoreError>`, with the CLI reduced
to a formatting shell.

Rejected: an MCP server as the primary surface. MCP over `println!`-shaped
commands would be miserable, and the typed API has to exist either way. Once it
does, an MCP layer is a thin addition — but it is not part of this work.

### 2. Command scope

In: `recipe`, `shopping_list` (both generation and the persistent store),
`search`, `doctor`, `pantry`, `report`.

Out: `server` (already a library-ish axum router), `lsp` (the editor embeds
`cooklang-language-server` directly), `login`/`logout`/`update` (auth and
self-update are not editor concerns), `import` and `sync` (async and
network-bound; shelling out to the binary is acceptable for a one-shot import).

`report` was added to the original core five because it is a 151-line wrapper
over `cooklang-reports` and it already exists on the editor's NAPI surface as
`renderReport`, making it the highest consumer-value-per-unit-work item in the
set.

### 3. Types at the boundary

`cook recipe -f json` already serialises `cooklang::Recipe` directly, so the
domain types are serde-ready.

**Re-export `cooklang` types** for recipes. The editor parses with `cooklang` on
both sides of the wire already, so hiding it would be fiction.

**cookcli-owned DTOs** for results CookCLI composes itself and which have no
`cooklang` equivalent: `AggregatedList`, `Diagnostic`, `ValidationReport`,
`SearchHit`. These are CookCLI's own vocabulary and must not be pinned to
`cooklang`'s release cadence.

The version trap this manages: `cookcli` and the editor's `cooklang-native` both
pin `cooklang 0.18.5`, while cookbot's TUI pins `0.18.7`. Any `cooklang` type in
a public signature forces every consumer to resolve to the identical version.
Confining that to `Recipe` means a `cooklang` bump is visible to consumers
exactly where it should be, and invisible everywhere else.

### 4. Crate structure

A cargo workspace. `cookcli` stays at the repository root; `crates/core` is
added as a member.

Keeping the binary package at root preserves `include`, `build.rs`, the
`templates/`, `static/`, `seed/` and `locales/` paths, CI, release-please
configuration and the Homebrew formula. Relocating it would be tidier and would
cost significant packaging work for no functional gain.

**`cookcli-core` contains:**

| Area | Source today |
|---|---|
| `Context` + config discovery | `src/main.rs`, `src/lib.rs` (two divergent copies) |
| `recipe` | `src/recipe/read.rs` |
| formatters | `src/util/cooklang_to_*.rs`, `src/util/format.rs` |
| `shopping_list` (generate) | `src/shopping_list.rs` |
| `shopping_list` (store) | `src/server/shopping_list_store.rs` |
| `search` | `src/search.rs` |
| `doctor` | `src/doctor.rs` |
| `pantry` | `src/pantry.rs` |
| `report` | `src/report.rs` |

**`cookcli` keeps:** clap argument definitions, i18n/fluent, embedded assets,
`server`, `sync`, `lsp`, `import`, `update`.

**Dependency budget for `cookcli-core`:** `cooklang`, `cooklang-find`,
`cooklang-reports`, `camino`, `serde`, `serde_json`, `serde_yaml`, `thiserror`,
`tracing`, `tabular`, `yansi`. Explicitly not: `clap`, `axum`, `reqwest`,
`tokio`, `fluent`/`fluent-templates`, `rust-embed`, `askama`. Everything in core
is synchronous.

Verified during design: fluent/i18n is confined to server templates and is not
reachable from any of the six commands.

### 5. Errors and diagnostics

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    RecipeNotFound { name: String },
    Parse { name: String, diagnostics: Vec<Diagnostic> },
    Config { path: Utf8PathBuf, message: String },
    Io(#[from] std::io::Error),
}

pub struct Outcome<T> {
    pub value: T,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,          // Error | Warning | Hint
    pub message: String,
    pub location: Option<Location>,  // file + span
}
```

`anyhow` is adequate for a binary that prints and exits, but a consumer cannot
match on it. The editor needs to distinguish "recipe not found" from "parse
failed" from "aisle.conf is malformed" in order to render them differently.

`cooklang` parses leniently, returning a recipe *and* a warning report. Today
those warnings are `warn!`-logged and discarded — `src/shopping_list.rs` logs a
warning for a malformed aisle file and silently falls back to
`Default::default()`. That is precisely the information an AI session needs
most.

Every command therefore returns `Outcome<T>`. `doctor` stops being a special
case: it is "run the parse, collect the diagnostics, do not build the output,"
using the same `Diagnostic` type as everything else. Uniformity means the NAPI
layer writes one mapping instead of six. Consumers that do not care ignore the
field.

`CoreError` is public API; `#[non_exhaustive]` from day one keeps added variants
non-breaking.

### 6. Input sources and Context

```rust
pub enum RecipeSource { Path(Utf8PathBuf), Content { text: String, name: String } }
pub enum ConfigSource { Path(Utf8PathBuf), Inline(String), None }

pub struct Context { /* base_path, aisle, pantry */ }

impl Context {
    pub fn new(base_path: Utf8PathBuf) -> Self;              // touches nothing
    pub fn discover(base_path: Utf8PathBuf) -> Result<Self>; // CLI search order, opt-in
    pub fn with_aisle(self, src: ConfigSource) -> Self;
    pub fn with_pantry(self, src: ConfigSource) -> Self;
}
```

The editor's existing NAPI surface is entirely string-based — `parse(input)`,
`generateShoppingList(recipesJson, aisleConf?, pantryConf?)` where the configs
are text, not paths. That is not incidental: an editor works on unsaved buffers.
A path-only API cannot serve the case the editor cares about most, which is the
recipe currently being typed.

Core never touches the filesystem unless handed a path. `Context::discover()`
implements the CLI's exact search order (`./config/`, then the platform config
directory) as a one-line opt-in, so the search order is written once and shared
rather than cloned per consumer.

This also resolves an existing divergence: `src/lib.rs`'s `Context::aisle()`
checks only `./config/aisle.conf`, while `src/main.rs`'s checks that *and* falls
back to the global config directory. The lib copy is the one the current tests
exercise.

`Context` becomes the resolved configuration bundle rather than a bare
`base_path` wrapper.

### 7. API surface

One module per command:

```rust
recipe::read(&ctx, ReadRequest { source, scale })      -> Outcome<cooklang::Recipe>
shopping_list::generate(&ctx, GenerateRequest { .. })  -> Outcome<AggregatedList>
shopping_list::store::Store                            // .shopping-list / .shopping-checked
search::search(&ctx, SearchRequest { query })          -> Outcome<Vec<SearchHit>>
doctor::validate(&ctx, ValidateRequest { .. })         -> Outcome<ValidationReport>
pantry::{list, add, remove, update, depleted, expiring, recipes, plan}
report::render(&ctx, RenderRequest { .. })             -> Outcome<String>

format::{human, markdown, cooklang, latex, typst, schema}
```

Formatters keep their `&Recipe -> impl Write` signatures (streaming, no forced
allocation) and gain `_to_string` convenience variants.

**Naming.** `cooklang::shopping_list::ShoppingList` already means the *persisted
recipe-reference list*. What `cook shopping-list` generates — ingredients
aggregated and bucketed by aisle — is a different type. The generated one is
`AggregatedList`; the persisted one is re-exported unchanged. Giving both the
same name would cause bugs.

**Human formatter.** `format::human` takes an explicit `Style { Plain, Ansi }`
rather than reading `yansi`'s global state. A library must not emit escape codes
by default, and global mutable style state is not acceptable in a shared crate.
The CLI passes `Ansi`. Plain human text is also a plausible format for feeding
recipe context to an LLM.

## The shopping-list store

`src/server/shopping_list_store.rs` (440 lines) and the editor's
`packages/cooklang-native/src/shopping_list.rs` are wrappers over the same
`cooklang::shopping_list` API and the same `.shopping-list` / `.shopping-checked`
file pair. CookCLI's is the richer implementation: atomic writes, legacy
`.shopping_list.txt` migration, recipe items with multipliers, and a filesystem
watcher. The editor's is a thin subset.

This is real duplication, and it is currently trapped behind the `server`
feature — the exact feature a library consumer compiles out.

A related gap, deliberately **not** addressed in this spec: `cook shopping-list`
does not touch that store. It takes recipes as arguments, aggregates, prints and
exits. The persistent list — recipe references plus checked state — exists only
inside the web server, so the CLI cannot see the list the server and editor
share. Closing that gap requires new CLI verbs, which is a product decision and
belongs to Spec 2.

This spec moves the store into `core::shopping_list::store` and has the server
import it from there. Behaviour is unchanged.

## Project decomposition

**Spec 1 (this document) — `cookcli-core` extraction.** Everything above: the
crate, six commands, the store lift, the CLI reduced to a shell, published to
crates.io. One repository, fully verifiable by the existing test suite.

**Spec 2 — consumers and the persistent-list CLI.** Rewiring the editor's
`cooklang-native` onto `cookcli-core`, and new `cook shopping-list` subcommands
for the persisted list. Both depend on Spec 1 being published, and both involve
product decisions that are not extraction questions.

They cannot land atomically in any case — the crates.io publish must precede
consumer adoption. Combining them would block the extraction behind the design
of a new CLI command surface, which is a false dependency.

### What Spec 2 will and will not collapse

Of the editor's ~11 NAPI exports:

| Export | Collapses onto core? |
|---|---|
| `parse` | Yes — `recipe` |
| `generateShoppingList` | Yes — `shopping_list::generate` |
| `findRecipe` | Yes — lookup |
| `renderReport` | Yes — `report` |
| `parseShoppingList`, `writeShoppingList`, `parseChecked`, `writeCheckEntry`, `checkedSet`, `compactChecked` | Yes — `shopping_list::store` |
| `parseMenu` | No — needs menu/`build` logic, out of scope |
| `startSync`, `stopSync`, `getSyncStatus`, `onSyncStatusChanged` | No — `sync` feature, out of scope |
| `LspServer` | No — editor embeds the LSP directly |

## Migration sequence

Strangler pattern, one command per commit.

0. **Foundation** — `crates/core` skeleton: `CoreError`, `Diagnostic`,
   `Outcome`, `RecipeSource`/`ConfigSource`, `Context` (reconciling the two
   divergent copies), formatters. Nothing calls it yet.
1. **`recipe`** — largest surface, exercises every formatter, proves the pattern.
2. **`shopping_list` (generate)** — first command with aisle/pantry config,
   proves `ConfigSource`.
3. **`search`** — trivial, fast confirmation.
4. **`doctor`** — where `Diagnostic` becomes the actual return type.
5. **`pantry`** — 1,260 lines, the bulkiest.
6. **`report`**.
7. **Store lift** — `server/shopping_list_store.rs` into
   `core::shopping_list::store`; server imports from core. Last, because it is
   the only step touching server code.

After each step, the command's `run()` in `cookcli` is: parse clap args, build
request, call core, format, print. A `run()` that still contains recipe logic
after its step means that step is not done.

## Verification

The existing suite is the contract and does not change: 4,216 lines across
`tests/`, driving the real binary through `assert_cmd`, plus 22 insta snapshots.
It pins CLI behaviour from the outside, so it stays valid while the internals
move.

- `cargo test` green after every step.
- `cargo fmt` and `cargo clippy` clean, per the repository rule.
- **No snapshot is regenerated.** Running `cargo insta accept` during this work
  means a behaviour change slipped in. Any accepted snapshot requires an
  explicit written justification in the commit message.

New tests live in `crates/core` and cover what the CLI cannot reach:

- `RecipeSource::Content` — parsing an in-memory buffer.
- `ConfigSource::Inline` — aisle and pantry configuration supplied as text.
- Diagnostics surviving on the success path, specifically the malformed
  `aisle.conf` case that currently logs a warning and silently falls back to
  `Default::default()`.

That last item is a deliberate behaviour **fix**, and the one place CLI output
legitimately changes. It gets its own commit and its own snapshot update, so the
change is visible in review rather than folded into a refactor.

## Publishing

`cookcli-core` is declared with both `version` and `path`, so local builds use
the workspace copy while the published crate resolves from crates.io.
`cookcli-core` publishes first; `cookcli` then bumps to depend on the released
version. That ordering is what unblocks Spec 2.

## Risks

**API designed against one consumer.** The extraction is driven by the CLI's
requirements, and the editor is not exercising the API while it is being shaped.
Mitigation: treat the editor's existing `index.d.ts` as a checklist — every
function there that is in scope must be expressible in `cookcli-core` — without
doing the wiring.

**`cooklang` version alignment.** Re-exporting `cooklang::Recipe` hard-pins
consumers. cookbot is already on 0.18.7 against CookCLI's 0.18.5. This surfaces
in Spec 2, not here, but the alignment cost is real and belongs to whoever
adopts core.

**Public API of the current `cookcli` lib target.** `src/lib.rs` currently
exports the command modules. Reducing them to a CLI shell changes that surface.
It exists to support the test suite rather than external consumers, but it
warrants a version bump.
