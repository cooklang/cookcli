# cooklang-format Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract CookCLI's recipe output converters from `cookcli-core` into a standalone, publishable `cooklang-format` crate, so other projects can render Cooklang recipes without depending on the CLI's internals.

**Architecture:** A new workspace member `crates/format` receives 8 files (~3,400 lines) from `crates/core/src/format/`, bottom-up in dependency order. `cookcli-core` gains a dependency on it and keeps `pub mod format` as a re-export shim, so every existing call site in the CLI, server and web builders compiles unchanged. The shopping-list formatter stays in core because it renders core's `AggregatedList`.

**Tech Stack:** Rust 2021, Cargo workspace, `cooklang` 0.18.5, insta snapshots, release-please + GitHub Actions for publishing.

**Spec:** `docs/superpowers/specs/2026-08-14-cooklang-format-crate-design.md`

---

## How to verify this refactor

This is a **move, not a rewrite**. The existing test suite is the specification: it must stay green at every commit, and the insta snapshots must not change by a single byte. If a snapshot changes, the move altered rendering — that is a bug, not a snapshot to accept. **Never run `cargo insta accept` during this plan.**

**The green check.** Run this after every move; it is referenced by name throughout the plan:

```bash
out=$(cargo test --workspace --quiet 2>&1)
echo "failures: $(echo "$out" | grep -cE 'test result: FAILED')"
echo "passed:   $(echo "$out" | grep -oE '^test result: ok\. [0-9]+ passed' | grep -oE '[0-9]+' | paste -sd+ - | bc)"
```

Expected at every commit: `failures: 0` and `passed: 760`.

Count **passing tests, not suites**. Suite count is not an invariant here: `cargo test --workspace` emits one result line per test binary, so merely adding `crates/format` to the workspace takes the line count from 22 to 24 (its empty unit-test and doc-test harnesses) without any test being added or lost. Passing-test count is what a move must preserve — 760, measured on 2026-08-14 on branch `feat/cooklang-format-crate` after Unit A's scaffolding. It rises only where the plan adds real tests (Tasks 9 and 10); it must never fall.

The one place genuinely new tests are called for is the new crate's public surface — Task 9 adds an integration test proving the crate works standalone, without `cookcli-core`.

## File Structure

**Created:**

| path | responsibility |
|---|---|
| `crates/format/Cargo.toml` | manifest for the new crate |
| `crates/format/LICENSE` | MIT, copy of root `LICENSE` |
| `crates/format/README.md` | front page + attribution + doctested example |
| `crates/format/src/lib.rs` | `Style`, `PaperSize`, `REFERENCE_SEPARATOR`, `*_to_string` wrappers, module declarations, `#[cfg(test)] mod test_support` |
| `crates/format/src/{number,quantity,schema,human,markdown,cooklang,latex,typst}.rs` | moved verbatim from `crates/core/src/format/` |
| `crates/format/tests/standalone.rs` | proves the crate renders without core |

**Modified:**

| path | change |
|---|---|
| `Cargo.toml` | add `crates/format` to workspace members |
| `crates/core/Cargo.toml` | add `cooklang-format` dep; drop deps that leave with the files |
| `crates/core/src/format/mod.rs` | reduced to a re-export shim + `pub mod shopping_list;` |
| `crates/core/src/find.rs` | `REFERENCE_SEPARATOR` becomes a re-export |
| `.github/workflows/release.yaml` | publish `cooklang-format` before `cookcli-core` |
| `CLAUDE.md` | workspace structure section |

**Unchanged (deliberately):** `crates/core/src/format/shopping_list.rs`, `crates/core/src/parser.rs`, all of `src/`, all of `tests/`.

---

### Task 1: Scaffold the crate

**Files:**
- Create: `crates/format/Cargo.toml`, `crates/format/src/lib.rs`, `crates/format/LICENSE`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Confirm the baseline is green before touching anything**

Run: the **green check** (see "How to verify this refactor")
Expected: `failures: 0`. Record the `passed:` number — that is the invariant every later step must preserve. If anything fails here, stop: the failure predates this work and the plan's safety net is not in place.

- [ ] **Step 2: Create `crates/format/Cargo.toml`**

Dependency versions are copied from `crates/core/Cargo.toml` verbatim, so all three manifests resolve the same versions. The `cooklang` declaration in particular must stay identical — see the comment.

```toml
[package]
name = "cooklang-format"
version = "0.1.0"
edition = "2021"
description = "Render Cooklang recipes as Markdown, LaTeX, Typst, JSON, terminal text, or Cooklang source"
license = "MIT"
readme = "README.md"
repository = "https://github.com/cooklang/cookcli"
homepage = "https://cooklang.org"
keywords = ["cooklang", "recipes", "markdown", "formatter"]
categories = ["text-processing"]

[dependencies]
# Strips ANSI escape codes at the writer, so that `Style::Plain` can be
# honoured without touching yansi's global colour switch.
anstream = "0.6"
anstyle = "1"
anstyle-yansi = "2"
# Keep this declaration in step with the root and `cookcli-core` manifests.
# `bundled_units` is deliberately absent: enabling it here would switch it back
# on for both other crates through feature unification and undo #433, which
# removed it so quantities keep their authored units.
cooklang = { version = "0.18.5", default-features = false, features = ["aisle", "pantry", "shopping_list"] }
# Timer durations in the human formatter.
humantime = "2"
regex = "1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
# `ansi-cell` lets the human formatter measure column widths ignoring escapes.
tabular = { version = "0.2", features = ["ansi-cell"] }
# `terminal_size` backs `textwrap::termwidth()`, which sets the wrap width.
textwrap = { version = "0.16", features = ["terminal_size"] }
yansi = "1"
```

- [ ] **Step 3: Create `crates/format/src/lib.rs` as a placeholder**

Content is filled in by later tasks; this only needs to compile.

```rust
//! Render Cooklang recipes into text formats.
//!
//! Each module turns a parsed [`cooklang::Recipe`] into one target format.
//! The `print_*` functions write into a [`std::io::Write`]; the `*_to_string`
//! wrappers are for callers that want a `String`.

#![warn(missing_docs)]

/// The `cooklang` crate this library was built against.
///
/// Every public function takes `cooklang` types, so they are part of this
/// crate's public surface. Re-exporting lets consumers name them without
/// adding their own `cooklang` dependency, which could otherwise resolve to a
/// different version and fail to unify.
pub use cooklang;
```

- [ ] **Step 4: Copy the licence**

```bash
cp LICENSE crates/format/LICENSE
```

- [ ] **Step 5: Add the crate to the workspace**

In `Cargo.toml`, change:

```toml
members = [".", "crates/core"]
```

to:

```toml
members = [".", "crates/core", "crates/format"]
```

- [ ] **Step 6: Verify it builds**

Run: `cargo build -p cooklang-format`
Expected: `Finished` with no warnings. (Unused dependencies do not warn; that is expected at this stage.)

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/format
git commit -m "chore(format): scaffold the cooklang-format crate"
```

---

### Task 2: Wire core to depend on the new crate

**Files:**
- Modify: `crates/core/Cargo.toml`

- [ ] **Step 1: Add the dependency**

In `crates/core/Cargo.toml`, in the `[dependencies]` table (alphabetical order puts it after `cooklang-find`):

```toml
# The output formatters, split out so other projects can render recipes
# without depending on the CLI's internals. `version` + `path`: local builds
# use the workspace copy, the published crate resolves from crates.io.
cooklang-format = { version = "0.1.0", path = "../format" }
```

- [ ] **Step 2: Verify the workspace still builds and tests pass**

Run: the **green check** (see "How to verify this refactor")
Expected: `failures: 0` and `passed: 760`

- [ ] **Step 3: Commit**

```bash
git add crates/core/Cargo.toml Cargo.lock
git commit -m "chore(core): depend on cooklang-format"
```

---

### Task 3: Move the test-support parser

The moving files' tests reach for `crate::parser::{PARSER, parse_recipe}`, which stays in core. The new crate needs its own test-only equivalent with the **same call shape** (`parse_recipe(text, name, scale).expect("..").value`), so the moved test bodies need no edits — which is what keeps this refactor honest.

**Files:**
- Modify: `crates/format/src/lib.rs`

- [ ] **Step 1: Append the test-support module to `crates/format/src/lib.rs`**

```rust
/// A test-only stand-in for `cookcli-core`'s parser.
///
/// The formatters take an already-parsed recipe, so the parser is not part of
/// this crate's public surface — but its own tests still need to build a
/// `Recipe` from source. This reproduces `cookcli_core::parser`'s
/// configuration exactly (no extensions, default converter) and its call
/// shape, so the tests read the same on both sides of the split.
#[cfg(test)]
pub(crate) mod test_support {
    use cooklang::{Converter, CooklangParser, Extensions, Recipe};
    use std::sync::LazyLock;

    pub(crate) static PARSER: LazyLock<CooklangParser> =
        LazyLock::new(|| CooklangParser::new(Extensions::empty(), Converter::default()));

    /// Stands in for `cookcli_core::Outcome`, of which the formatter tests
    /// only ever use `.value`.
    pub(crate) struct Parsed {
        pub(crate) value: Recipe,
    }

    /// Parse and scale, mirroring `cookcli_core::parse_recipe`.
    ///
    /// `scale` is applied unconditionally, including at `1.0`, because that is
    /// what core does — see the note on `parse_unscaled` there.
    pub(crate) fn parse_recipe(text: &str, name: &str, scale: f64) -> Result<Parsed, String> {
        let parsed = PARSER.parse(text);
        if parsed.report().has_errors() {
            return Err(format!("{name} failed to parse"));
        }
        match parsed.into_result() {
            Ok((mut recipe, _)) => {
                recipe.scale(scale, PARSER.converter());
                Ok(Parsed { value: recipe })
            }
            Err(_) => Err(format!("{name} produced no output")),
        }
    }
}
```

- [ ] **Step 2: Verify it compiles under `cfg(test)`**

Run: `cargo test -p cooklang-format`
Expected: compiles; `0 passed` (there are no tests yet). `dead_code` warnings on `parse_recipe`/`PARSER`/`Parsed` are expected here — `PARSER` gains a user in Task 5, `parse_recipe` in Task 6.

- [ ] **Step 3: Commit**

```bash
git add crates/format/src/lib.rs
git commit -m "test(format): add a test-only parser matching core's configuration"
```

---

### Task 4: Move `REFERENCE_SEPARATOR`

Five moving files use it. It defines how a recipe reference is *written*, so it belongs with the writers.

**Files:**
- Modify: `crates/format/src/lib.rs`, `crates/core/src/find.rs:28`, `crates/core/src/lib.rs:32`

- [ ] **Step 1: Read the const and its doc comment**

Run: `sed -n '10,28p' crates/core/src/find.rs`
Expected: the doc comment explaining why this is `/` rather than `std::path::MAIN_SEPARATOR`, ending in `pub const REFERENCE_SEPARATOR: &str = "/";`

- [ ] **Step 2: Move it into `crates/format/src/lib.rs`**

Cut the const **and its full doc comment** from `crates/core/src/find.rs` and paste them into `crates/format/src/lib.rs`, after the `pub use cooklang;` re-export.

- [ ] **Step 3: Re-export it from core so nothing downstream moves**

In `crates/core/src/find.rs`, at the line the const used to occupy:

```rust
/// Re-exported from [`cooklang_format`], which is where the writers that
/// depend on this spelling now live.
pub use cooklang_format::REFERENCE_SEPARATOR;
```

`crates/core/src/lib.rs:32`'s `pub use find::REFERENCE_SEPARATOR;` keeps working unchanged, so the three call sites in `src/server/handlers/` and `src/web/builders.rs` are untouched.

- [ ] **Step 4: Point the moving files' references at the new home**

The five files still say `crate::find::REFERENCE_SEPARATOR`. Inside core that still resolves through the re-export, so they compile as-is. Leave them; Task 6 rewrites those lines as each file moves.

- [ ] **Step 5: Verify**

Run: the **green check** (see "How to verify this refactor")
Expected: `failures: 0` and `passed: 760`

- [ ] **Step 6: Commit**

```bash
git add crates/format/src/lib.rs crates/core/src/find.rs
git commit -m "refactor(format): move REFERENCE_SEPARATOR to cooklang-format"
```

---

### Task 5: Move the primitives — `number.rs` and `quantity.rs`

Leaves of the dependency graph: nothing in them depends on anything else that is moving.

**Files:**
- Create: `crates/format/src/number.rs`, `crates/format/src/quantity.rs`
- Delete: `crates/core/src/format/number.rs`, `crates/core/src/format/quantity.rs`
- Modify: `crates/format/src/lib.rs`, `crates/core/src/format/mod.rs`

- [ ] **Step 1: Move the files**

```bash
git mv crates/core/src/format/number.rs crates/format/src/number.rs
git mv crates/core/src/format/quantity.rs crates/format/src/quantity.rs
```

- [ ] **Step 2: Fix the test import in `quantity.rs`**

In `crates/format/src/quantity.rs`, inside `mod tests`, change:

```rust
    use crate::parser::PARSER;
```

to:

```rust
    use crate::test_support::PARSER;
```

- [ ] **Step 3: Declare the modules in `crates/format/src/lib.rs`**

Add above the `pub use cooklang;` line:

```rust
pub mod number;
pub mod quantity;
```

- [ ] **Step 4: Re-export them from core's shim**

In `crates/core/src/format/mod.rs`, replace the two lines:

```rust
pub mod number;
pub mod quantity;
```

with:

```rust
pub use cooklang_format::{number, quantity};
```

Every `crate::format::quantity::grouped_quantity_fmt` in the still-in-core formatters, and `crate::format::quantity::ordered_components` in `core::shopping_list`, resolves through this re-export unchanged.

- [ ] **Step 5: Verify — the moved tests must run in their new home**

Run: `cargo test -p cooklang-format 2>&1 | grep "test result"`
Expected: `test result: ok.` with a non-zero count (the `quantity` and `number` unit tests).

Run: the **green check** (see "How to verify this refactor")
Expected: `failures: 0` and `passed: 760`

- [ ] **Step 6: Commit**

```bash
git add -A crates/format crates/core
git commit -m "refactor(format): move number and quantity primitives"
```

---

### Task 6: Move the self-contained formatters — `schema`, `markdown`, `cooklang`

These three reference neither `Style` nor `PaperSize` (verified: zero occurrences in each), so they move while those types still live in core. `human`, `latex` and `typst` do reference them and therefore move in Task 7, together with the types themselves.

**Repeat Steps 1–5 below once per file, committing after each**, in this order: `schema.rs`, `markdown.rs`, `cooklang.rs`.

**Files (per file `X.rs`):**
- Create: `crates/format/src/X.rs`
- Delete: `crates/core/src/format/X.rs`
- Modify: `crates/format/src/lib.rs`, `crates/core/src/format/mod.rs`

- [ ] **Step 1: Move the file**

```bash
git mv crates/core/src/format/X.rs crates/format/src/X.rs
```

- [ ] **Step 2: Rewrite its intra-crate paths**

Apply whichever of these appear in the file — the exhaustive list of what crosses the boundary:

| before | after |
|---|---|
| `crate::find::REFERENCE_SEPARATOR` | `crate::REFERENCE_SEPARATOR` |
| `crate::format::quantity::grouped_quantity_fmt` | `crate::quantity::grouped_quantity_fmt` |
| `crate::format::{quantity::grouped_quantity_fmt, PaperSize}` | `crate::{quantity::grouped_quantity_fmt, PaperSize}` |
| `crate::format::{ .. }` (any other) | `crate::{ .. }` |
| `crate::parser::parse_recipe` (tests only) | `crate::test_support::parse_recipe` |
| `crate::parser::PARSER` (tests only) | `crate::test_support::PARSER` |

Find them with: `grep -n "crate::" crates/format/src/X.rs`
Expected after the rewrite: no occurrence of `crate::format::`, `crate::find::` or `crate::parser::` remains.

- [ ] **Step 3: Declare the module in `crates/format/src/lib.rs`**

Add `pub mod X;` to the module list, keeping it alphabetical.

- [ ] **Step 4: Re-export from core's shim**

In `crates/core/src/format/mod.rs`, remove `pub mod X;` and add `X` to the re-export list, which grows as files move until it reads:

```rust
pub use cooklang_format::{cooklang, markdown, number, quantity, schema};
```

- [ ] **Step 5: Verify and commit**

Run: the **green check** (see "How to verify this refactor")
Expected: `failures: 0` and `passed: 760`

Run: `git status --porcelain tests/snapshots`
Expected: empty output. **A modified snapshot means the move changed rendering — stop and investigate rather than accepting it.**

```bash
git add -A crates/format crates/core
git commit -m "refactor(format): move the X formatter to cooklang-format"
```

---

### Task 7: Move `Style`, `PaperSize`, the remaining formatters and the `*_to_string` wrappers

These move as one unit because they cannot compile apart: `human.rs` needs `Style`, `latex.rs`/`typst.rs` need `PaperSize`, and the `*_to_string` wrappers call `human::print_human` and `markdown::print_md` (the latter already moved in Task 6).

**Files:**
- Modify: `crates/format/src/lib.rs`, `crates/core/src/format/mod.rs`
- Move: `crates/core/src/format/{human,latex,typst}.rs` → `crates/format/src/`

- [ ] **Step 1: Move `Style`, `PaperSize` and their impls**

Cut from `crates/core/src/format/mod.rs` into `crates/format/src/lib.rs`, **with their doc comments intact**: the `Style` enum + `impl Style`, and the `PaperSize` enum + `impl PaperSize` (`latex_name`, `typst_name`). Both keep `#[non_exhaustive]` and their derives.

Note for `human.rs`: it contains a private `mod style` that does `use anstyle::Style;`. That is a *different* `Style` and must not be touched — only `crate::format::Style` paths get rewritten.

- [ ] **Step 2: Move `human.rs`, `latex.rs` and `typst.rs` following Task 6's Steps 1–4**

All three files, same path rewrites.

- [ ] **Step 3: Move the wrappers and `into_string`**

Cut `human_to_string`, `markdown_to_string` and the private `into_string` helper from core's `format/mod.rs` into `crates/format/src/lib.rs`, doc comments intact.

- [ ] **Step 4: Move `mod.rs`'s test module**

Cut the whole `#[cfg(test)] mod tests` block from core's `format/mod.rs` into `crates/format/src/lib.rs`. Change its `use crate::parser::{parse_recipe, PARSER};` to `use crate::test_support::{parse_recipe, PARSER};`. The four tests — `plain_is_ansi_with_the_escapes_removed`, `ansi_keeps_the_escape_codes`, `paper_sizes_map_to_both_typesetter_spellings`, `style_default_is_plain_and_only_ansi_is_ansi` — move unchanged otherwise.

- [ ] **Step 5: Re-export from core's shim**

In `crates/core/src/format/mod.rs`, the re-export now covers everything that has moved:

```rust
pub use cooklang_format::{
    cooklang, human, human_to_string, latex, markdown, markdown_to_string, number, quantity,
    schema, typst, PaperSize, Style,
};
```

`crates/core/src/lib.rs:33`'s `pub use format::{PaperSize, Style};` keeps working, so `src/recipe/read.rs`'s `impl From<PaperSizeArg> for format::PaperSize` and every `format::Style` use in the CLI are untouched.

- [ ] **Step 6: Verify**

Run: the **green check** (see "How to verify this refactor")
Expected: `failures: 0` and `passed: 760`

Run: `cargo test -p cooklang-format 2>&1 | grep "paper_sizes_map_to_both_typesetter_spellings"`
Expected: the test name appears and passes, proving the moved test module runs in its new home.

- [ ] **Step 7: Commit**

```bash
git add -A crates/format crates/core
git commit -m "refactor(format): move Style, PaperSize and the to_string wrappers"
```

---

### Task 8: Reduce core's `format` module to its final shim

**Files:**
- Modify: `crates/core/src/format/mod.rs`
- Modify: `crates/core/Cargo.toml`

- [ ] **Step 1: Write the final shim**

`crates/core/src/format/mod.rs` becomes exactly:

```rust
//! Recipe output formatters.
//!
//! The recipe formatters live in [`cooklang_format`], which is publishable on
//! its own so other projects can render recipes without depending on this
//! crate. They are re-exported here so that callers keep reaching them as
//! `cookcli_core::format::..`.
//!
//! [`shopping_list`] stays here: it renders this crate's
//! [`AggregatedList`](crate::shopping_list::AggregatedList), so moving it would
//! point the dependency between the two crates the wrong way.

pub mod shopping_list;

pub use cooklang_format::{
    cooklang, human, human_to_string, latex, markdown, markdown_to_string, number, quantity,
    schema, typst, PaperSize, Style,
};
```

- [ ] **Step 2: Confirm nothing is left behind**

Run: `ls crates/core/src/format/`
Expected: exactly `mod.rs` and `shopping_list.rs`.

- [ ] **Step 3: Drop the dependencies that left with the files**

Check each of these in `crates/core/Cargo.toml` and remove the ones no longer referenced anywhere under `crates/core/src/`: `anstyle`, `anstyle-yansi`, `humantime`, `textwrap`.

Verify each before removing, e.g.:

```bash
for d in anstyle anstyle_yansi humantime textwrap regex; do printf "%-16s " "$d"; grep -rl "$d" crates/core/src | tr '\n' ' '; echo; done
```

Expected: `anstyle`, `anstyle_yansi`, `humantime` and `textwrap` return no files → remove those four from the manifest. Keep any that still show a hit. `anstream`, `serde_json`, `serde_yaml`, `tabular`, `yansi` and `camino` are still used by `shopping_list.rs` and elsewhere — keep them.

- [ ] **Step 4: Verify the whole workspace**

Run: the **green check** (see "How to verify this refactor")
Expected: `failures: 0` and `passed: 760`

Run: `git status --porcelain tests/snapshots`
Expected: empty.

- [ ] **Step 5: Commit**

```bash
git add crates/core Cargo.lock
git commit -m "refactor(core): reduce the format module to a re-export shim"
```

---

### Task 9: Prove the crate stands alone

The point of the split is that a consumer can render a recipe **without** `cookcli-core`. Nothing so far tests that — the workspace always builds both. This test does, and it is written failing-first.

**Files:**
- Create: `crates/format/tests/standalone.rs`

- [ ] **Step 1: Write the failing test**

```rust
//! The crate's promise: render a recipe using nothing but `cooklang-format`.
//!
//! An integration test, so it sees only the public API — if something a
//! consumer needs is private or unexported, this fails to compile, which is
//! exactly the failure worth catching.

use cooklang_format::cooklang::{Converter, CooklangParser, Extensions, Recipe};
use cooklang_format::{markdown_to_string, PaperSize, Style};

const RECIPE: &str = "---\ntitle: Tea\n---\n\nBoil @water{2%cups} in a #pot for ~{5%minutes}.\n";

fn parse(parser: &CooklangParser) -> Recipe {
    let (recipe, _) = parser
        .parse(RECIPE)
        .into_result()
        .expect("the fixture parses");
    recipe
}

#[test]
fn renders_markdown_without_cookcli_core() {
    let parser = CooklangParser::new(Extensions::empty(), Converter::default());
    let recipe = parse(&parser);

    let md = markdown_to_string(&recipe, "Tea", 1.0, parser.converter()).expect("formats");

    assert!(md.contains("water"), "ingredient missing from:\n{md}");
    assert!(md.contains("pot"), "cookware missing from:\n{md}");
}

#[test]
fn renders_every_format_the_crate_advertises() {
    let parser = CooklangParser::new(Extensions::empty(), Converter::default());
    let recipe = parse(&parser);
    let conv = parser.converter();

    let mut buf = Vec::new();
    cooklang_format::human::print_human(&recipe, "Tea", 1.0, conv, Style::Plain, &mut buf)
        .expect("human");
    assert!(!buf.is_empty(), "human output is empty");

    let mut buf = Vec::new();
    cooklang_format::latex::print_latex(&recipe, "Tea", 1.0, conv, PaperSize::A4, &mut buf)
        .expect("latex");
    assert!(!buf.is_empty(), "latex output is empty");

    let mut buf = Vec::new();
    cooklang_format::typst::print_typst(&recipe, "Tea", 1.0, conv, PaperSize::A4, &mut buf)
        .expect("typst");
    assert!(!buf.is_empty(), "typst output is empty");

    let mut buf = Vec::new();
    cooklang_format::cooklang::print_cooklang(&recipe, &mut buf).expect("cooklang");
    assert!(!buf.is_empty(), "cooklang output is empty");
}
```

> **Note on signatures:** the `print_latex`, `print_typst`, `print_human` and `print_cooklang` argument lists above are taken from `src/recipe/read.rs:206-258`, which is the CLI's live call site. If a signature differs, match the call site — do not change the formatter.

- [ ] **Step 2: Run it to see where it stands**

Run: `cargo test -p cooklang-format --test standalone`
Expected: PASS if Tasks 5–8 are complete. If it fails to compile with "unresolved import" or "private", the missing item is not exported from `lib.rs` — fix `lib.rs`, not the test.

- [ ] **Step 3: Prove it really is standalone**

Run: `cargo tree -p cooklang-format -e normal | grep -c cookcli`
Expected: `0`. A non-zero count means the new crate depends on core, which defeats the split.

- [ ] **Step 4: Commit**

```bash
git add crates/format/tests/standalone.rs
git commit -m "test(format): prove the crate renders without cookcli-core"
```

---

### Task 10: README, attribution and doctest

Three of the moved files are derived from `Zheoni/cooklang-chef` (MIT, © 2023 Francisco J. Sanchez) and carry its header. This crate is now the only place they live, so the notice belongs on its front page.

**Files:**
- Create: `crates/format/README.md`
- Modify: `crates/format/src/lib.rs`

- [ ] **Step 1: Write `crates/format/README.md`**

````markdown
# cooklang-format

Render [Cooklang](https://cooklang.org) recipes into text formats.

Extracted from [CookCLI](https://github.com/cooklang/cookcli), where these
formatters back `cook recipe -f <format>`. They are published separately so
other projects can render recipes without depending on the CLI's internals.

| module | output |
|---|---|
| `markdown` | Markdown |
| `human` | terminal text, optionally ANSI-styled |
| `cooklang` | Cooklang source (round-trips) |
| `latex`, `typst` | typeset documents, paper-size aware |
| `schema` | JSON / YAML |

## Usage

```rust
use cooklang_format::cooklang::{Converter, CooklangParser, Extensions};
use cooklang_format::markdown_to_string;

let parser = CooklangParser::new(Extensions::empty(), Converter::default());
let source = "---\ntitle: Tea\n---\n\nBoil @water{2%cups} in a #pot.\n";
let (recipe, _) = parser.parse(source).into_result().unwrap();

let markdown = markdown_to_string(&recipe, "Tea", 1.0, parser.converter()).unwrap();
assert!(markdown.contains("water"));
```

Each formatter also has a `print_*` form that writes into a `std::io::Write`,
so a caller already holding a file or socket does not pay for a second copy of
the document.

## Colour

`Style::Plain` is the default and emits no escape codes. Colour is passed
explicitly rather than through `yansi`'s global switch, so a library consumer
cannot have escape sequences appear in a file it is writing.

## License

MIT. See [LICENSE](LICENSE).

Some source files include code from
[cooklang-chef](https://github.com/Zheoni/cooklang-chef), also under MIT
license.
````

- [ ] **Step 2: Compile the README as a doctest**

Append to `crates/format/src/lib.rs`, mirroring what `cookcli-core` does:

```rust
/// Compiles `README.md`'s example as a doctest, so the crate's front page
/// cannot rot into something that no longer builds.
///
/// Exists only under `cfg(doctest)`, so it is not part of the public API and
/// does not appear in the rendered documentation.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
```

- [ ] **Step 3: Run the doctest**

Run: `cargo test -p cooklang-format --doc`
Expected: `test result: ok.` with at least 1 test. If the example fails, fix the **README** to match the real API.

- [ ] **Step 4: Commit**

```bash
git add crates/format/README.md crates/format/src/lib.rs
git commit -m "docs(format): add README with attribution, compiled as a doctest"
```

---

### Task 11: Wire the release

**Files:**
- Modify: `.github/workflows/release.yaml` (the `publish_crates` job)

- [ ] **Step 1: Verify the package is publishable**

Run: `cargo publish --dry-run -p cooklang-format --allow-dirty`
Expected: `Packaged N files` and no error. A "missing field" or "dependency not published" error must be fixed here, not in CI.

- [ ] **Step 2: Add the publish step**

In `.github/workflows/release.yaml`, insert **before** the existing `Publish | crates.io (cookcli-core)` step, reusing its version-probe guard:

```yaml
      # The chain is cooklang-format -> cookcli-core -> cookcli: crates.io
      # rejects a package whose dependency is not already published, so each
      # link goes up only after the one it depends on is resolvable from the
      # registry. The guard is for a re-run of an already-published release —
      # see the note on the cookcli-core step below.
      - name: Publish | crates.io (cooklang-format)
        run: |
          set -euo pipefail
          version=$(cargo metadata --no-deps --format-version 1 \
            | jq -r '.packages[] | select(.name == "cooklang-format") | .version')
          echo "cooklang-format version: $version"
          status=$(curl -sS -o /dev/null -w '%{http_code}' \
            -H 'User-Agent: cookcli-release (https://github.com/cooklang/cookcli)' \
            "https://crates.io/api/v1/crates/cooklang-format/$version")
          case "$status" in
            200)
              echo "cooklang-format $version is already on crates.io; skipping."
              ;;
            404)
              cargo publish -p cooklang-format --allow-dirty
              ;;
            *)
              echo "Unexpected status $status from crates.io; refusing to guess." >&2
              exit 1
              ;;
          esac
        env:
          CARGO_REGISTRY_TOKEN: ${{ steps.auth.outputs.token }}
```

- [ ] **Step 3: Check the YAML parses**

Run: `python3 -c "import yaml,sys; d=yaml.safe_load(open('.github/workflows/release.yaml')); print([s['name'] for s in d['jobs']['publish_crates']['steps']])"`
Expected: the printed list shows `Publish | crates.io (cooklang-format)` immediately before `Publish | crates.io (cookcli-core)`.

- [ ] **Step 4: Record the release-please caveat**

Add this comment directly above the new step, since it applies to the whole chain:

```yaml
      # release-please's `rust` strategy bumps every workspace member to the
      # release version. It must also rewrite the `version` requirement on the
      # path dependencies (cookcli -> cookcli-core -> cooklang-format); if it
      # does not, publishing fails here on an unresolvable dependency rather
      # than silently shipping something wrong. Check the release PR's diff the
      # first time this runs.
```

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/release.yaml
git commit -m "ci: publish cooklang-format ahead of cookcli-core"
```

---

### Task 12: Documentation and final verification

**Files:**
- Modify: `CLAUDE.md` (Workspace Structure section)

- [ ] **Step 1: Update the workspace description**

In `CLAUDE.md`, under `### Workspace Structure`, the list currently reads:

```markdown
- `cookcli` (this crate) - The CLI application
```

Add beneath it, above the `../cooklang-rs` entry:

```markdown
- `crates/core` (`cookcli-core`) - Command logic as a library
- `crates/format` (`cooklang-format`) - Recipe output formatters, published for reuse
```

- [ ] **Step 2: Run the full pre-PR check from CLAUDE.md**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all three clean. `cargo fmt --check` silent, clippy no warnings, tests 22 suites green.

- [ ] **Step 3: Confirm the refactor changed no output**

Run: `git diff --stat main -- tests/snapshots src/`
Expected: **empty**. Not one snapshot and not one line of `src/` should have changed — the shim exists precisely so the CLI is untouched. If `src/` shows changes, the shim is incomplete.

- [ ] **Step 4: Confirm the dependency shed actually happened**

Run: `cargo tree -p cooklang-format -e normal --depth 1`
Expected: `cooklang`, `anstream`, `anstyle`, `anstyle-yansi`, `humantime`, `regex`, `serde`, `serde_json`, `serde_yaml`, `tabular`, `textwrap`, `yansi` — and none of `cooklang-find`, `cooklang-reports`, `directories`, `toml_edit`, `chrono`, `camino`, `thiserror`, `tracing`.

- [ ] **Step 5: Commit and open the PR**

```bash
git add CLAUDE.md
git commit -m "docs: describe the crates/format workspace member"
gh pr create --title "feat(format): publish the recipe converters as cooklang-format" \
  --body "$(cat <<'EOF'
Closes #443.

Extracts the recipe output converters from `cookcli-core` into a new
`cooklang-format` crate, so other projects can render Cooklang recipes without
depending on the CLI's internals or paying for recipe discovery and report
templating.

`cooklang-to-md`, `cooklang-to-human` and `cooklang-to-cooklang` are already
taken by Zheoni/cooklang-chef — which is also where three of these files came
from — so the crate takes a single neutral name rather than standing derived
work beside its origin under near-identical names.

`cookcli-core` keeps `pub mod format` as a re-export shim: no call site in the
CLI, server or web builders changed, and no snapshot moved.

Design: `docs/superpowers/specs/2026-08-14-cooklang-format-crate-design.md`
EOF
)"
```

---

## Done when

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` are clean, and the green check reports `failures: 0` with `passed:` at or above 760.
- `git diff main -- tests/snapshots src/` is empty.
- `cargo publish --dry-run -p cooklang-format` succeeds.
- `cargo tree -p cooklang-format -e normal | grep cookcli` finds nothing.
