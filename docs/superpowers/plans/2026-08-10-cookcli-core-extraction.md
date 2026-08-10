# cookcli-core Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract CookCLI's `recipe`, `shopping_list`, `search`, `doctor`, `pantry` and `report` commands into a new `cookcli-core` crate with a typed, consumer-facing API, reducing `cookcli`'s command modules to clap-and-print shells.

**Architecture:** A cargo workspace with `cookcli` at the repository root (preserving all packaging paths) and `crates/core` as a new member. Every command becomes `fn(&Context, Request) -> Result<Outcome<T>, CoreError>`. `Outcome<T>` carries diagnostics that are currently `warn!`-logged and discarded. Inputs arrive via `RecipeSource`/`ConfigSource` enums so in-memory buffers work as well as paths. Migration is a strangler pattern: one command per task, with the existing binary-level test suite as the unchanged contract.

**Tech Stack:** Rust 2021, `cooklang` 0.18.5, `cooklang-find`, `cooklang-reports`, `thiserror`, `camino`, `serde`, `insta`, `assert_cmd`.

**Spec:** `docs/superpowers/specs/2026-08-10-cookcli-core-library-design.md`

**Branch:** `feat/cookcli-core-library` (already created from `0c3feb3`)

---

## Prerequisite: compiled front-end assets

`build.rs` hard-fails without `static/css/output.css` and `static/js/editor.bundle.js`. Both are generated and gitignored, so **a fresh clone or a fresh git worktree cannot build at all** until they exist:

```
error: missing compiled front-end assets:
  - static/css/output.css  (generate with: npm run build-css)
  - static/js/editor.bundle.js  (generate with: npm run build-js)
```

Produce them with `npm install && npm run build-css && npm run build-js`, or copy them from an existing checkout of the same commit — they are pure build products of `static/css/input.css`, `tailwind.config.js` and the JS sources, so copying is safe when those inputs match.

Nothing in this plan touches the web UI, so once the assets are present they stay valid for every task.

---

## Critical Constraint: The Test Suite Is The Contract — Except For Shopping Lists

`tests/` contains 4,216 lines driving the real binary through `assert_cmd`, plus 22 insta snapshots in `tests/snapshots/`. **These files are not modified by this plan**, with two exceptions, both stated explicitly where they occur (Task 6 adds characterization snapshots; Task 10 is a deliberate behaviour fix).

### Two numbers, two meanings

The repository root is *both* the workspace root and a member package, so Cargo scopes bare commands to `cookcli` alone. That gives two distinct figures, and you must check both:

| Command | Scope | Baseline | Meaning |
|---|---|---|---|
| `cargo test` | `cookcli` only | `296 passed; 0 failed; 26 ignored` | **The CLI behaviour contract.** Must not move, except Task 6 (→ 310). |
| `cargo test --workspace` | `cookcli` + `cookcli-core` | `306` after Task 1 | Everything, including core's unit tests. Grows every task. |

Keeping them separate is useful: if `cargo test` moves you changed CLI behaviour, regardless of what core's tests say.

**This bites in CI.** `cargo test`, `cargo clippy` and `cargo fmt` at the root all skip `cookcli-core` entirely, so core's tests would run nowhere. `.github/workflows/test.yml` was updated in Task 1 to use `--workspace` (and `cargo fmt --all`) for exactly this reason. If you add a crate, check CI actually runs its tests.

- **Never run `cargo insta accept`.** If a snapshot fails, you introduced a behaviour change — find it and fix the code, not the snapshot. The only exception is Task 6, which creates new snapshots deliberately.
- `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` must be clean before every commit (repository rule from `CLAUDE.md`).

### The coverage hole you must know about

**All 26 ignored tests are shopping-list tests.** Every one:

| File | Ignored |
|---|---|
| `tests/shopping_list_test.rs` | 18 — the entire file |
| `tests/snapshot_test.rs` | 3 — `shopping_list_{categorized,json,plain}` |
| `tests/output_formats_test.rs` | 3 — all shopping-list |
| `tests/cli_integration_test.rs` | 2 — both shopping-list |

They are marked with a bare `#[ignore]` and no reason. Running them shows why:

```bash
cargo test --test shopping_list_test -- --ignored
# 7 passed; 11 failed
```

They are stale — they assert output the command no longer produces (a `"Shopping List"` header, for instance). So `shopping_list` — the command with the most intricate extraction in this plan (recursive `extract_ingredients`, aisle and pantry loading, four output builders, aisle categorisation) — currently has **effectively zero regression coverage**.

Recipe, pantry, doctor, search, build and schema all have real coverage and are safe to refactor against the suite. Shopping lists are not. **Task 6 exists to close that gap before Task 7 touches the code.**

Repairing the 11 broken ignored tests is deliberately *not* part of this plan. Making them pass means deciding what the output *should* be, which is a product question; characterization snapshots capture what it *is*, which is what a refactor needs.

---

## File Structure

**New crate — `crates/core/`:**

| File | Responsibility |
|---|---|
| `Cargo.toml` | Package manifest, minimal dependency set |
| `src/lib.rs` | Public re-exports, module declarations |
| `src/error.rs` | `CoreError` |
| `src/diagnostic.rs` | `Diagnostic`, `Severity`, `Location` |
| `src/outcome.rs` | `Outcome<T>` |
| `src/source.rs` | `RecipeSource`, `ConfigSource` |
| `src/context.rs` | `Context`, `discover()`, config resolution |
| `src/parser.rs` | `PARSER` static, `parse_recipe`, diagnostic conversion |
| `src/format/mod.rs` | `Style`, formatter re-exports |
| `src/format/human.rs` etc. | Moved from `src/util/cooklang_to_*.rs` |
| `src/recipe.rs` | `ReadRequest`, `read()` |
| `src/shopping_list/mod.rs` | `GenerateRequest`, `generate()`, `AggregatedList` |
| `src/shopping_list/store.rs` | Moved from `src/server/shopping_list_store.rs` |
| `src/search.rs` | `SearchRequest`, `SearchHit`, `search()` |
| `src/doctor.rs` | `ValidateRequest`, `ValidationReport`, `validate()` |
| `src/pantry.rs` | Pantry read/query/mutate |
| `src/report.rs` | `RenderRequest`, `render()` |

**Modified in `cookcli`:** `Cargo.toml` (workspace + core dep), `src/main.rs`, `src/lib.rs`, and each command module reduced to a shell.

---

## Task 1: Workspace scaffold and core types

**Files:**
- Modify: `Cargo.toml` (root)
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `crates/core/src/error.rs`
- Create: `crates/core/src/diagnostic.rs`
- Create: `crates/core/src/outcome.rs`
- Create: `crates/core/src/source.rs`

- [ ] **Step 1: Add the workspace table to the root `Cargo.toml`**

Insert this **above** the existing `[package]` section in `/Users/alexeydubovskoy/Cooklang/CookCLI/Cargo.toml`:

```toml
[workspace]
members = [".", "crates/core"]
```

The root package stays exactly where it is. This is deliberate: `include`, `build.rs`, `templates/`, `static/`, `seed/`, `locales/`, CI, release-please and the Homebrew formula all resolve against the root.

- [ ] **Step 2: Create `crates/core/Cargo.toml`**

```toml
[package]
name = "cookcli-core"
version = "0.1.0"
edition = "2021"
description = "Recipe, shopping list, pantry and report operations for Cooklang, extracted from CookCLI"
license = "MIT"
repository = "https://github.com/cooklang/cookcli"
homepage = "https://cooklang.org"
keywords = ["cooklang", "recipes", "cooking"]
categories = ["parser-implementations"]

[dependencies]
camino = { version = "1", features = ["serde1"] }
cooklang = { version = "0.18.5", default-features = false, features = ["aisle", "bundled_units", "pantry", "shopping_list"] }
cooklang-find = { version = "0.6.1", default-features = false }
cooklang-reports = "0.5.1"
directories = "6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
tabular = { version = "0.2", features = ["ansi-cell"] }
thiserror = "2"
tracing = "0.1"
yansi = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Write the failing test for `Diagnostic` and `Outcome`**

Create `crates/core/src/outcome.rs`:

```rust
//! The result wrapper every command returns.

use crate::diagnostic::Diagnostic;

/// A command result paired with any non-fatal diagnostics produced along the way.
///
/// `cooklang` parses leniently: a recipe can parse successfully and still carry
/// warnings. Consumers that do not care about diagnostics ignore the field.
#[derive(Debug, Clone)]
pub struct Outcome<T> {
    pub value: T,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> Outcome<T> {
    /// Wrap a value with no diagnostics.
    pub fn new(value: T) -> Self {
        Self {
            value,
            diagnostics: Vec::new(),
        }
    }

    /// Wrap a value together with diagnostics.
    pub fn with_diagnostics(value: T, diagnostics: Vec<Diagnostic>) -> Self {
        Self { value, diagnostics }
    }

    /// Discard diagnostics and take the value.
    pub fn into_value(self) -> T {
        self.value
    }

    /// True when any diagnostic has `Severity::Error`.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == crate::diagnostic::Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Diagnostic, Severity};

    #[test]
    fn new_has_no_diagnostics() {
        let outcome = Outcome::new(42);
        assert_eq!(outcome.value, 42);
        assert!(outcome.diagnostics.is_empty());
        assert!(!outcome.has_errors());
    }

    #[test]
    fn has_errors_detects_error_severity() {
        let outcome = Outcome::with_diagnostics(
            (),
            vec![Diagnostic {
                severity: Severity::Warning,
                message: "just a warning".to_string(),
                location: None,
            }],
        );
        assert!(!outcome.has_errors());

        let outcome = Outcome::with_diagnostics(
            (),
            vec![Diagnostic {
                severity: Severity::Error,
                message: "a real error".to_string(),
                location: None,
            }],
        );
        assert!(outcome.has_errors());
    }

    #[test]
    fn into_value_discards_diagnostics() {
        let outcome = Outcome::with_diagnostics(
            "hello",
            vec![Diagnostic {
                severity: Severity::Hint,
                message: "hint".to_string(),
                location: None,
            }],
        );
        assert_eq!(outcome.into_value(), "hello");
    }
}
```

- [ ] **Step 4: Create `crates/core/src/diagnostic.rs`**

```rust
//! Structured diagnostics shared by every command.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

/// Where in a source file a diagnostic applies.
///
/// `span` is a byte range into the file content, matching `cooklang`'s
/// source-span convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub file: Option<Utf8PathBuf>,
    pub span: Option<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

impl Diagnostic {
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            location: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            location: None,
        }
    }

    /// Attach a source file to this diagnostic.
    pub fn at_file(mut self, file: impl Into<Utf8PathBuf>) -> Self {
        let location = self.location.get_or_insert(Location {
            file: None,
            span: None,
        });
        location.file = Some(file.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_constructor_sets_severity() {
        let d = Diagnostic::warning("bad aisle line");
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.message, "bad aisle line");
        assert!(d.location.is_none());
    }

    #[test]
    fn at_file_attaches_location() {
        let d = Diagnostic::error("boom").at_file("config/aisle.conf");
        let location = d.location.expect("location set");
        assert_eq!(
            location.file.as_deref().map(|p| p.as_str()),
            Some("config/aisle.conf")
        );
    }

    #[test]
    fn serializes_without_null_location() {
        let d = Diagnostic::warning("no location");
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(
            json,
            r#"{"severity":"warning","message":"no location"}"#
        );
    }
}
```

- [ ] **Step 5: Create `crates/core/src/error.rs`**

```rust
//! The public error type.

use crate::diagnostic::Diagnostic;
use camino::Utf8PathBuf;

/// Errors returned by `cookcli-core` commands.
///
/// `#[non_exhaustive]` so that adding variants stays non-breaking for
/// downstream consumers.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("recipe not found: {name}")]
    RecipeNotFound { name: String },

    /// A recipe failed to parse. `rendered` is `cooklang`'s own report output
    /// with source line context, which the CLI prints verbatim.
    #[error("failed to parse recipe '{name}'\n{rendered}")]
    Parse {
        name: String,
        diagnostics: Vec<Diagnostic>,
        rendered: String,
    },

    #[error("invalid configuration at {path}: {message}")]
    Config {
        path: Utf8PathBuf,
        message: String,
    },

    #[error("template rendering failed: {message}")]
    Render { message: String },

    #[error("circular recipe reference: {chain}")]
    CircularReference { chain: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

- [ ] **Step 6: Create `crates/core/src/source.rs`**

```rust
//! Input sources. Core never touches the filesystem unless handed a path.

use camino::Utf8PathBuf;

/// Where a recipe comes from.
///
/// `Content` exists so editors can pass an unsaved buffer straight in — the
/// case a path-only API cannot serve.
#[derive(Debug, Clone)]
pub enum RecipeSource {
    /// A path or bare recipe name, resolved through `cooklang-find`.
    Path(Utf8PathBuf),
    /// In-memory recipe text. `name` is used in diagnostics and titles.
    Content { text: String, name: String },
}

/// Where an aisle or pantry configuration comes from.
#[derive(Debug, Clone, Default)]
pub enum ConfigSource {
    Path(Utf8PathBuf),
    Inline(String),
    #[default]
    None,
}

impl ConfigSource {
    /// Read the configuration text, if any.
    ///
    /// Returns `Ok(None)` for `ConfigSource::None`.
    pub fn read(&self) -> Result<Option<String>, crate::CoreError> {
        match self {
            ConfigSource::None => Ok(None),
            ConfigSource::Inline(text) => Ok(Some(text.clone())),
            ConfigSource::Path(path) => {
                let text = std::fs::read_to_string(path)?;
                Ok(Some(text))
            }
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, ConfigSource::None)
    }

    /// The path this source reads from, when it is path-backed.
    pub fn path(&self) -> Option<&Utf8PathBuf> {
        match self {
            ConfigSource::Path(p) => Some(p),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_reads_as_none() {
        assert!(ConfigSource::None.read().unwrap().is_none());
        assert!(ConfigSource::None.is_none());
    }

    #[test]
    fn inline_reads_its_text() {
        let source = ConfigSource::Inline("[produce]\ntomato".to_string());
        assert_eq!(
            source.read().unwrap().as_deref(),
            Some("[produce]\ntomato")
        );
    }

    #[test]
    fn path_reads_from_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("aisle.conf");
        std::fs::write(&path, "[dairy]\nmilk").unwrap();
        let utf8 = camino::Utf8PathBuf::from_path_buf(path).unwrap();

        let source = ConfigSource::Path(utf8);
        assert_eq!(source.read().unwrap().as_deref(), Some("[dairy]\nmilk"));
    }

    #[test]
    fn missing_path_is_an_io_error() {
        let source = ConfigSource::Path(camino::Utf8PathBuf::from("/nonexistent/aisle.conf"));
        assert!(matches!(source.read(), Err(crate::CoreError::Io(_))));
    }
}
```

- [ ] **Step 7: Create `crates/core/src/lib.rs`**

```rust
//! Recipe, shopping list, pantry and report operations for Cooklang.
//!
//! This crate holds the logic behind CookCLI's commands, with the CLI reduced
//! to argument parsing and output formatting on top of it.

pub mod diagnostic;
pub mod error;
pub mod outcome;
pub mod source;

pub use diagnostic::{Diagnostic, Location, Severity};
pub use error::CoreError;
pub use outcome::Outcome;
pub use source::{ConfigSource, RecipeSource};

/// Convenience alias for core results.
pub type Result<T> = std::result::Result<T, CoreError>;
```

- [ ] **Step 8: Run the core tests and verify they pass**

```bash
cargo test -p cookcli-core
```

Expected: all tests in `outcome`, `diagnostic` and `source` pass. Roughly 11 tests.

- [ ] **Step 9: Verify the workspace still builds and the CLI suite is untouched**

```bash
cargo build && cargo test 2>&1 | tail -20
```

Expected: `passed=296 failed=0 ignored=26` (Task 6 has not run yet). `cookcli` does not depend on `cookcli-core` yet, so nothing about its behaviour can have changed.

- [ ] **Step 10: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add Cargo.toml Cargo.lock crates/core
git commit -m "feat(core): add cookcli-core crate with error, diagnostic and source types"
```

---

## Task 2: Context and configuration discovery

**Files:**
- Create: `crates/core/src/context.rs`
- Modify: `crates/core/src/lib.rs`

This reconciles the two divergent `Context` copies. `src/lib.rs`'s `aisle()` checks only `./config/aisle.conf`; `src/main.rs`'s checks that **and** falls back to the platform config directory. The main.rs behaviour is correct and is what `discover()` implements.

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/context.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConfigSource;

    #[test]
    fn new_touches_nothing() {
        let ctx = Context::new(Utf8PathBuf::from("/nonexistent"));
        assert!(ctx.aisle().is_none());
        assert!(ctx.pantry().is_none());
        assert_eq!(ctx.base_path(), "/nonexistent");
    }

    #[test]
    fn with_aisle_overrides() {
        let ctx = Context::new(Utf8PathBuf::from("/tmp"))
            .with_aisle(ConfigSource::Inline("[produce]\nleek".to_string()));
        assert_eq!(
            ctx.aisle().read().unwrap().as_deref(),
            Some("[produce]\nleek")
        );
    }

    #[test]
    fn discover_finds_local_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let config_dir = dir.path().join("config");
        std::fs::create_dir(&config_dir).unwrap();
        std::fs::write(config_dir.join("aisle.conf"), "[produce]\nleek").unwrap();

        let base = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let ctx = Context::discover(base.clone());

        let aisle_path = ctx.aisle().path().expect("local aisle found").clone();
        assert_eq!(aisle_path, base.join("config").join("aisle.conf"));
    }

    #[test]
    fn discover_leaves_pantry_none_when_absent() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let ctx = Context::discover(base);

        // No local pantry.conf. The global one may or may not exist on this
        // machine, so assert only that discovery did not invent a local path.
        assert!(ctx
            .pantry()
            .path()
            .map(|p| !p.as_str().contains("config/pantry.conf"))
            .unwrap_or(true));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p cookcli-core context
```

Expected: FAIL to compile — `Context` is not defined.

- [ ] **Step 3: Implement `Context`**

Prepend to `crates/core/src/context.rs`:

```rust
//! Resolved configuration for a set of recipe operations.

use crate::{ConfigSource, CoreError};
use camino::{Utf8Path, Utf8PathBuf};

const APP_NAME: &str = "cook";
const LOCAL_CONFIG_DIR: &str = "config";
const AUTO_AISLE: &str = "aisle.conf";
const AUTO_PANTRY: &str = "pantry.conf";

/// The configuration bundle every command operates against.
///
/// `Context::new` performs no filesystem access. Ambient configuration
/// discovery is opt-in through `Context::discover`.
#[derive(Debug, Clone)]
pub struct Context {
    base_path: Utf8PathBuf,
    aisle: ConfigSource,
    pantry: ConfigSource,
}

impl Context {
    /// A context with no aisle or pantry configuration. Touches nothing.
    pub fn new(base_path: Utf8PathBuf) -> Self {
        Self {
            base_path,
            aisle: ConfigSource::None,
            pantry: ConfigSource::None,
        }
    }

    /// A context with aisle and pantry resolved using CookCLI's search order:
    /// `<base>/config/<name>`, then the platform config directory
    /// (`~/.config/cook/<name>` on Linux, the equivalent elsewhere).
    ///
    /// This is the only constructor that reads ambient state, and it is
    /// explicitly opted into.
    pub fn discover(base_path: Utf8PathBuf) -> Self {
        let aisle = Self::discover_one(&base_path, AUTO_AISLE);
        let pantry = Self::discover_one(&base_path, AUTO_PANTRY);
        Self {
            base_path,
            aisle,
            pantry,
        }
    }

    fn discover_one(base_path: &Utf8Path, name: &str) -> ConfigSource {
        let local = base_path.join(LOCAL_CONFIG_DIR).join(name);
        tracing::trace!("checking local config file: {local}");
        if local.is_file() {
            return ConfigSource::Path(local);
        }

        match global_file_path(name) {
            Ok(global) => {
                tracing::trace!("checking global config file: {global}");
                if global.is_file() {
                    ConfigSource::Path(global)
                } else {
                    ConfigSource::None
                }
            }
            Err(_) => ConfigSource::None,
        }
    }

    pub fn with_aisle(mut self, source: ConfigSource) -> Self {
        self.aisle = source;
        self
    }

    pub fn with_pantry(mut self, source: ConfigSource) -> Self {
        self.pantry = source;
        self
    }

    pub fn base_path(&self) -> &Utf8PathBuf {
        &self.base_path
    }

    pub fn aisle(&self) -> &ConfigSource {
        &self.aisle
    }

    pub fn pantry(&self) -> &ConfigSource {
        &self.pantry
    }
}

/// Resolve a global configuration file path (e.g. `~/.config/cook/{name}`).
pub fn global_file_path(name: &str) -> Result<Utf8PathBuf, CoreError> {
    let dirs = directories::ProjectDirs::from("", "", APP_NAME).ok_or_else(|| {
        CoreError::Config {
            path: Utf8PathBuf::from(name),
            message: "could not determine home directory path".to_string(),
        }
    })?;
    let config = Utf8Path::from_path(dirs.config_dir()).ok_or_else(|| CoreError::Config {
        path: Utf8PathBuf::from(name),
        message: "cook only supports UTF-8 paths".to_string(),
    })?;
    Ok(config.join(name))
}
```

- [ ] **Step 4: Export it from `lib.rs`**

Add to `crates/core/src/lib.rs`:

```rust
pub mod context;
pub use context::{global_file_path, Context};
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo test -p cookcli-core
```

Expected: PASS, including the four new `context` tests.

- [ ] **Step 6: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add crates/core
git commit -m "feat(core): add Context with opt-in configuration discovery"
```

---

## Task 3: Parser and diagnostic conversion

**Files:**
- Create: `crates/core/src/parser.rs`
- Modify: `crates/core/src/lib.rs`

This is the bridge between `cooklang`'s lenient parse report and core's `Diagnostic`. Today `src/util/mod.rs:59` (`parse_recipe_from_entry`) logs warnings through `warn!` and drops them. Core returns them.

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/parser.rs` with its test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Severity;

    const GOOD: &str = "Boil @water{2%cups} for ~{5%minutes}.\n";

    #[test]
    fn parses_a_clean_recipe_without_diagnostics() {
        let outcome = parse_recipe(GOOD, "simple", 1.0).expect("parses");
        assert_eq!(outcome.value.ingredients.len(), 1);
        assert!(outcome.diagnostics.is_empty());
    }

    #[test]
    fn scaling_multiplies_quantities() {
        let single = parse_recipe(GOOD, "simple", 1.0).unwrap().into_value();
        let double = parse_recipe(GOOD, "simple", 2.0).unwrap().into_value();

        let one = format!("{:?}", single.ingredients[0].quantity);
        let two = format!("{:?}", double.ingredients[0].quantity);
        assert_ne!(one, two, "scaling should change the quantity");
    }

    #[test]
    fn warnings_are_returned_not_swallowed() {
        // Deprecated `>>` metadata parses successfully but warns.
        let text = ">> title: Old Style\n\nBoil @water{}.\n";
        let outcome = parse_recipe(text, "old", 1.0).expect("parses despite warning");
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Warning),
            "expected a warning diagnostic, got {:?}",
            outcome.diagnostics
        );
    }

    #[test]
    fn parse_errors_carry_diagnostics_and_rendered_output() {
        // An unclosed component is a hard parse error.
        let text = "Add @salt{1%tsp to the pot.\n";
        match parse_recipe(text, "broken", 1.0) {
            Err(CoreError::Parse {
                name,
                diagnostics,
                rendered,
            }) => {
                assert_eq!(name, "broken");
                assert!(!diagnostics.is_empty());
                assert!(!rendered.is_empty(), "rendered report should be populated");
            }
            other => panic!("expected CoreError::Parse, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p cookcli-core parser
```

Expected: FAIL to compile — `parse_recipe` is not defined.

- [ ] **Step 3: Implement the parser module**

Prepend to `crates/core/src/parser.rs`:

```rust
//! Recipe parsing and conversion of `cooklang` reports into `Diagnostic`s.

use crate::{CoreError, Diagnostic, Location, Outcome, Severity};
use camino::Utf8PathBuf;
use cooklang::{Converter, CooklangParser, Extensions, Recipe};
use std::sync::LazyLock;

/// The shared parser. Matches CookCLI's configuration exactly: no extensions,
/// default converter for unit support.
pub static PARSER: LazyLock<CooklangParser> =
    LazyLock::new(|| CooklangParser::new(Extensions::empty(), Converter::default()));

/// Parse recipe text, scale it, and collect diagnostics.
///
/// `name` is used for diagnostics and for the `CoreError::Parse` message.
pub fn parse_recipe(text: &str, name: &str, scale: f64) -> Result<Outcome<Recipe>, CoreError> {
    parse_recipe_at(text, name, scale, None)
}

/// As `parse_recipe`, but attributing diagnostics to a specific file path.
pub fn parse_recipe_at(
    text: &str,
    name: &str,
    scale: f64,
    file: Option<&Utf8PathBuf>,
) -> Result<Outcome<Recipe>, CoreError> {
    let parsed = PARSER.parse(text);
    let report = parsed.report();

    let diagnostics = collect_diagnostics(&parsed, file);

    if report.has_errors() {
        let display_path = file
            .map(|p| p.to_string())
            .unwrap_or_else(|| name.to_string());
        let mut buf = Vec::new();
        report.write(&display_path, text, false, &mut buf).ok();
        return Err(CoreError::Parse {
            name: name.to_string(),
            diagnostics,
            rendered: String::from_utf8_lossy(&buf).into_owned(),
        });
    }

    let (mut recipe, _) = parsed
        .into_result()
        .expect("report has no errors, so a recipe is present");
    recipe.scale(scale, PARSER.converter());

    Ok(Outcome::with_diagnostics(recipe, diagnostics))
}

/// Render a parse report the way the CLI prints it, with source line context.
///
/// `ansi` controls colour. The CLI passes `true` for terminal output.
pub fn render_report(
    parsed: &cooklang::PassResult<Recipe>,
    display_path: &str,
    content: &str,
    ansi: bool,
) -> String {
    let mut buf = Vec::new();
    parsed
        .report()
        .write(display_path, content, ansi, &mut buf)
        .ok();
    String::from_utf8_lossy(&buf).into_owned()
}

fn collect_diagnostics(
    parsed: &cooklang::PassResult<Recipe>,
    file: Option<&Utf8PathBuf>,
) -> Vec<Diagnostic> {
    let report = parsed.report();
    let mut out = Vec::new();

    for error in report.errors() {
        out.push(Diagnostic {
            severity: Severity::Error,
            message: error.to_string(),
            location: file.map(|f| Location {
                file: Some(f.clone()),
                span: None,
            }),
        });
    }

    for warning in report.warnings() {
        out.push(Diagnostic {
            severity: Severity::Warning,
            message: warning.to_string(),
            location: file.map(|f| Location {
                file: Some(f.clone()),
                span: None,
            }),
        });
    }

    out
}
```

**Note on the `PassResult` type:** `cooklang`'s parse result type name and its `report()`/`into_result()` methods are used verbatim from `src/util/mod.rs:59-95`. If the concrete type path differs, mirror exactly what `src/util/mod.rs` does — that file compiles today against this same `cooklang` version.

- [ ] **Step 4: Export from `lib.rs`**

Add to `crates/core/src/lib.rs`:

```rust
pub mod parser;
pub use parser::{parse_recipe, parse_recipe_at, PARSER};
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo test -p cookcli-core parser
```

Expected: PASS, four tests.

If `warnings_are_returned_not_swallowed` fails because `>>` metadata does not warn in this `cooklang` version, find a construct that does warn by checking what `tests/common/mod.rs`'s `with_errors.cook` fixture triggers, and use that instead. Do not delete the test — the point is proving warnings survive.

- [ ] **Step 6: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add crates/core
git commit -m "feat(core): add recipe parsing with diagnostic collection"
```

---

## Task 4: Move the formatters

**Files:**
- Create: `crates/core/src/format/mod.rs`
- Create: `crates/core/src/format/{human,markdown,cooklang,latex,typst,schema}.rs`
- Delete: `src/util/cooklang_to_{human,md,cooklang,latex,typst,schema}.rs`
- Modify: `src/util/mod.rs`, `src/recipe/read.rs`, root `Cargo.toml`

The formatters are pure `&Recipe -> impl Write`. The only coupling to break is `cooklang_to_human.rs`'s use of `yansi::Paint`, which reads a global colour setting. Core replaces that with an explicit `Style` parameter.

- [ ] **Step 1: Add the core dependency to `cookcli`**

In the root `Cargo.toml` `[dependencies]`:

```toml
cookcli-core = { version = "0.1.0", path = "crates/core" }
```

Both `version` and `path`: local builds use the workspace copy, and the published crate resolves from crates.io.

- [ ] **Step 2: Create `crates/core/src/format/mod.rs`**

```rust
//! Recipe output formatters.
//!
//! Every formatter writes into an `io::Write`. `*_to_string` convenience
//! wrappers are provided for consumers that just want a `String`.

pub mod cooklang;
pub mod human;
pub mod latex;
pub mod markdown;
pub mod schema;
pub mod typst;

/// Whether formatters emit ANSI escape codes.
///
/// A library must not emit escape sequences by default, and `yansi`'s global
/// enable/disable is not acceptable shared mutable state, so colour is passed
/// explicitly. The CLI passes `Ansi`; consumers get `Plain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Style {
    #[default]
    Plain,
    Ansi,
}

impl Style {
    pub fn is_ansi(self) -> bool {
        matches!(self, Style::Ansi)
    }
}

/// Paper size for the print-oriented formatters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperSize {
    A4,
    Letter,
}

impl PaperSize {
    pub fn latex_name(self) -> &'static str {
        match self {
            PaperSize::A4 => "a4paper",
            PaperSize::Letter => "letterpaper",
        }
    }

    pub fn typst_name(self) -> &'static str {
        match self {
            PaperSize::A4 => "a4",
            PaperSize::Letter => "us-letter",
        }
    }
}
```

**Before writing `latex_name`/`typst_name`, read the existing definitions** in `src/recipe/read.rs` (the `PaperSize` enum and its two methods) and copy the exact string values. The values above are the expected ones, but the snapshots pin the real output — use what is in the file.

- [ ] **Step 3: Move each formatter file**

```bash
git mv src/util/cooklang_to_human.rs crates/core/src/format/human.rs
git mv src/util/cooklang_to_md.rs crates/core/src/format/markdown.rs
git mv src/util/cooklang_to_cooklang.rs crates/core/src/format/cooklang.rs
git mv src/util/cooklang_to_latex.rs crates/core/src/format/latex.rs
git mv src/util/cooklang_to_typst.rs crates/core/src/format/typst.rs
git mv src/util/cooklang_to_schema.rs crates/core/src/format/schema.rs
git mv src/util/format.rs crates/core/src/format/number.rs
```

Using `git mv` keeps file history, which matters for a 2,800-line move.

- [ ] **Step 4: Fix the imports in each moved file**

In every moved file, replace `crate::util::format::` with `crate::format::number::` and `crate::util::PARSER` with `crate::parser::PARSER`. Change `anyhow::Result` to `std::io::Result` or `crate::Result` as the signature requires — `markdown.rs` imports `anyhow::{Context, Result}` at line 35 and must lose it, since core does not depend on `anyhow`.

Add `pub mod number;` to `crates/core/src/format/mod.rs`.

- [ ] **Step 5: Thread `Style` through the human formatter**

In `crates/core/src/format/human.rs`, change the signature at what was line 132:

```rust
pub fn print_human(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
    style: Style,
    writer: &mut impl std::io::Write,
) -> std::io::Result<()> {
```

Inside, guard every `yansi::Paint` call on `style.is_ansi()`. The mechanical way to do this without touching every call site: at the top of the function,

```rust
let _colour_guard = if style.is_ansi() {
    yansi::enable();
} else {
    yansi::disable();
};
```

is **not** acceptable — it mutates global state. Instead, wrap the writer:

```rust
let mut plain;
let mut coloured;
let writer: &mut dyn std::io::Write = if style.is_ansi() {
    coloured = writer;
    &mut coloured
} else {
    plain = anstream::StripStream::new(writer);
    &mut plain
};
```

This requires adding `anstream = "0.6"` to `crates/core/Cargo.toml`. Stripping at the writer is exactly what `src/util/mod.rs:98` (`write_to_output`) already does for file output, so the behaviour is already proven in this codebase.

- [ ] **Step 6: Add `_to_string` wrappers to `format/mod.rs`**

```rust
use cooklang::{Converter, Recipe};

/// Render a recipe as human-readable text.
pub fn human_to_string(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
    style: Style,
) -> Result<String, std::io::Error> {
    let mut buf = Vec::new();
    human::print_human(recipe, name, scale, converter, style, &mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Render a recipe as Markdown.
pub fn markdown_to_string(
    recipe: &Recipe,
    name: &str,
    scale: f64,
    converter: &Converter,
) -> Result<String, std::io::Error> {
    let mut buf = Vec::new();
    markdown::print_md(recipe, name, scale, converter, &mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
```

- [ ] **Step 7: Point `cookcli` at the moved formatters**

In `src/util/mod.rs`, delete the seven `pub mod cooklang_to_*;` / `pub mod format;` declarations and replace with re-exports so existing call sites keep compiling:

```rust
pub use cookcli_core::format;
pub use cookcli_core::parser::PARSER;
```

In `src/recipe/read.rs`, change each formatter call to the core path and pass `Style::Ansi` to the human formatter:

```rust
OutputFormat::Human => cookcli_core::format::human::print_human(
    &recipe,
    &title,
    scale,
    PARSER.converter(),
    cookcli_core::format::Style::Ansi,
    writer,
)?,
```

- [ ] **Step 8: Build and run the full suite**

```bash
cargo build && cargo test 2>&1 | tail -30
```

Expected: `passed=296 failed=0 ignored=26` (Task 6 has not run yet). The snapshot tests
`test_recipe_human_output`, `test_recipe_markdown_output`, `test_recipe_json_output`,
`test_recipe_yaml_output`, `test_scaled_recipe_output` and
`test_recipe_with_references_output` are the ones that would catch a formatting
regression here. If any of them fails, the `Style` threading changed the output —
fix the code. **Do not accept the snapshot.**

- [ ] **Step 9: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A
git commit -m "refactor(core): move recipe formatters into cookcli-core"
```

---

## Task 5: recipe::read

**Files:**
- Create: `crates/core/src/recipe.rs`
- Modify: `crates/core/src/lib.rs`, `src/recipe/read.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/recipe.rs` with its test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Context, RecipeSource};

    fn fixture_dir() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("simple.cook"),
            "Boil @water{2%cups} for ~{5%minutes}.\nAdd @salt{1%tsp}.\n",
        )
        .unwrap();
        dir
    }

    fn ctx_for(dir: &tempfile::TempDir) -> Context {
        Context::new(camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap())
    }

    #[test]
    fn reads_a_recipe_from_a_path() {
        let dir = fixture_dir();
        let ctx = ctx_for(&dir);

        let outcome = read(
            &ctx,
            ReadRequest {
                source: RecipeSource::Path("simple.cook".into()),
                scale: 1.0,
            },
        )
        .expect("reads");

        assert_eq!(outcome.value.recipe.ingredients.len(), 2);
        assert_eq!(outcome.value.title, "simple");
    }

    #[test]
    fn reads_a_recipe_from_memory() {
        let ctx = Context::new(camino::Utf8PathBuf::from("/nonexistent"));

        let outcome = read(
            &ctx,
            ReadRequest {
                source: RecipeSource::Content {
                    text: "Boil @water{1%cup}.\n".to_string(),
                    name: "buffer".to_string(),
                },
                scale: 1.0,
            },
        )
        .expect("reads from memory without touching the filesystem");

        assert_eq!(outcome.value.recipe.ingredients.len(), 1);
        assert_eq!(outcome.value.title, "buffer");
    }

    #[test]
    fn missing_recipe_is_recipe_not_found() {
        let dir = fixture_dir();
        let ctx = ctx_for(&dir);

        let err = read(
            &ctx,
            ReadRequest {
                source: RecipeSource::Path("nope.cook".into()),
                scale: 1.0,
            },
        )
        .unwrap_err();

        assert!(matches!(err, CoreError::RecipeNotFound { .. }), "got {err:?}");
    }

    #[test]
    fn scale_is_applied() {
        let dir = fixture_dir();
        let ctx = ctx_for(&dir);

        let single = read(
            &ctx,
            ReadRequest {
                source: RecipeSource::Path("simple.cook".into()),
                scale: 1.0,
            },
        )
        .unwrap();
        let double = read(
            &ctx,
            ReadRequest {
                source: RecipeSource::Path("simple.cook".into()),
                scale: 2.0,
            },
        )
        .unwrap();

        assert_ne!(
            format!("{:?}", single.value.recipe.ingredients[0].quantity),
            format!("{:?}", double.value.recipe.ingredients[0].quantity)
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p cookcli-core recipe
```

Expected: FAIL to compile — `read` and `ReadRequest` are not defined.

- [ ] **Step 3: Implement `recipe::read`**

Prepend to `crates/core/src/recipe.rs`:

```rust
//! Reading and scaling a single recipe.

use crate::{Context, CoreError, Outcome, RecipeSource};
use cooklang::Recipe;

#[derive(Debug, Clone)]
pub struct ReadRequest {
    pub source: RecipeSource,
    /// Scaling factor applied to all quantities.
    pub scale: f64,
}

/// A parsed recipe together with the title to display for it.
#[derive(Debug, Clone)]
pub struct ReadResult {
    pub recipe: Recipe,
    pub title: String,
}

/// Split a `name:factor` query into its parts.
///
/// `"pasta.cook:2"` becomes `("pasta.cook", 2.0)`.
pub fn split_name_and_scale(query: &str) -> Option<(&str, f64)> {
    let (name, factor) = query.trim().rsplit_once(':')?;
    let factor = factor.parse::<f64>().ok()?;
    Some((name, factor))
}

pub fn read(ctx: &Context, req: ReadRequest) -> Result<Outcome<ReadResult>, CoreError> {
    match req.source {
        RecipeSource::Content { text, name } => {
            let parsed = crate::parser::parse_recipe(&text, &name, req.scale)?;
            Ok(Outcome::with_diagnostics(
                ReadResult {
                    recipe: parsed.value,
                    title: name,
                },
                parsed.diagnostics,
            ))
        }
        RecipeSource::Path(path) => {
            let query = path.as_str();
            let (name, inline_scale) = split_name_and_scale(query)
                .map(|(n, f)| (n, Some(f)))
                .unwrap_or((query, None));
            let scale = inline_scale.unwrap_or(req.scale);

            let entry = cooklang_find::get_recipe(vec![ctx.base_path().clone()], name.into())
                .map_err(|_| CoreError::RecipeNotFound {
                    name: name.to_string(),
                })?;

            let content = entry.content().map_err(|_| CoreError::RecipeNotFound {
                name: name.to_string(),
            })?;
            let title = entry.name().clone().unwrap_or_default();
            let file = entry.path().cloned();

            let parsed =
                crate::parser::parse_recipe_at(&content, &title, scale, file.as_ref())?;

            Ok(Outcome::with_diagnostics(
                ReadResult {
                    recipe: parsed.value,
                    title,
                },
                parsed.diagnostics,
            ))
        }
    }
}
```

- [ ] **Step 4: Export from `lib.rs`**

```rust
pub mod recipe;
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo test -p cookcli-core recipe
```

Expected: PASS, four tests.

- [ ] **Step 6: Reduce `src/recipe/read.rs` to a shell**

Replace the recipe-loading block at `src/recipe/read.rs:138-167` with a core call, keeping everything from `let format = ...` onward untouched:

```rust
    let source = match args.input.recipe {
        Some(query) => cookcli_core::RecipeSource::Path(query),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("Failed to read stdin")?;
            cookcli_core::RecipeSource::Content {
                text: buf,
                name: "stdin".to_string(),
            }
        }
    };

    let outcome = cookcli_core::recipe::read(
        &ctx.to_core(),
        cookcli_core::recipe::ReadRequest {
            source,
            scale: args.input.scale,
        },
    )?;

    for diagnostic in &outcome.diagnostics {
        tracing::warn!("{}", diagnostic.message);
    }

    let recipe = outcome.value.recipe;
    let title = outcome.value.title;
    let scale = args.input.scale;
```

Logging diagnostics through `tracing::warn!` preserves today's behaviour exactly: `src/util/mod.rs:63-69` already `warn!`s each parse warning, and warnings go to stderr, which the snapshots do not capture.

- [ ] **Step 7: Add `Context::to_core` to `cookcli`**

In both `src/main.rs` and `src/lib.rs`, add to `impl Context`:

```rust
    /// Build the core context, carrying over resolved aisle and pantry paths.
    pub fn to_core(&self) -> cookcli_core::Context {
        let mut core = cookcli_core::Context::new(self.base_path.clone());
        if let Some(aisle) = self.aisle() {
            core = core.with_aisle(cookcli_core::ConfigSource::Path(aisle));
        }
        if let Some(pantry) = self.pantry() {
            core = core.with_pantry(cookcli_core::ConfigSource::Path(pantry));
        }
        core
    }
```

- [ ] **Step 8: Run the full suite**

```bash
cargo test 2>&1 | tail -30
```

Expected: `passed=296 failed=0 ignored=26` (Task 6 has not run yet). `tests/recipe_test.rs`, `tests/output_formats_test.rs`, `tests/schema_output_test.rs` and the recipe snapshots are the relevant guards.

One behaviour to check by hand, because the tests may not cover it:

```bash
echo 'Boil @water{1%cup}.' | cargo run -- recipe
```

Expected: renders with the title `stdin`, exactly as before.

- [ ] **Step 9: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A
git commit -m "refactor(recipe): delegate recipe reading to cookcli-core"
```

---

## Task 6: Characterize shopping-list behaviour before touching it

**Files:**
- Create: `tests/shopping_list_characterization_test.rs`
- Create: `tests/snapshots/shopping_list_characterization_test__*.snap` (generated)

This task adds tests, changes no production code, and exists solely so Task 7 has a contract. Without it you would be refactoring the most intricate command in the plan blind.

These are **characterization tests** (golden master): they record what the command does today, correct or not. If a snapshot here looks wrong to you, that is a finding to report — not something to fix in this plan.

- [ ] **Step 1: Confirm the gap for yourself**

```bash
cargo test --test shopping_list_test -- --ignored 2>&1 | tail -20
```

Expected: `7 passed; 11 failed`. These stale tests are why the coverage hole exists. Do not fix them.

- [ ] **Step 2: Write the characterization test file**

Create `tests/shopping_list_characterization_test.rs`:

```rust
//! Characterization tests for `cook shopping-list`.
//!
//! These snapshots record CURRENT behaviour so the cookcli-core extraction can
//! be verified as behaviour-preserving. They are not assertions that the output
//! is correct — only that it has not changed.
//!
//! See docs/superpowers/plans/2026-08-10-cookcli-core-extraction.md, Task 6.

#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use insta::assert_snapshot;
use std::fs;
use tempfile::TempDir;

/// A recipe set with overlapping ingredients, a reference, units that combine,
/// and units that do not.
fn setup() -> TempDir {
    let dir = TempDir::new().unwrap();

    fs::write(
        dir.path().join("pasta.cook"),
        "Cook @pasta{200%g} in @water{2%l} with @salt{1%tsp}.\n",
    )
    .unwrap();

    fs::write(
        dir.path().join("salad.cook"),
        "Chop @tomato{3} and @cucumber{1}.\nDress with @olive oil{2%tbsp} and @salt{1%tsp}.\n",
    )
    .unwrap();

    fs::write(
        dir.path().join("sauce.cook"),
        "Heat @olive oil{1%tbsp} and add @garlic{2%cloves}.\n",
    )
    .unwrap();

    fs::write(
        dir.path().join("dinner.cook"),
        "Make @./sauce{} and serve with @pasta{100%g}.\n",
    )
    .unwrap();

    let config = dir.path().join("config");
    fs::create_dir(&config).unwrap();
    fs::write(
        config.join("aisle.conf"),
        "[produce]\ntomato\ncucumber\ngarlic\n\n[pantry]\npasta\nsalt\nolive oil\n",
    )
    .unwrap();

    dir
}

fn run(dir: &TempDir, args: &[&str]) -> String {
    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(dir.path())
        .args(args)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "command failed: cook {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn characterize_single_recipe_human() {
    let dir = setup();
    assert_snapshot!(run(&dir, &["shopping-list", "pasta.cook"]));
}

#[test]
fn characterize_multiple_recipes_aggregated() {
    let dir = setup();
    assert_snapshot!(run(&dir, &["shopping-list", "pasta.cook", "salad.cook"]));
}

#[test]
fn characterize_aisle_categorization() {
    let dir = setup();
    // config/aisle.conf is picked up automatically
    assert_snapshot!(run(&dir, &["shopping-list", "salad.cook", "pasta.cook"]));
}

#[test]
fn characterize_plain_output() {
    let dir = setup();
    assert_snapshot!(run(&dir, &["shopping-list", "--plain", "salad.cook"]));
}

#[test]
fn characterize_json_output() {
    let dir = setup();
    assert_snapshot!(run(
        &dir,
        &["shopping-list", "-f", "json", "--pretty", "salad.cook"]
    ));
}

#[test]
fn characterize_yaml_output() {
    let dir = setup();
    assert_snapshot!(run(&dir, &["shopping-list", "-f", "yaml", "salad.cook"]));
}

#[test]
fn characterize_markdown_output() {
    let dir = setup();
    assert_snapshot!(run(&dir, &["shopping-list", "-f", "markdown", "salad.cook"]));
}

#[test]
fn characterize_scaling() {
    let dir = setup();
    assert_snapshot!(run(&dir, &["shopping-list", "pasta.cook:3"]));
}

#[test]
fn characterize_recipe_reference_expansion() {
    let dir = setup();
    assert_snapshot!(run(&dir, &["shopping-list", "dinner.cook"]));
}

#[test]
fn characterize_ignore_references() {
    let dir = setup();
    assert_snapshot!(run(&dir, &["shopping-list", "--ignore-references", "dinner.cook"]));
}

#[test]
fn characterize_ingredients_only() {
    let dir = setup();
    assert_snapshot!(run(&dir, &["shopping-list", "--ingredients-only", "salad.cook"]));
}

#[test]
fn characterize_pantry_subtraction() {
    let dir = setup();
    fs::write(
        dir.path().join("config").join("pantry.conf"),
        "[pantry]\nsalt = \"500%g\"\npasta = \"1%kg\"\n",
    )
    .unwrap();
    assert_snapshot!(run(&dir, &["shopping-list", "pasta.cook"]));
}

#[test]
fn characterize_ignore_pantry() {
    let dir = setup();
    fs::write(
        dir.path().join("config").join("pantry.conf"),
        "[pantry]\nsalt = \"500%g\"\npasta = \"1%kg\"\n",
    )
    .unwrap();
    assert_snapshot!(run(&dir, &["shopping-list", "--ignore-pantry", "pasta.cook"]));
}

#[test]
fn characterize_malformed_aisle_config() {
    let dir = setup();
    fs::write(
        dir.path().join("config").join("aisle.conf"),
        "this is not a valid aisle file [[[\n",
    )
    .unwrap();
    // Records the silent-fallback behaviour that Task 10 changes.
    // Only stdout is captured; the Task 10 fix is stderr-only.
    assert_snapshot!(run(&dir, &["shopping-list", "salad.cook"]));
}
```

- [ ] **Step 3: Verify every characterization test compiles and runs**

```bash
cargo test --test shopping_list_characterization_test 2>&1 | tail -25
```

Expected: 14 tests, all failing with insta's "snapshot missing" — that is the correct first run. If a test fails with a *command* failure instead (non-zero exit), the flag or syntax is wrong for this version; fix the test until the command succeeds.

Two flags to confirm against `src/shopping_list.rs`'s `ShoppingListArgs` before running: `--ingredients-only` and `--ignore-references`. Use `cargo run -- shopping-list --help` to check the real names.

- [ ] **Step 4: Accept the snapshots — this is the one place it is correct**

```bash
cargo insta accept
```

- [ ] **Step 5: Read every generated snapshot**

```bash
ls tests/snapshots/shopping_list_characterization_test__*.snap
cat tests/snapshots/shopping_list_characterization_test__characterize_aisle_categorization.snap
```

Read all 14. You are looking for empty snapshots or output that shows the command silently did nothing — those mean the test is not exercising what its name claims, and it would give false confidence in Task 7. Fix the fixture and re-accept if so.

- [ ] **Step 6: Verify they now pass and the suite total went up**

```bash
cargo test 2>&1 | grep -E "^test result:" | awk '{p+=$4; f+=$6; i+=$8} END {print "passed="p" failed="f" ignored="i}'
```

Expected: `passed=310 failed=0 ignored=26`. From here on, **310** is the number every later task must hold.

- [ ] **Step 7: Commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add tests/shopping_list_characterization_test.rs tests/snapshots/
git commit -m "test(shopping-list): add characterization snapshots before core extraction

All 26 ignored tests in the suite are shopping-list tests, and 11 of them
fail when run, so the command had no regression coverage. These snapshots
record current behaviour so the cookcli-core extraction can be verified as
behaviour-preserving. They assert what the output IS, not what it should be."
```

---

## Task 7: shopping_list::generate

**Files:**
- Create: `crates/core/src/shopping_list/mod.rs`
- Modify: `crates/core/src/lib.rs`, `src/shopping_list.rs`, `src/util/mod.rs`

**Guarded by Task 6's characterization snapshots.** They are the only thing standing between this refactor and a silent regression.

`src/util/mod.rs:155` (`extract_ingredients`) is the recursive aggregation core and moves wholesale. The output builders (`build_human_table`, `build_json_value`, `build_yaml_value`, `build_md_value` in `src/shopping_list.rs:370+`) move too, so output stays byte-identical by construction.

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/shopping_list/mod.rs` with its test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConfigSource, Context};

    fn fixture_dir() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("pasta.cook"),
            "Cook @pasta{200%g} in @water{2%l}.\nAdd @salt{1%tsp}.\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("salad.cook"),
            "Chop @tomato{3} and @salt{1%tsp}.\n",
        )
        .unwrap();
        dir
    }

    fn ctx_for(dir: &tempfile::TempDir) -> Context {
        Context::new(camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap())
    }

    #[test]
    fn aggregates_across_recipes() {
        let dir = fixture_dir();
        let ctx = ctx_for(&dir);

        let outcome = generate(
            &ctx,
            GenerateRequest {
                recipes: vec!["pasta.cook".to_string(), "salad.cook".to_string()],
                ignore_references: false,
                ignore_pantry: false,
            },
        )
        .expect("generates");

        let names: Vec<&str> = outcome
            .value
            .items()
            .map(|item| item.name.as_str())
            .collect();
        // salt appears in both recipes and must be merged into one entry
        assert_eq!(names.iter().filter(|n| **n == "salt").count(), 1);
    }

    #[test]
    fn inline_aisle_config_categorises() {
        let dir = fixture_dir();
        let ctx = ctx_for(&dir).with_aisle(ConfigSource::Inline(
            "[produce]\ntomato\n\n[pantry]\nsalt\npasta\n".to_string(),
        ));

        let outcome = generate(
            &ctx,
            GenerateRequest {
                recipes: vec!["salad.cook".to_string()],
                ignore_references: false,
                ignore_pantry: false,
            },
        )
        .expect("generates");

        let categories: Vec<&str> = outcome
            .value
            .categories
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(
            categories.contains(&"produce"),
            "expected a produce category, got {categories:?}"
        );
    }

    #[test]
    fn malformed_aisle_config_returns_a_diagnostic() {
        let dir = fixture_dir();
        let ctx = ctx_for(&dir).with_aisle(ConfigSource::Inline(
            "this is not a valid aisle file [[[\n".to_string(),
        ));

        let outcome = generate(
            &ctx,
            GenerateRequest {
                recipes: vec!["salad.cook".to_string()],
                ignore_references: false,
                ignore_pantry: false,
            },
        )
        .expect("still generates a list");

        assert!(
            !outcome.diagnostics.is_empty(),
            "a malformed aisle file must surface a diagnostic rather than being swallowed"
        );
    }

    #[test]
    fn circular_reference_is_reported() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.cook"), "Make @./b{}.\n").unwrap();
        std::fs::write(dir.path().join("b.cook"), "Make @./a{}.\n").unwrap();
        let ctx = Context::new(
            camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap(),
        );

        let err = generate(
            &ctx,
            GenerateRequest {
                recipes: vec!["a.cook".to_string()],
                ignore_references: false,
                ignore_pantry: false,
            },
        )
        .unwrap_err();

        assert!(
            matches!(err, CoreError::CircularReference { .. }),
            "got {err:?}"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p cookcli-core shopping_list
```

Expected: FAIL to compile — `generate` and `GenerateRequest` are not defined.

- [ ] **Step 3: Implement `shopping_list::generate`**

Prepend to `crates/core/src/shopping_list/mod.rs`:

```rust
//! Shopping list generation: recipes in, aggregated and categorised list out.

use crate::{Context, CoreError, Diagnostic, Outcome};
use cooklang::aisle::AisleConf;
use cooklang::ingredient_list::IngredientList;
use cooklang::pantry::PantryConf;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct GenerateRequest {
    /// Recipe names or paths, each optionally suffixed with `:factor`.
    pub recipes: Vec<String>,
    pub ignore_references: bool,
    pub ignore_pantry: bool,
}

/// One line of a shopping list.
#[derive(Debug, Clone, Serialize)]
pub struct ListItem {
    pub name: String,
    /// Pre-formatted quantities, e.g. `["200 g", "2 cups"]`.
    pub quantities: Vec<String>,
}

/// A named aisle group.
#[derive(Debug, Clone, Serialize)]
pub struct Category {
    pub name: String,
    pub items: Vec<ListItem>,
}

/// An aggregated shopping list.
///
/// `list` and `aisle` are crate-private so `cooklang` types stay out of the
/// public API; the serialisable view is `categories` and `plain_items`.
#[derive(Debug, Clone, Serialize)]
pub struct AggregatedList {
    pub categories: Vec<Category>,
    pub plain_items: Vec<ListItem>,
    #[serde(skip)]
    pub(crate) list: IngredientList,
    #[serde(skip)]
    pub(crate) aisle: AisleConf,
}

impl AggregatedList {
    /// Every item, ignoring categorisation.
    pub fn items(&self) -> impl Iterator<Item = &ListItem> {
        self.plain_items.iter()
    }
}

pub fn generate(
    ctx: &Context,
    req: GenerateRequest,
) -> Result<Outcome<AggregatedList>, CoreError> {
    let mut diagnostics = Vec::new();

    let aisle = load_aisle(ctx, &mut diagnostics)?;
    let pantry = if req.ignore_pantry {
        None
    } else {
        load_pantry(ctx, &mut diagnostics)?
    };

    let mut list = IngredientList::new();
    let mut seen = BTreeMap::new();

    for entry in &req.recipes {
        extract_ingredients(
            entry,
            &mut list,
            &mut seen,
            ctx.base_path(),
            crate::parser::PARSER.converter(),
            req.ignore_references,
            None,
            &mut diagnostics,
        )?;
    }

    list = list.use_common_names(&aisle, crate::parser::PARSER.converter());
    if let Some(pantry_conf) = &pantry {
        list = list.subtract_pantry(pantry_conf, crate::parser::PARSER.converter());
    }

    let aggregated = build_aggregated(list, aisle);
    Ok(Outcome::with_diagnostics(aggregated, diagnostics))
}

fn load_aisle(ctx: &Context, diagnostics: &mut Vec<Diagnostic>) -> Result<AisleConf, CoreError> {
    let Some(content) = ctx.aisle().read()? else {
        diagnostics.push(Diagnostic::warning(
            "No aisle file found. Docs https://cooklang.org/docs/spec/#shopping-lists",
        ));
        return Ok(AisleConf::default());
    };

    let result = cooklang::aisle::parse_lenient(&content);
    for warning in result.report().warnings() {
        let mut d = Diagnostic::warning(format!("Aisle configuration warning: {warning}"));
        if let Some(path) = ctx.aisle().path() {
            d = d.at_file(path.clone());
        }
        diagnostics.push(d);
    }

    match result.output().cloned() {
        Some(conf) => Ok(conf),
        None => {
            let mut d = Diagnostic::warning(
                "Aisle file parsing failed, using default configuration",
            );
            if let Some(path) = ctx.aisle().path() {
                d = d.at_file(path.clone());
            }
            diagnostics.push(d);
            Ok(AisleConf::default())
        }
    }
}

fn load_pantry(
    ctx: &Context,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Option<PantryConf>, CoreError> {
    let Some(content) = ctx.pantry().read()? else {
        return Ok(None);
    };

    let result = cooklang::pantry::parse_lenient(&content);
    for warning in result.report().warnings() {
        let mut d = Diagnostic::warning(format!("Pantry configuration warning: {warning}"));
        if let Some(path) = ctx.pantry().path() {
            d = d.at_file(path.clone());
        }
        diagnostics.push(d);
    }

    match result.output().cloned() {
        Some(mut conf) => {
            conf.rebuild_index();
            Ok(Some(conf))
        }
        None => {
            diagnostics.push(Diagnostic::warning("Failed to parse pantry file"));
            Ok(None)
        }
    }
}

fn build_aggregated(list: IngredientList, aisle: AisleConf) -> AggregatedList {
    let plain_items: Vec<ListItem> = list
        .clone()
        .into_iter()
        .map(|(name, quantity)| ListItem {
            name,
            quantities: quantity.iter().map(format_quantity).collect(),
        })
        .collect();

    let categories = list
        .clone()
        .categorize(&aisle)
        .into_iter()
        .map(|(name, items)| Category {
            name,
            items: items
                .into_iter()
                .map(|(item_name, quantity)| ListItem {
                    name: item_name,
                    quantities: quantity.iter().map(format_quantity).collect(),
                })
                .collect(),
        })
        .collect();

    AggregatedList {
        categories,
        plain_items,
        list,
        aisle,
    }
}

fn format_quantity(qty: &cooklang::quantity::Quantity) -> String {
    match qty.unit() {
        Some(unit) => format!("{} {}", qty.value(), unit),
        None => format!("{}", qty.value()),
    }
}
```

**Note:** `format_quantity` is copied verbatim from `src/shopping_list.rs`'s `quantity_fmt`. Keep them byte-identical — the CLI's table output depends on it.

- [ ] **Step 4: Move `extract_ingredients` into the same module**

`git mv` is not possible for part of a file. Cut `extract_ingredients` from `src/util/mod.rs:155` through the end of that function and paste it into `crates/core/src/shopping_list/mod.rs`, making these changes:

- Add a `diagnostics: &mut Vec<Diagnostic>` parameter as the last argument.
- Replace the circular-dependency `anyhow::anyhow!` at the top with:

```rust
    if seen.contains_key(entry) {
        return Err(CoreError::CircularReference {
            chain: format!(
                "{} -> {}",
                seen.keys().cloned().collect::<Vec<_>>().join(" -> "),
                entry
            ),
        });
    }
```

- Replace `get_recipe(base_path, name).with_context(...)` with:

```rust
    let recipe_entry = crate::find::get_recipe(base_path, name)
        .map_err(|_| CoreError::RecipeNotFound { name: name.to_string() })?;
```

- Replace `parse_recipe_from_entry(&recipe_entry, scaling_factor)?` with a `parse_recipe_at` call that appends its diagnostics to the `diagnostics` parameter.

Also move `get_recipe` (`src/util/mod.rs:384`) into a new `crates/core/src/find.rs`, changing its error to `CoreError::RecipeNotFound`, and declare it in `crates/core/src/lib.rs`:

```rust
pub mod find;
```

`split_recipe_name_and_scaling_factor` is already available as `crate::recipe::split_name_and_scale` — use that and delete the duplicate.

- [ ] **Step 5: Run the core tests**

```bash
cargo test -p cookcli-core shopping_list
```

Expected: PASS, four tests. `malformed_aisle_config_returns_a_diagnostic` is the one proving the silent-fallback bug is fixed.

- [ ] **Step 6: Move the output builders into core**

Cut `build_human_table`, `build_json_value`, `build_yaml_value`, `build_md_value`, `total_quantity_fmt` and `quantity_fmt` from `src/shopping_list.rs` into a new `crates/core/src/format/shopping_list.rs`. Change their first parameter from `IngredientList` to `&AggregatedList` and read `list.list` / `list.aisle` inside — the crate-private fields exist for exactly this.

Add `pub mod shopping_list;` to `crates/core/src/format/mod.rs`.

- [ ] **Step 7: Reduce `src/shopping_list.rs:155` to a shell**

Everything from `let aile_path = ...` (line ~198) through `list = list.subtract_pantry(...)` (line ~312) is replaced by:

```rust
    let mut ctx_core = ctx.to_core();
    if let Some(aisle) = args.aisle {
        ctx_core = ctx_core.with_aisle(cookcli_core::ConfigSource::Path(aisle));
    }
    if args.ignore_pantry {
        ctx_core = ctx_core.with_pantry(cookcli_core::ConfigSource::None);
    } else if let Some(pantry) = args.pantry {
        ctx_core = ctx_core.with_pantry(cookcli_core::ConfigSource::Path(pantry));
    }

    let outcome = cookcli_core::shopping_list::generate(
        &ctx_core,
        cookcli_core::shopping_list::GenerateRequest {
            recipes: expanded_recipes,
            ignore_references: args.ignore_references,
            ignore_pantry: args.ignore_pantry,
        },
    )?;

    for diagnostic in &outcome.diagnostics {
        tracing::warn!("{}", diagnostic.message);
    }
    let aggregated = outcome.value;
```

The directory-expansion block at the top of `run()` (lines 156-196) stays in the CLI — it is argument handling, not recipe logic.

The `--pantry` read-failure case at `src/shopping_list.rs:265` currently hard-errors when `--pantry` was explicit and only warns otherwise. `ConfigSource::Path(..).read()` returns `CoreError::Io` in both cases. Preserve the distinction in the CLI: when `args.pantry` was `None`, probe the discovered path with `Utf8Path::is_file()` before setting it, so a missing discovered pantry becomes `ConfigSource::None` rather than an error.

- [ ] **Step 8: Run the full suite**

```bash
cargo test 2>&1 | tail -30
```

Expected: `passed=310 failed=0 ignored=26`.

The 14 `shopping_list_characterization_test` snapshots from Task 6 are the guards — `tests/shopping_list_test.rs` and the three `shopping_list_*` snapshots in `snapshot_test.rs` are all `#[ignore]`d and will not catch anything.

If a characterization snapshot fails, you have changed behaviour. The most likely causes, in order:
1. `format_quantity` diverged from the original `quantity_fmt`.
2. The output builders were changed while being moved rather than moved verbatim.
3. Aisle categorisation ordering changed — `categorize()` output order is load-bearing.

**Do not run `cargo insta accept`.**

- [ ] **Step 9: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A
git commit -m "refactor(shopping-list): delegate list generation to cookcli-core"
```

---

## Task 8: search

**Files:**
- Create: `crates/core/src/search.rs`
- Modify: `crates/core/src/lib.rs`, `src/search.rs`

The smallest command — 47 lines. Do it after the two big ones to confirm the pattern holds at the small end.

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/search.rs` with its test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Context;

    fn fixture_dir() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("chicken.cook"),
            "Roast @chicken{1} with @rice{200%g}.\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("salad.cook"), "Chop @tomato{3}.\n").unwrap();
        dir
    }

    #[test]
    fn finds_matching_recipes() {
        let dir = fixture_dir();
        let ctx = Context::new(
            camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap(),
        );

        let outcome = search(
            &ctx,
            SearchRequest {
                query: "chicken".to_string(),
                base_dir: None,
            },
        )
        .expect("searches");

        assert_eq!(outcome.value.len(), 1);
        assert!(outcome.value[0].relative_path.as_str().contains("chicken"));
    }

    #[test]
    fn returns_empty_for_no_matches() {
        let dir = fixture_dir();
        let ctx = Context::new(
            camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap(),
        );

        let outcome = search(
            &ctx,
            SearchRequest {
                query: "zzzznotfound".to_string(),
                base_dir: None,
            },
        )
        .expect("searches");

        assert!(outcome.value.is_empty());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p cookcli-core search
```

Expected: FAIL to compile.

- [ ] **Step 3: Implement `search`**

Prepend to `crates/core/src/search.rs`:

```rust
//! Full-text recipe search.

use crate::{Context, CoreError, Outcome};
use camino::Utf8PathBuf;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct SearchRequest {
    /// Search terms, already joined. All terms must match.
    pub query: String,
    /// Directory to search. Defaults to the context base path.
    pub base_dir: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    /// Path relative to the search root.
    pub relative_path: Utf8PathBuf,
    /// Absolute path on disk.
    pub path: Utf8PathBuf,
    /// Recipe title, when the entry has one.
    pub name: Option<String>,
}

pub fn search(ctx: &Context, req: SearchRequest) -> Result<Outcome<Vec<SearchHit>>, CoreError> {
    let base_dir = req.base_dir.unwrap_or_else(|| ctx.base_path().clone());

    let found = cooklang_find::search(&base_dir, &req.query).map_err(|e| CoreError::Config {
        path: base_dir.clone(),
        message: e.to_string(),
    })?;

    let hits = found
        .into_iter()
        .filter_map(|entry| {
            let path = entry.path()?.clone();
            let relative_path = path.strip_prefix(&base_dir).unwrap_or(&path).to_path_buf();
            Some(SearchHit {
                relative_path,
                path,
                name: entry.name().clone(),
            })
        })
        .collect();

    Ok(Outcome::new(hits))
}
```

- [ ] **Step 4: Export from `lib.rs`**

```rust
pub mod search;
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo test -p cookcli-core search
```

Expected: PASS, two tests.

- [ ] **Step 6: Reduce `src/search.rs:32` to a shell**

```rust
pub fn run(ctx: &Context, args: SearchArgs) -> Result<()> {
    let outcome = cookcli_core::search::search(
        &ctx.to_core(),
        cookcli_core::search::SearchRequest {
            query: args.query.join(" "),
            base_dir: args.base_dir,
        },
    )?;

    for hit in &outcome.value {
        println!("\"{}\"", hit.relative_path);
    }

    Ok(())
}
```

The quoting and the use of the *relative* path are load-bearing — `tests/snapshots/snapshot_test__search_output.snap` pins them.

- [ ] **Step 7: Run the full suite**

```bash
cargo test 2>&1 | tail -30
```

Expected: `passed=310 failed=0 ignored=26`, with `search_output`, `search_output_windows` and `search_results_snapshot` passing unchanged.

- [ ] **Step 8: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A
git commit -m "refactor(search): delegate recipe search to cookcli-core"
```

---

## Task 9: doctor::validate

**Files:**
- Create: `crates/core/src/doctor.rs`
- Modify: `crates/core/src/lib.rs`, `src/doctor.rs`

The hardest task. `run_validate` at `src/doctor.rs:424` **prints while it walks the tree**, and its output includes `cooklang`'s own report rendering with source line context (`parsed.report().print(...)` at line 486). Reproducing that byte-for-byte from a structured type is not practical, so `RecipeValidation` carries the rendered text alongside the structured diagnostics. The CLI prints `rendered`; consumers read `diagnostics`.

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/doctor.rs` with its test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Context, Severity};

    fn fixture_dir() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("good.cook"), "Boil @water{1%l}.\n").unwrap();
        std::fs::write(
            dir.path().join("bad.cook"),
            "Add @salt{1%tsp to the pot.\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn reports_totals_across_the_tree() {
        let dir = fixture_dir();
        let ctx = Context::new(
            camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap(),
        );

        let outcome = validate(&ctx, ValidateRequest { base_dir: None }).expect("validates");

        assert_eq!(outcome.value.total_recipes, 2);
        assert_eq!(outcome.value.recipes_with_errors, 1);
    }

    #[test]
    fn failing_recipe_carries_diagnostics_and_rendered_text() {
        let dir = fixture_dir();
        let ctx = Context::new(
            camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap(),
        );

        let outcome = validate(&ctx, ValidateRequest { base_dir: None }).unwrap();

        let bad = outcome
            .value
            .recipes
            .iter()
            .find(|r| r.path.as_str().contains("bad"))
            .expect("bad.cook present");

        assert!(bad
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error));
        assert!(
            !bad.rendered.is_empty(),
            "rendered report is what the CLI prints"
        );
    }

    #[test]
    fn clean_recipe_has_no_diagnostics() {
        let dir = fixture_dir();
        let ctx = Context::new(
            camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap(),
        );

        let outcome = validate(&ctx, ValidateRequest { base_dir: None }).unwrap();

        let good = outcome
            .value
            .recipes
            .iter()
            .find(|r| r.path.as_str().contains("good"))
            .expect("good.cook present");

        assert!(good.diagnostics.is_empty());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p cookcli-core doctor
```

Expected: FAIL to compile.

- [ ] **Step 3: Implement `doctor::validate`**

Prepend to `crates/core/src/doctor.rs`:

```rust
//! Recipe validation.

use crate::{Context, CoreError, Diagnostic, Outcome};
use camino::Utf8PathBuf;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ValidateRequest {
    /// Directory to validate. Defaults to the context base path.
    pub base_dir: Option<Utf8PathBuf>,
}

/// Validation result for a single recipe.
#[derive(Debug, Clone, Serialize)]
pub struct RecipeValidation {
    /// Path relative to the validation root.
    pub path: Utf8PathBuf,
    pub diagnostics: Vec<Diagnostic>,
    /// `cooklang`'s own report rendering, with source line context.
    ///
    /// The CLI prints this verbatim so its output is unchanged. Structured
    /// consumers should read `diagnostics` instead.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rendered: String,
    /// Recipe references found in this file, as slash-separated paths.
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ValidationReport {
    pub recipes: Vec<RecipeValidation>,
    pub total_recipes: usize,
    pub recipes_with_errors: usize,
    pub recipes_with_warnings: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    /// Reference path -> recipes that reference it, for broken-link checking.
    pub references: BTreeMap<String, Vec<String>>,
}

pub fn validate(
    ctx: &Context,
    req: ValidateRequest,
) -> Result<Outcome<ValidationReport>, CoreError> {
    let base_dir = req.base_dir.unwrap_or_else(|| ctx.base_path().clone());

    let tree = cooklang_find::build_tree(&base_dir).map_err(|e| CoreError::Config {
        path: base_dir.clone(),
        message: e.to_string(),
    })?;

    let mut report = ValidationReport::default();
    walk(&tree, &base_dir, &mut report);

    Ok(Outcome::new(report))
}

fn walk(tree: &cooklang_find::RecipeTree, base_dir: &Utf8PathBuf, report: &mut ValidationReport) {
    if let Some(entry) = &tree.recipe {
        report.total_recipes += 1;

        let recipe_name = entry.name().clone().unwrap_or_else(|| "unknown".to_string());
        let recipe_path = entry
            .path()
            .cloned()
            .unwrap_or_else(|| base_dir.join(&recipe_name));
        let relative_path = recipe_path
            .strip_prefix(base_dir)
            .unwrap_or(&recipe_path)
            .to_path_buf();

        match std::fs::read_to_string(&recipe_path) {
            Ok(content) => {
                let parsed = crate::parser::PARSER.parse(&content);
                let parse_report = parsed.report();

                let error_count = parse_report.errors().count();
                let warning_count = parse_report.warnings().count();

                if error_count > 0 {
                    report.recipes_with_errors += 1;
                    report.total_errors += error_count;
                }
                if warning_count > 0 {
                    report.recipes_with_warnings += 1;
                    report.total_warnings += warning_count;
                }

                let rendered = if error_count > 0 || warning_count > 0 {
                    crate::parser::render_report(
                        &parsed,
                        relative_path.as_str(),
                        &content,
                        true,
                    )
                } else {
                    String::new()
                };

                let mut diagnostics = Vec::new();
                for error in parse_report.errors() {
                    diagnostics.push(
                        Diagnostic::error(error.to_string()).at_file(relative_path.clone()),
                    );
                }
                for warning in parse_report.warnings() {
                    diagnostics.push(
                        Diagnostic::warning(warning.to_string()).at_file(relative_path.clone()),
                    );
                }

                let mut references = Vec::new();
                if let Some(recipe) = parsed.output() {
                    for ingredient in &recipe.ingredients {
                        if let Some(reference) = &ingredient.reference {
                            references.push(if reference.components.is_empty() {
                                reference.name.clone()
                            } else {
                                reference.path("/")
                            });
                        }
                    }
                }
                if !references.is_empty() {
                    report
                        .references
                        .insert(relative_path.to_string(), references.clone());
                }

                report.recipes.push(RecipeValidation {
                    path: relative_path,
                    diagnostics,
                    rendered,
                    references,
                });
            }
            Err(e) => {
                report.recipes_with_errors += 1;
                report.total_errors += 1;
                report.recipes.push(RecipeValidation {
                    path: relative_path.clone(),
                    diagnostics: vec![Diagnostic::error(format!(
                        "Failed to read file: {e}"
                    ))
                    .at_file(relative_path.clone())],
                    rendered: String::new(),
                    references: Vec::new(),
                });
            }
        }
    }

    for child in tree.children.values() {
        walk(child, base_dir, report);
    }
}
```

**Before writing `walk`, read `src/doctor.rs:441-520`** and mirror its traversal exactly — particularly how it recurses into `tree.children`, which the snippet above assumes is a map. Match the real field name and shape.

- [ ] **Step 4: Export from `lib.rs`**

```rust
pub mod doctor;
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo test -p cookcli-core doctor
```

Expected: PASS, three tests.

- [ ] **Step 6: Reduce `src/doctor.rs`'s `run_validate` to a shell**

```rust
fn run_validate(ctx: &Context, args: ValidateArgs) -> Result<()> {
    let outcome = cookcli_core::doctor::validate(
        &ctx.to_core(),
        cookcli_core::doctor::ValidateRequest {
            base_dir: args.base_path,
        },
    )?;
    let report = outcome.value;

    for recipe in &report.recipes {
        if !recipe.rendered.is_empty() {
            println!("\n📄 {}", recipe.path);
            print!("{}", recipe.rendered);
        }
    }

    // ... existing summary printing, reading from `report` instead of `stats`
}
```

The summary block after the tree walk (`src/doctor.rs:520` onward) keeps its exact wording and emoji; only its data source changes from the `stats` tuple to `report`'s named fields. The reference-checking block reads `report.references`.

- [ ] **Step 7: Run the full suite**

```bash
cargo test 2>&1 | tail -30
```

Expected: `passed=310 failed=0 ignored=26`. `doctor_validate_output` and
`doctor_validate_output_snapshot` are the strict guards here — they pin the exact
rendered diagnostics. If they fail, the most likely cause is the `ansi` argument
to `render_report`: `src/doctor.rs:486` calls `report().print(path, content, true)`,
so core must pass `true`.

- [ ] **Step 8: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A
git commit -m "refactor(doctor): delegate recipe validation to cookcli-core"
```

---

## Task 10: The aisle diagnostic behaviour fix

**Files:**
- Modify: `src/shopping_list.rs`
- Modify: `tests/snapshots/` (only if a snapshot legitimately changes)

This is the one deliberate behaviour change in the plan, isolated into its own commit so it is visible in review rather than folded into a refactor.

Today, a malformed `aisle.conf` logs through `warn!` and silently falls back to `Default::default()` (`src/shopping_list.rs:218-227`). Task 7 made core return that as a `Diagnostic`. This task decides what the CLI does with it.

- [ ] **Step 1: Reproduce the current behaviour**

```bash
mkdir -p /tmp/cook-aisle-test/config
cd /tmp/cook-aisle-test
printf 'Chop @tomato{3}.\n' > salad.cook
printf 'this is not valid [[[\n' > config/aisle.conf
cargo run --manifest-path /Users/alexeydubovskoy/Cooklang/CookCLI/Cargo.toml -- shopping-list salad.cook
```

Record exactly what is printed to stdout and stderr.

- [ ] **Step 2: Make the diagnostic visible on stderr**

In `src/shopping_list.rs`, the diagnostic loop added in Task 7 already does this:

```rust
    for diagnostic in &outcome.diagnostics {
        tracing::warn!("{}", diagnostic.message);
    }
```

Confirm the malformed-aisle message appears on stderr at default verbosity. `configure_logging` maps verbosity 0 to `"warn,cook=warn"`, so `warn!` is visible by default.

- [ ] **Step 3: Verify stdout is unchanged**

```bash
cd /tmp/cook-aisle-test
cargo run --manifest-path /Users/alexeydubovskoy/Cooklang/CookCLI/Cargo.toml -- shopping-list salad.cook 2>/dev/null
```

Expected: byte-identical to what you recorded in Step 1's stdout. The fix surfaces
information on stderr; it must not change the list itself.

- [ ] **Step 4: Run the full suite**

```bash
cargo test 2>&1 | tail -30
```

Expected: `passed=310 failed=0 ignored=26`, no snapshot changes. `assert_cmd` snapshots capture stdout, so a stderr-only change cannot move them.

**If a snapshot does change:** stop. That means stdout moved, which is not what this task is for. Investigate before proceeding, and do not accept the snapshot without writing the reason into the commit message.

- [ ] **Step 5: Clean up and commit**

```bash
rm -rf /tmp/cook-aisle-test
cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A
git commit -m "fix(shopping-list): surface malformed aisle config instead of silently defaulting

A malformed aisle.conf previously fell back to Default::default() with the
warning discarded. It now returns a diagnostic, which the CLI logs to stderr.
Stdout is unchanged."
```

---

## Task 11: pantry

**Files:**
- Create: `crates/core/src/pantry.rs`
- Modify: `crates/core/src/lib.rs`, `src/pantry.rs`

The bulkiest command at 1,260 lines with eight subcommands: `list`, `add`, `remove`, `update`, `depleted`, `expiring`, `recipes`, `plan`.

- [ ] **Step 1: Read the existing command in full**

```bash
sed -n '317,1260p' /Users/alexeydubovskoy/Cooklang/CookCLI/src/pantry.rs
```

Map each subcommand's handler to a core function before writing any code. The eight
handlers are dispatched from `run()` at line 317.

- [ ] **Step 2: Write the failing test for `list`**

Create `crates/core/src/pantry.rs` with its test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConfigSource, Context};

    const PANTRY: &str = r#"
[dairy]
milk = { quantity = "1%l", expiry = "2026-12-01" }
butter = "200%g"

[staples]
flour = { quantity = "1%kg", low = "200%g" }
"#;

    fn ctx() -> Context {
        Context::new(camino::Utf8PathBuf::from("/nonexistent"))
            .with_pantry(ConfigSource::Inline(PANTRY.to_string()))
    }

    #[test]
    fn lists_all_items_grouped_by_section() {
        let outcome = list(&ctx(), ListRequest { section: None }).expect("lists");

        let sections: Vec<&str> = outcome
            .value
            .sections
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(sections.contains(&"dairy"));
        assert!(sections.contains(&"staples"));
    }

    #[test]
    fn filters_by_section() {
        let outcome = list(
            &ctx(),
            ListRequest {
                section: Some("dairy".to_string()),
            },
        )
        .expect("lists");

        assert_eq!(outcome.value.sections.len(), 1);
        assert_eq!(outcome.value.sections[0].name, "dairy");
    }

    #[test]
    fn missing_pantry_yields_an_empty_list() {
        let ctx = Context::new(camino::Utf8PathBuf::from("/nonexistent"));
        let outcome = list(&ctx, ListRequest { section: None }).expect("lists");
        assert!(outcome.value.sections.is_empty());
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
cargo test -p cookcli-core pantry
```

Expected: FAIL to compile.

- [ ] **Step 4: Implement the read-side functions**

Prepend to `crates/core/src/pantry.rs`:

```rust
//! Pantry inspection and mutation.

use crate::{Context, CoreError, Diagnostic, Outcome};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PantryItem {
    pub name: String,
    pub quantity: Option<String>,
    pub expiry: Option<String>,
    pub low: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PantrySection {
    pub name: String,
    pub items: Vec<PantryItem>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PantryContents {
    pub sections: Vec<PantrySection>,
}

#[derive(Debug, Clone, Default)]
pub struct ListRequest {
    /// Restrict output to one section.
    pub section: Option<String>,
}

/// Load and parse the pantry, collecting any parse warnings as diagnostics.
pub fn load(ctx: &Context) -> Result<Outcome<PantryContents>, CoreError> {
    let mut diagnostics = Vec::new();

    let Some(content) = ctx.pantry().read()? else {
        return Ok(Outcome::new(PantryContents::default()));
    };

    let result = cooklang::pantry::parse_lenient(&content);
    for warning in result.report().warnings() {
        diagnostics.push(Diagnostic::warning(format!(
            "Pantry configuration warning: {warning}"
        )));
    }

    let Some(mut conf) = result.output().cloned() else {
        diagnostics.push(Diagnostic::warning("Failed to parse pantry file"));
        return Ok(Outcome::with_diagnostics(
            PantryContents::default(),
            diagnostics,
        ));
    };
    conf.rebuild_index();

    Ok(Outcome::with_diagnostics(to_contents(&conf), diagnostics))
}

pub fn list(ctx: &Context, req: ListRequest) -> Result<Outcome<PantryContents>, CoreError> {
    let mut outcome = load(ctx)?;

    if let Some(section) = req.section {
        outcome
            .value
            .sections
            .retain(|s| s.name.eq_ignore_ascii_case(&section));
    }

    Ok(outcome)
}
```

**`to_contents` must be written against the real `cooklang::pantry::PantryConf` shape.** Read `src/pantry.rs`'s existing rendering code (the `list` handler) to see how it walks `conf.sections` and reads each item's quantity, expiry and low-stock threshold, and mirror those accessors exactly.

- [ ] **Step 5: Run the read-side tests**

```bash
cargo test -p cookcli-core pantry
```

Expected: PASS, three tests.

- [ ] **Step 6: Port the remaining seven subcommands**

For each of `add`, `remove`, `update`, `depleted`, `expiring`, `recipes`, `plan`:

1. Add a `XxxRequest` struct and an `xxx()` function to `crates/core/src/pantry.rs`, moving the logic from the corresponding handler in `src/pantry.rs`.
2. Add at least one test to the `tests` module covering its primary behaviour, following the shape of the `list` tests above.
3. Reduce the CLI handler to: build request, call core, print.

The mutating commands (`add`, `remove`, `update`) write the pantry file. Keep the
write path in core and have it take the target path from
`ctx.pantry().path()`, returning `CoreError::Config` when the pantry source is
`Inline` or `None` — an in-memory pantry cannot be written back.

- [ ] **Step 7: Run the full suite**

```bash
cargo test 2>&1 | tail -30
```

Expected: `passed=310 failed=0 ignored=26`. `tests/pantry_test.rs` is 1,062 lines and is the guard.

- [ ] **Step 8: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A
git commit -m "refactor(pantry): delegate pantry operations to cookcli-core"
```

---

## Task 12: report

**Files:**
- Create: `crates/core/src/report.rs`
- Modify: `crates/core/src/lib.rs`, `src/report.rs`

Note two things about the current implementation: it reads the recipe with a plain
`fs::read_to_string` rather than through `cooklang-find` (`src/report.rs:74`), and it
calls `std::process::exit(1)` on a render failure (`src/report.rs:143`). Core must
return an error instead; the CLI keeps the exit.

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/report.rs` with its test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Context, RecipeSource};

    #[test]
    fn renders_a_template() {
        let ctx = Context::new(camino::Utf8PathBuf::from("."));

        let outcome = render(
            &ctx,
            RenderRequest {
                source: RecipeSource::Content {
                    text: "Boil @water{1%l}.\n".to_string(),
                    name: "simple".to_string(),
                },
                template: "{{ ingredients | length }}".to_string(),
                scale: 1.0,
                datastore: None,
            },
        )
        .expect("renders");

        assert_eq!(outcome.value.trim(), "1");
    }

    #[test]
    fn a_broken_template_is_a_render_error() {
        let ctx = Context::new(camino::Utf8PathBuf::from("."));

        let err = render(
            &ctx,
            RenderRequest {
                source: RecipeSource::Content {
                    text: "Boil @water{1%l}.\n".to_string(),
                    name: "simple".to_string(),
                },
                template: "{% for x in %}".to_string(),
                scale: 1.0,
                datastore: None,
            },
        )
        .unwrap_err();

        assert!(matches!(err, CoreError::Render { .. }), "got {err:?}");
    }
}
```

The exact template syntax depends on `cooklang-reports` 0.5.1. If
`{{ ingredients | length }}` is not valid, read `src/report.rs`'s doc comment
(lines 13-23) for the real variable names and adjust — but keep both tests.

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p cookcli-core report
```

Expected: FAIL to compile.

- [ ] **Step 3: Implement `report::render`**

Prepend to `crates/core/src/report.rs`:

```rust
//! Jinja2 template rendering over recipes.

use crate::{Context, CoreError, Outcome, RecipeSource};
use camino::Utf8PathBuf;
use cooklang_reports::{render_template_with_config, Config};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub source: RecipeSource,
    /// Jinja2 template text.
    pub template: String,
    pub scale: f64,
    pub datastore: Option<Utf8PathBuf>,
}

pub fn render(ctx: &Context, req: RenderRequest) -> Result<Outcome<String>, CoreError> {
    let (recipe_text, scale) = match req.source {
        RecipeSource::Content { text, .. } => (text, req.scale),
        RecipeSource::Path(path) => {
            let query = path.as_str();
            let (name, inline_scale) = crate::recipe::split_name_and_scale(query)
                .map(|(n, f)| (n, Some(f)))
                .unwrap_or((query, None));
            let text = std::fs::read_to_string(name)?;
            (text, inline_scale.unwrap_or(req.scale))
        }
    };

    let mut builder = Config::builder();
    builder.scale(scale);
    builder.base_path(PathBuf::from(ctx.base_path()));

    if let Some(datastore) = req.datastore {
        builder.datastore_path(PathBuf::from(datastore));
    }
    if let Some(aisle) = ctx.aisle().path() {
        builder.aisle_path(PathBuf::from(aisle));
    }
    if let Some(pantry) = ctx.pantry().path() {
        builder.pantry_path(PathBuf::from(pantry));
    }

    let config = builder.build();

    let output = render_template_with_config(&recipe_text, &req.template, &config).map_err(
        |err| CoreError::Render {
            message: err.format_with_source(),
        },
    )?;

    Ok(Outcome::new(output))
}
```

Putting `format_with_source()` into the error message preserves the enhanced
error output the CLI prints today at `src/report.rs:142`.

- [ ] **Step 4: Export from `lib.rs`**

```rust
pub mod report;
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo test -p cookcli-core report
```

Expected: PASS, two tests.

- [ ] **Step 6: Reduce `src/report.rs:64` to a shell**

```rust
pub fn run(ctx: &crate::Context, args: ReportArgs) -> Result<()> {
    warn!("⚠️  The report command is a prototype feature and will change in future versions.");

    let template = fs::read_to_string(&args.template)
        .with_context(|| format!("Failed to read template file: {}", args.template))?;

    let mut ctx_core = ctx.to_core();
    if let Some(base_path) = args.base_path {
        ctx_core = cookcli_core::Context::new(resolve_to_absolute_path(&base_path)?)
            .with_aisle(ctx_core.aisle().clone())
            .with_pantry(ctx_core.pantry().clone());
    }
    if let Some(aisle) = args.aisle {
        ctx_core = ctx_core.with_aisle(cookcli_core::ConfigSource::Path(
            resolve_to_absolute_path(&aisle)?,
        ));
    }
    if let Some(pantry) = args.pantry {
        ctx_core = ctx_core.with_pantry(cookcli_core::ConfigSource::Path(
            resolve_to_absolute_path(&pantry)?,
        ));
    }

    let result = cookcli_core::report::render(
        &ctx_core,
        cookcli_core::report::RenderRequest {
            source: cookcli_core::RecipeSource::Path(args.recipe.into()),
            template,
            scale: 1.0,
            datastore: args.datastore,
        },
    );

    match result {
        Ok(outcome) => {
            println!("{}", outcome.value);
            Ok(())
        }
        Err(cookcli_core::CoreError::Render { message }) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}
```

- [ ] **Step 7: Run the full suite**

```bash
cargo test 2>&1 | tail -30
```

Expected: `passed=310 failed=0 ignored=26`.

- [ ] **Step 8: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A
git commit -m "refactor(report): delegate template rendering to cookcli-core"
```

---

## Task 13: Lift the shopping list store out of the server

**Files:**
- Move: `src/server/shopping_list_store.rs` → `crates/core/src/shopping_list/store.rs`
- Modify: `crates/core/src/shopping_list/mod.rs`, `src/server/mod.rs`, `src/server/handlers/shopping_list.rs`, `src/server/shopping_list_watcher.rs`

Last, because it is the only task touching server code. The store is 440 lines
already built on `cooklang::shopping_list`; it is trapped behind the `server`
feature, which library consumers compile out.

- [ ] **Step 1: Move the file**

```bash
git mv src/server/shopping_list_store.rs crates/core/src/shopping_list/store.rs
```

- [ ] **Step 2: Convert its error type**

The store uses `anyhow::{Context, Result}`. Core does not depend on `anyhow`.
Replace every `Result<T>` with `Result<T, CoreError>`, and every
`.context("reading .shopping-list")` with the `?` operator — `CoreError::Io`
already carries the underlying `std::io::Error` through `#[from]`.

For the two `shopping_list::parse` calls that map a parse failure into
`anyhow::anyhow!`, use:

```rust
        shopping_list::parse(&content).map_err(|e| CoreError::Config {
            path: self.list_path.clone(),
            message: e.to_string(),
        })
```

- [ ] **Step 3: Declare it and re-export**

In `crates/core/src/shopping_list/mod.rs`:

```rust
pub mod store;
pub use store::{ShoppingListApiItem, ShoppingListStore};
```

- [ ] **Step 4: Add a round-trip test**

Append to `crates/core/src/shopping_list/store.rs`'s test module (or create one):

```rust
#[cfg(test)]
mod store_tests {
    use super::*;

    #[test]
    fn writes_and_reads_back_a_list() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let store = ShoppingListStore::new(&base);

        let items = vec![ShoppingListApiItem {
            path: "pasta.cook".to_string(),
            name: "Pasta".to_string(),
            scale: 2.0,
            included_references: None,
            recipes: None,
        }];

        store.write_items(&items).expect("writes");
        let read_back = store.read_items().expect("reads");

        assert_eq!(read_back.len(), 1);
        assert_eq!(read_back[0].path, "pasta.cook");
        assert_eq!(read_back[0].scale, 2.0);
    }
}
```

**Use the store's real method names.** Read the moved file for the actual
read/write API before writing this test — `write_items`/`read_items` are the
expected names but the file is the authority.

- [ ] **Step 5: Point the server at core**

In `src/server/mod.rs`, delete `mod shopping_list_store;` and add:

```rust
pub use cookcli_core::shopping_list::store::{ShoppingListApiItem, ShoppingListStore};
```

Existing `use` statements in `src/server/handlers/shopping_list.rs` and
`src/server/shopping_list_watcher.rs` then resolve unchanged.

- [ ] **Step 6: Run the tests**

```bash
cargo test -p cookcli-core && cargo test 2>&1 | tail -30
```

Expected: core tests pass; full suite at `passed=310 failed=0 ignored=26`. `tests/e2e/` and the
server integration tests exercise the store through HTTP.

- [ ] **Step 7: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A
git commit -m "refactor(core): lift the shopping list store out of the server feature"
```

---

## Task 14: Verify the CLI is a shell and prepare to publish

**Files:**
- Modify: `src/lib.rs`, `crates/core/Cargo.toml`, `crates/core/README.md`

- [ ] **Step 1: Confirm no command module still holds recipe logic**

```bash
grep -n "PARSER\.\|cooklang::parse\|IngredientList::new\|parse_lenient" \
  src/recipe/*.rs src/shopping_list.rs src/search.rs src/doctor.rs src/pantry.rs src/report.rs
```

Expected: no matches. Every hit is logic that belongs in core and did not move.

- [ ] **Step 2: Confirm core's dependency tree is small**

```bash
cargo tree -p cookcli-core --depth 1
```

Expected: no `axum`, `reqwest`, `tokio`, `clap`, `fluent`, `rust-embed`, `askama`,
`self_update`. If any appear, a moved file dragged in an import that belongs in the CLI.

- [ ] **Step 3: Update `src/lib.rs` to re-export core**

```rust
// Re-export the core library so downstream consumers of `cookcli` can reach it.
pub use cookcli_core;
```

Keep the existing `pub mod` declarations — the test suite uses them.

- [ ] **Step 4: Write `crates/core/README.md`**

```markdown
# cookcli-core

The recipe, shopping list, pantry and report operations behind
[CookCLI](https://github.com/cooklang/cookcli), packaged as a library.

Every command is `fn(&Context, Request) -> Result<Outcome<T>, CoreError>`.
`Outcome<T>` carries diagnostics alongside the value, so warnings that the CLI
prints to stderr are available to programmatic consumers.

Recipes and configuration can be supplied as paths or as in-memory text, so
editors can operate on unsaved buffers.

## Example

```rust
use cookcli_core::{Context, RecipeSource, recipe};

let ctx = Context::new("/path/to/recipes".into());
let outcome = recipe::read(&ctx, recipe::ReadRequest {
    source: RecipeSource::Path("pasta.cook".into()),
    scale: 2.0,
})?;

println!("{} ingredients", outcome.value.recipe.ingredients.len());
for diagnostic in &outcome.diagnostics {
    eprintln!("{}: {}", diagnostic.severity as u8, diagnostic.message);
}
# Ok::<(), cookcli_core::CoreError>(())
```
```

Add `readme = "README.md"` to `crates/core/Cargo.toml`.

- [ ] **Step 5: Dry-run the publish**

```bash
cargo publish -p cookcli-core --dry-run
```

Expected: packaging succeeds. Fix any `include`/`exclude` or missing-metadata
complaints.

- [ ] **Step 6: Check the API against the editor's existing surface**

The spec names this as the mitigation for designing the API against a single
consumer. Read the editor's NAPI declarations:

```bash
cat /Users/alexeydubovskoy/Cooklang/editor/packages/cooklang-native/index.d.ts
```

For each export that is in scope, confirm it is expressible against
`cookcli-core` today, and write the mapping into
`crates/core/README.md` under a `## Consumer coverage` heading:

| Editor export | Core equivalent |
|---|---|
| `parse` | `recipe::read` with `RecipeSource::Content` |
| `generateShoppingList` | `shopping_list::generate` with `ConfigSource::Inline` |
| `findRecipe` | `find::get_recipe` |
| `renderReport` | `report::render` |
| `parseShoppingList`, `writeShoppingList`, `parseChecked`, `writeCheckEntry`, `checkedSet`, `compactChecked` | `shopping_list::store` |

Out of scope, and expected to have no equivalent: `parseMenu`, `startSync`,
`stopSync`, `getSyncStatus`, `onSyncStatusChanged`, `LspServer`.

**If any in-scope row cannot be expressed, stop and report it.** That is a real
API gap and it is far cheaper to fix now than after Spec 2 has started.

- [ ] **Step 7: Final full verification**

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test 2>&1 | tail -30
```

Expected: all clean, counts at `passed=310 failed=0 ignored=26`.

- [ ] **Step 8: Confirm no snapshot was silently accepted**

```bash
git diff main --stat -- tests/snapshots/ | grep -v shopping_list_characterization_test
```

Expected: empty output. The only snapshots this plan may add are Task 6's 14
`shopping_list_characterization_test__*.snap` files, which the filter excludes.
Task 10 was designed to be a stderr-only change, so no pre-existing snapshot
should have moved; if one did, its commit message must explain why.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "chore(core): add README and prepare cookcli-core for publishing"
```

---

## Definition of Done

- [ ] `crates/core` exists with the six commands, the store, formatters, and its own unit tests.
- [ ] `cargo test` reports `310 passed; 0 failed; 26 ignored`.
- [ ] `tests/snapshots/` is unchanged from `main` apart from Task 6's 14 new `shopping_list_characterization_test__*.snap` files.
- [ ] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean.
- [ ] `cargo tree -p cookcli-core --depth 1` shows no CLI-only or server-only dependencies.
- [ ] No command module in `src/` contains parsing or aggregation logic.
- [ ] `cargo publish -p cookcli-core --dry-run` succeeds.

## Out of Scope (Spec 2)

- Rewiring `editor/packages/cooklang-native` onto `cookcli-core`.
- New `cook shopping-list` subcommands for the persisted `.shopping-list` state.
- Collapsing cookbot's `crates/tui/src/cooklang/`.
- `parseMenu` / menu logic, `sync`, `lsp`, `import`.
