# Shopping List Pantry Flags — Design

## Background

The `cook shopping-list` CLI command already loads a pantry file via `Context::pantry()` (`src/main.rs:104`) and subtracts pantry quantities from the generated shopping list (`src/shopping_list.rs:264-267`). However, two ergonomic gaps remain compared to the aisle handling and compared to the web server:

1. No way to specify a custom pantry file path on the command line. The aisle flag (`--aisle <path>`) supports this; pantry does not.
2. No way to disable pantry subtraction. If a user wants the full unfiltered shopping list, they must move or delete their pantry file.

This spec closes both gaps.

## Goals

- Add `--pantry <path>` argument to `cook shopping-list`, mirroring the existing `--aisle <path>`.
- Add `--ignore-pantry` boolean flag that completely skips pantry loading and subtraction.

## Non-Goals

- No changes to pantry parsing, subtraction logic, or the underlying `cooklang::pantry` API.
- No changes to the web server pantry handling.
- No changes to the `pantry` subcommand or other commands.

## Design

### CLI Surface

Add two fields to `ShoppingListArgs` in `src/shopping_list.rs`:

```rust
/// Load pantry conf file
#[arg(long)]
pantry: Option<Utf8PathBuf>,

/// Don't subtract pantry items from the shopping list
#[arg(long)]
ignore_pantry: bool,
```

Notes:
- `--pantry` is long-only (no short flag) to avoid colliding with `-p` / `--plain`.
- `--ignore-pantry` follows the naming pattern of the existing `--ignore-references` flag.

### Resolution Logic

Replace the current pantry loading block (`src/shopping_list.rs:198-233`) with logic that mirrors the aisle resolution pattern at `src/shopping_list.rs:168-175`:

1. If `args.ignore_pantry` is `true`: skip loading entirely. `pantry` is `None`. No file I/O occurs.
2. Otherwise, resolve the path as `args.pantry.or_else(|| ctx.pantry())`.
3. If a path is resolved, load and parse as today (parse_lenient, surface warnings, rebuild index).
4. If no path is resolved, behave as today (no pantry, no subtraction).

The existing subtraction call at `src/shopping_list.rs:264-267` is unchanged — it already short-circuits when `pantry` is `None`.

### Interaction Matrix

| `--ignore-pantry` | `--pantry <path>` | `ctx.pantry()` resolves | Behavior                                  |
| ----------------- | ----------------- | ----------------------- | ----------------------------------------- |
| true              | (any)             | (any)                   | No load, no subtraction                   |
| false             | Some(path)        | (any)                   | Load from `path`, subtract                |
| false             | None              | Some(auto)              | Load from auto path, subtract (unchanged) |
| false             | None              | None                    | No load, no subtraction (unchanged)       |

When both `--ignore-pantry` and `--pantry` are passed, `--ignore-pantry` wins. The provided path is not opened. This matches the user's stated intent ("skip pantry items") and avoids surprising failures from a bad path that the user explicitly asked to ignore.

## Testing

Manual verification using `cook seed` fixtures:

1. Default behavior unchanged: `cook shopping-list <recipe>` still subtracts auto-discovered pantry.
2. Custom path: create a pantry file at a non-default location, run `cook shopping-list --pantry <path> <recipe>`, verify subtraction uses the custom file.
3. Skip flag: with an auto-discoverable pantry file present, run `cook shopping-list --ignore-pantry <recipe>` and verify no items are subtracted.
4. Skip flag dominates: run `cook shopping-list --ignore-pantry --pantry /nonexistent/path <recipe>` and verify it succeeds with no subtraction (does not error on the bad path).

## Out of Scope / Future Work

- Documentation update in `docs/shopping-list.md` to mention the new flags.
- Mirroring the web server's reporting of which items were filtered out (the web UI shows pantry-subtracted items separately; the CLI does not).
