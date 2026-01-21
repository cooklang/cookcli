# Recipe Editor for Web UI

## Overview

Add edit mode for recipes and menu files in the CookCLI web UI with syntax highlighting and autocomplete powered by the existing cooklang-language-server via LSP over WebSocket.

## Goals

- Full recipe authoring experience in the browser
- Syntax highlighting for Cooklang format
- Autocomplete for ingredients (from aisle.conf), cookware, units
- Real-time diagnostics (syntax errors, warnings)
- Toggle between edit and preview modes

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Browser                                                     │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  CodeMirror 6 Editor                                    ││
│  │  - cooklang mode (ported from obsidian plugin)          ││
│  │  - LSP client adapter for completions/diagnostics       ││
│  └────────────────────────┬────────────────────────────────┘│
└───────────────────────────┼─────────────────────────────────┘
                            │ WebSocket (JSON-RPC)
┌───────────────────────────┼─────────────────────────────────┐
│  CookCLI Server           │                                  │
│  ┌────────────────────────▼────────────────────────────────┐│
│  │  WebSocket Handler (/ws/lsp)                            ││
│  │  - Bridges WebSocket ↔ LSP stdio                        ││
│  └────────────────────────┬────────────────────────────────┘│
│                           │ stdio                            │
│  ┌────────────────────────▼────────────────────────────────┐│
│  │  cooklang-language-server (subprocess)                  ││
│  │  - Completions, diagnostics, hover                      ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

## Backend Components

### New Routes

| Method | Path | Description |
|--------|------|-------------|
| GET | `/edit/{path}` | Editor page for existing recipe |
| GET | `/new` | Editor page for new recipe |
| GET | `/api/recipe/{path}/raw` | Raw file content as text |
| PUT | `/api/recipe/{path}` | Save edited content |
| POST | `/api/recipe` | Create new recipe file |
| WS | `/ws/lsp` | WebSocket LSP bridge |

### WebSocket LSP Bridge (`src/server/lsp_bridge.rs`)

- On connect: spawn `cooklang-language-server` as child process
- Forward: WebSocket JSON → LSP stdin, LSP stdout → WebSocket JSON
- Set workspace folder to `base_path` during LSP initialize
- Clean up subprocess on disconnect
- One LSP process per WebSocket connection (simplifies state)

### File Operations

- **Read**: `fs::read_to_string`
- **Write**: Validate path within `base_path`, atomic write (temp + rename)
- **Create**: Generate safe filename from title or user input

## Frontend Components

### Editor Page Layout

```
┌─────────────────────────────────────────────────────────────┐
│  [← Back]  Recipe Name                    [Preview] [Save]  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  CodeMirror Editor (full height)                            │
│  - Line numbers                                             │
│  - Syntax highlighting                                      │
│  - Autocomplete popup                                       │
│  - Error squiggles                                          │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  Status: Connected ●  |  Line 12, Col 8                     │
└─────────────────────────────────────────────────────────────┘
```

### JavaScript Modules

1. **`editor.js`** - CodeMirror setup, save/discard/preview handlers
2. **`lsp-client.js`** - WebSocket connection, LSP message handling
3. **`cooklang-mode.js`** - Ported from `cooklang-obsidian/src/mode/cook/cook.ts`

### Preview Mode

Toggle button switches between:
- Edit: CodeMirror editor
- Preview: Rendered recipe (fetch from existing `/recipe/{path}` endpoint)

## Dependencies

### Rust (Cargo.toml)
- WebSocket support already available via `axum` with `ws` feature

### JavaScript (package.json)
```json
{
  "@codemirror/autocomplete": "^6.x",
  "@codemirror/commands": "^6.x",
  "@codemirror/language": "^6.x",
  "@codemirror/state": "^6.x",
  "@codemirror/view": "^6.x",
  "@codemirror/search": "^6.x"
}
```

## Implementation Phases

### Issue 1: Basic Editor Infrastructure
- Routes: `/edit/{path}`, `/api/recipe/{path}/raw`, `PUT /api/recipe/{path}`
- Editor template with simple textarea
- Save/load functionality
- "Edit" button on recipe detail page

### Issue 2: CodeMirror Integration
- Port `cook.ts` to vanilla JS
- Replace textarea with CodeMirror 6
- Syntax highlighting
- Update npm dependencies and build

### Issue 3: LSP WebSocket Bridge
- WebSocket endpoint `/ws/lsp`
- Spawn/manage LSP subprocess
- JSON-RPC message proxying
- Connection lifecycle

### Issue 4: LSP Client & Full Features
- CodeMirror LSP client integration
- Autocomplete from LSP
- Diagnostics display
- Preview toggle
- New recipe creation (`/new`)

## Security Considerations

- Path traversal prevention: all paths validated within `base_path`
- Atomic writes prevent file corruption
- No execution of user content

## Existing Code to Reuse

- **cooklang-obsidian**: CodeMirror mode (`src/mode/cook/cook.ts`)
- **cooklang-language-server**: Full LSP implementation
- **CookCLI**: `cook lsp` command shows LSP integration pattern

## Future Enhancements (Out of Scope)

- Collaborative editing
- Version history / undo across sessions
- Image upload for recipe photos
- Drag-and-drop ingredient reordering
