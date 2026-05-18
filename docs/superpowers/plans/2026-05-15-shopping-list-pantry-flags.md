# Shopping List Pantry Flags Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--pantry <path>` and `--ignore-pantry` flags to the `cook shopping-list` command so users can specify a custom pantry file or skip pantry subtraction entirely.

**Architecture:** Two new clap arguments on `ShoppingListArgs`. Pantry resolution becomes `if args.ignore_pantry { None } else { args.pantry.or_else(|| ctx.pantry()) }`. The existing parse/subtract logic is otherwise unchanged.

**Tech Stack:** Rust, clap 4 (derive API), `camino::Utf8PathBuf`, `cooklang::pantry::parse_lenient`.

**Note on testing:** Per `CLAUDE.md`, this project has no automated test suite. Verification is manual using `cargo run` against the `seed/` recipes, which include a `seed/config/pantry.conf`.

---

## File Structure

Only one source file is modified:

- Modify: `src/shopping_list.rs` — add two `ShoppingListArgs` fields and rewrite the pantry-loading block to honor them.

No new files. No changes to other modules: `Context::pantry()` (`src/main.rs:104`) is still used as the auto-discovery fallback, and `list.subtract_pantry(...)` at `src/shopping_list.rs:264-267` is unchanged.

---

### Task 1: Add `--pantry` and `--ignore-pantry` args and wire them into pantry resolution

**Files:**
- Modify: `src/shopping_list.rs:98-104` (add new args next to existing `--aisle` / `--ignore-references`)
- Modify: `src/shopping_list.rs:198-233` (rewrite pantry loading block)

- [ ] **Step 1: Add the two new clap fields to `ShoppingListArgs`**

In `src/shopping_list.rs`, locate this block (around lines 98-104):

```rust
    /// Load aisle conf file
    #[arg(short, long)]
    aisle: Option<Utf8PathBuf>,

    /// Don't expand referenced recipes
    #[arg(short, long)]
    ignore_references: bool,
```

Replace it with:

```rust
    /// Load aisle conf file
    #[arg(short, long)]
    aisle: Option<Utf8PathBuf>,

    /// Load pantry conf file
    #[arg(long)]
    pantry: Option<Utf8PathBuf>,

    /// Don't expand referenced recipes
    #[arg(short, long)]
    ignore_references: bool,

    /// Don't subtract pantry items from the shopping list
    #[arg(long)]
    ignore_pantry: bool,
```

Notes:
- `--pantry` is intentionally long-only. A short `-p` would collide with the existing `-p` / `--plain` flag.
- `--ignore-pantry` mirrors the naming of the existing `--ignore-references` flag.

- [ ] **Step 2: Rewrite the pantry loading block to honor the new flags**

In `src/shopping_list.rs`, locate this block (lines 198-233):

```rust
    // Load pantry configuration if available
    let pantry_path = ctx.pantry();
    let pantry = if let Some(path) = &pantry_path {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                tracing::debug!("Loading pantry from: {}", path);
                let result = cooklang::pantry::parse_lenient(&content);

                // Check if there are any warnings to display
                if result.report().has_warnings() {
                    for warning in result.report().warnings() {
                        warn!("Pantry configuration warning: {}", warning);
                    }
                }

                let mut pantry_conf = result.output().cloned();
                if let Some(ref mut pantry) = pantry_conf {
                    pantry.rebuild_index();
                    tracing::debug!(
                        "Pantry loaded successfully with {} sections",
                        pantry.sections.len()
                    );
                } else {
                    tracing::warn!("Failed to parse pantry file");
                }
                pantry_conf
            }
            Err(e) => {
                warn!("Failed to read pantry file: {}", e);
                None
            }
        }
    } else {
        tracing::debug!("No pantry file found");
        None
    };
```

Replace it with:

```rust
    // Resolve pantry path: --ignore-pantry skips entirely; otherwise prefer
    // --pantry, falling back to ctx.pantry() auto-discovery.
    let pantry_path = if args.ignore_pantry {
        tracing::debug!("Pantry ignored via --ignore-pantry");
        None
    } else {
        args.pantry.clone().or_else(|| ctx.pantry())
    };

    let pantry = if let Some(path) = &pantry_path {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                tracing::debug!("Loading pantry from: {}", path);
                let result = cooklang::pantry::parse_lenient(&content);

                // Check if there are any warnings to display
                if result.report().has_warnings() {
                    for warning in result.report().warnings() {
                        warn!("Pantry configuration warning: {}", warning);
                    }
                }

                let mut pantry_conf = result.output().cloned();
                if let Some(ref mut pantry) = pantry_conf {
                    pantry.rebuild_index();
                    tracing::debug!(
                        "Pantry loaded successfully with {} sections",
                        pantry.sections.len()
                    );
                } else {
                    tracing::warn!("Failed to parse pantry file");
                }
                pantry_conf
            }
            Err(e) => {
                warn!("Failed to read pantry file: {}", e);
                None
            }
        }
    } else {
        tracing::debug!("No pantry file found");
        None
    };
```

The only changes are:
1. The initial `pantry_path` binding now branches on `args.ignore_pantry`.
2. When not ignoring, `args.pantry.clone()` is preferred over `ctx.pantry()`. `.clone()` is required because `args` is consumed later by `args.output`, `args.ingredients_only`, etc.

The rest of the function (subtract call at the previous line ~264, output formatting) is unchanged.

- [ ] **Step 3: Run formatting and linting**

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
```

Expected: both succeed with no output / no warnings.

- [ ] **Step 4: Run the build**

```bash
cargo build
```

Expected: clean build, no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/shopping_list.rs
git commit -m "feat(shopping-list): add --pantry and --ignore-pantry flags"
```

---

### Task 2: Manual verification against seed recipes

**Files:** (no source changes — manual exercise only)
- Read: `seed/config/pantry.conf` (contains `butter`, `milk`, `eggs`, `flour`, etc.)
- Read: `seed/Breakfast/Easy Pancakes.cook` (uses some of the pantry ingredients)

- [ ] **Step 1: Baseline — confirm default behavior still subtracts pantry**

```bash
cargo run --quiet -- shopping-list --base-path ./seed "Breakfast/Easy Pancakes.cook"
```

Expected: a categorized shopping list. Ingredients also present in `seed/config/pantry.conf` with non-zero quantity (e.g. `flour`, `milk`, `eggs`, `butter`) should be absent or reduced in the output. Note which ingredients appear / are reduced — you'll compare against the next runs.

- [ ] **Step 2: `--ignore-pantry` produces the unfiltered list**

```bash
cargo run --quiet -- shopping-list --base-path ./seed --ignore-pantry "Breakfast/Easy Pancakes.cook"
```

Expected: same recipe, but the ingredients suppressed in Step 1 (e.g. `flour`, `milk`, `eggs`, `butter`) now appear at their full recipe quantities. Confirm at least one ingredient that was missing in Step 1 is present here.

- [ ] **Step 3: `--pantry <path>` uses the custom file**

Create a minimal temporary pantry file that subtracts only one ingredient:

```bash
cat > /tmp/test-pantry.conf <<'EOF'
[pantry]
flour = { quantity = "1%kg" }
EOF
```

Then run:

```bash
cargo run --quiet -- shopping-list --base-path ./seed --pantry /tmp/test-pantry.conf "Breakfast/Easy Pancakes.cook"
```

Expected: `flour` is reduced/removed (per the custom pantry), but `milk`, `eggs`, and `butter` (which are in `seed/config/pantry.conf` but NOT in the custom file) now appear at full recipe quantities. This confirms `--pantry` overrides auto-discovery.

Clean up: `rm /tmp/test-pantry.conf`

- [ ] **Step 4: `--ignore-pantry` wins over `--pantry`**

```bash
cargo run --quiet -- shopping-list --base-path ./seed --ignore-pantry --pantry /does/not/exist.conf "Breakfast/Easy Pancakes.cook"
```

Expected: succeeds with no error (does not attempt to open `/does/not/exist.conf`) and produces the same unfiltered list as Step 2.

- [ ] **Step 5: Help text reflects the new flags**

```bash
cargo run --quiet -- shopping-list --help
```

Expected: output includes lines for `--pantry <PANTRY>` ("Load pantry conf file") and `--ignore-pantry` ("Don't subtract pantry items from the shopping list").

- [ ] **Step 6: Confirm no commit is needed**

```bash
git status
```

Expected: working tree clean (Task 1 already committed; this task is verification only). If anything was modified, investigate before continuing.

---

## Self-Review

**Spec coverage:**
- Spec goal 1: add `--pantry <path>` arg → Task 1 Step 1.
- Spec goal 2: add `--ignore-pantry` flag → Task 1 Step 1.
- Spec resolution logic (ignore wins; else `args.pantry.or_else(ctx.pantry())`) → Task 1 Step 2.
- Spec interaction matrix row "ignore wins over `--pantry`" → Task 2 Step 4.
- Spec testing section bullets 1–4 → Task 2 Steps 1–4.

**Placeholder scan:** No TBDs, no "add appropriate X", no "similar to task N". All code shown in full.

**Type consistency:** Both new args use `Option<Utf8PathBuf>` / `bool`, matching the existing `aisle` and `ignore_references` fields on the same struct. The resolution expression `args.pantry.clone().or_else(|| ctx.pantry())` returns `Option<Utf8PathBuf>`, which is what the rest of the existing block already expects.
