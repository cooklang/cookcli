# cooklang-format: the output converters as a publishable crate

**Date:** 2026-08-14
**Status:** Approved, ready for planning
**Issue:** [#443](https://github.com/cooklang/cookcli/issues/443) — "Also publish as crates"

## Problem

CookCLI carries a set of converters that turn a parsed `cooklang::Recipe` into
Markdown, LaTeX, Typst, schema.org JSON-LD, terminal-styled text, and back
into Cooklang source. Other projects want them — the issue names them directly:
"we have some converters like cooklang to markdown etc which can be reused in
other projects".

Today they are reachable only as `cookcli_core::format::*`. Two things make
that a poor answer:

- **Discoverability.** The crate is named for the CLI it was extracted from.
  Nobody searching crates.io for a Cooklang Markdown renderer finds
  `cookcli-core`.
- **Dependency weight.** `cookcli-core` pulls `cooklang-find`,
  `cooklang-reports`, `directories`, `toml_edit`, `chrono`, `camino`,
  `thiserror` and `tracing`. A consumer that wants to render one recipe to
  Markdown pays for recipe discovery, report templating and config-file
  editing.

There is also a naming hazard. `Zheoni/cooklang-chef` already publishes
`cooklang-to-md`, `cooklang-to-human` and `cooklang-to-cooklang` (all 0.15.0,
last updated 2025-01-14) — exactly the three names the obvious scheme would
want. Worse, three of our converters are *derived from* cooklang-chef and carry
its MIT header. Publishing them under near-identical names beside their origin
would be actively confusing.

State at the time of writing: `cookcli` 0.33.1 is on crates.io; `cookcli-core`
0.1.0 exists in the workspace (PR #434) but has never been published, because
no release has run since it was added. The `publish_crates` job in
`release.yaml` already handles both.

## Decisions

| # | Decision | Choice |
|---|---|---|
| 1 | Granularity | One crate for all converters, not one crate per converter |
| 2 | Name | `cooklang-format` |
| 3 | Shopping-list formatter | Stays in `cookcli-core` |
| 4 | `REFERENCE_SEPARATOR` | Moves to `cooklang-format`, re-exported by core |
| 5 | `PARSER` | Stays in core; the new crate builds its own for tests |
| 6 | Downstream compatibility | Core keeps `pub mod format` as a shim; no call site changes |
| 7 | Versioning | Lockstep with the workspace (debuts at 0.33.x, not 0.1.0) |

### 1. Granularity

One crate. Rejected: one crate per converter (`cooklang-to-latex`,
`cooklang-to-typst`, …), which is the truest reading of the issue title but
fails on availability — three of the six natural names belong to cooklang-chef,
so the set would come out mixed (`cooklang-to-latex` ours, `cooklang-to-md`
theirs) and six crates would need lockstep versioning on every release.

Also rejected: publishing `cookcli-core` as-is and calling the issue closed.
Zero work, but it addresses neither the discoverability nor the dependency
weight that motivate the issue.

### 2. Name

`cooklang-format`, verified free on crates.io along with `cooklang-formats`,
`cooklang-render`, `cooklang-output`, `cooklang-convert` and `cooklang-export`.

A neutral name avoids standing a derived work next to its origin under a
near-identical name. See §8 on attribution.

### 3. What moves and what stays

Moves to `crates/format` (8 files, ~3,400 lines):

| file | contents |
|---|---|
| `format/mod.rs` → `lib.rs` | `Style`, `PaperSize`, `human_to_string`, `markdown_to_string` |
| `human.rs` | terminal rendering; ANSI via explicit `Style`, never yansi's global switch |
| `markdown.rs` | Markdown |
| `cooklang.rs` | round-trip Cooklang source |
| `latex.rs`, `typst.rs` | typeset output, paper-size aware |
| `schema.rs` | schema.org/Recipe JSON-LD |
| `number.rs`, `quantity.rs` | shared number and quantity primitives |

Stays in `cookcli-core`:

- **`format/shopping_list.rs`.** It renders core's `AggregatedList`. Moving it
  would invert the dependency between the two crates. It keeps its path,
  `cookcli_core::format::shopping_list`.
- **`parser::PARSER`.** The moving files reference it only from their own
  tests, and it is a one-liner
  (`CooklangParser::new(Extensions::empty(), Converter::default())`), so the
  new crate constructs its own under `#[cfg(test)]` rather than depending on
  core.

**Cycle check.** `core::shopping_list` uses `format::quantity::ordered_components`
(moving) *and* `format::shopping_list::quantity_fmt` (staying). After the split
every arrow runs core → format. No cycle.

### 4. `REFERENCE_SEPARATOR`

Currently `cookcli_core::find::REFERENCE_SEPARATOR`, a `pub const &str = "/"`.
It is a spelling rule for how a recipe reference is *written* — the Cooklang
writer re-emits it as source where a backslash is invalid syntax, and the
Markdown writer puts it in a link target — and five of the moving files use it.
It moves to `cooklang-format`; core re-exports it so
`cookcli_core::REFERENCE_SEPARATOR` keeps working at its three call sites in
`src/server/handlers/` and `src/web/builders.rs`, as do the two uses inside
`core::shopping_list`.

### 5. Dependencies

The new crate needs: `cooklang` (`default-features = false`, same feature set —
`bundled_units` stays off, per #433), `anstream`, `anstyle`, `anstyle-yansi`,
`humantime`, `regex`, `serde`, `serde_json`, `serde_yaml`, `tabular`,
`textwrap`, `yansi`.

It sheds `cooklang-find`, `cooklang-reports`, `directories`, `toml_edit`,
`chrono`, `camino`, `thiserror` and `tracing` — `camino` is used only by the
shopping-list formatter, which stays.

Every shared dependency declaration is kept in step with the root and core
manifests, following the convention those manifests already document, so all
three resolve the same versions and feature unification holds no surprises.

### 6. Downstream compatibility

No call site changes anywhere. Core keeps `pub mod format` as a shim:

```rust
pub mod format {
    pub use cooklang_format::*;
    pub mod shopping_list;  // stays here
}
```

That preserves `cookcli_core::format::human::print_human`,
`cookcli_core::format::Style`, `cookcli_core::format::shopping_list::*` and the
`pub use format::{PaperSize, Style}` at the crate root. `src/util/mod.rs`'s
`pub use cookcli_core::format;` is untouched, so every
`crate::util::format::..` reference in `src/recipe/read.rs`,
`src/shopping_list.rs`, `src/server/handlers/` and `src/web/builders.rs`
compiles unchanged.

### 7. Versioning and release

The repo has no release-please config file, so the `rust` strategy bumps every
workspace member to the release version in lockstep. `cooklang-format`
therefore debuts at 0.33.x alongside `cookcli-core`, rather than at 0.1.0.

Accepted deliberately: it is the behaviour already in place, and carving out
independent versions would mean introducing a release-please config and
manifest. The cost is a general-purpose crate whose version number tracks a
CLI.

Publishing is a three-step chain, each step depending on the one before:

```
cooklang-format  →  cookcli-core  →  cookcli
```

The `publish_crates` job gains a `cooklang-format` step ahead of the existing
two, reusing the crates.io version-probe guard already there (HTTP 200 → already
published, skip; 404 → publish; anything else → fail rather than guess). Core's
manifest declares the dependency in the `version` + `path` form the workspace
already uses, so local builds take the workspace copy and the published crate
resolves from the registry.

### 8. Licensing and attribution

`cooklang.rs`, `human.rs` and `markdown.rs` are derived from
`Zheoni/cooklang-chef` and carry its MIT header (© 2023 Francisco J. Sanchez).
Those per-file headers travel with the files.

Because the new crate becomes the only place those three files live, the notice
must also appear on its front page. Mirroring what `crates/core` already does,
the crate gets its own `LICENSE` (MIT, matching root) and a `README.md` whose
licence section carries the sentence the root README already uses: *"Some
source files include code from cooklang-chef, also under MIT license."*

### 9. Testing

- The inline `#[cfg(test)] mod tests` blocks move with their files.
- The insta snapshot suites pinned in `61bd8ee` (`latex`, `typst`, `cooklang`
  output) stay at the top level and act as the cross-crate regression net: they
  drive the CLI end to end, so they catch any rendering drift the move
  introduces.
- `cargo publish --dry-run -p cooklang-format` before the release path is
  trusted.
- `cargo fmt`, `cargo clippy` and `cargo test` clean, per CLAUDE.md.

## Out of scope

- Splitting any further crates out of `cookcli-core`.
- Independent versioning for workspace members.
- Any change to what the converters emit. This is a move, not a rewrite —
  the snapshots must not change.
