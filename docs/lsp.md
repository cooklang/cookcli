# LSP Command

The `lsp` command starts a Language Server Protocol (LSP) server for Cooklang recipe files. It enables IDE features in text editors and development environments.

## Overview

The LSP server provides intelligent editing features for `.cook` files:

- **Real-time syntax checking** – Catch errors as you type
- **Auto-completion** – Suggestions for ingredients, cookware, and timers
- **Semantic highlighting** – Rich syntax coloring beyond basic highlighting
- **Hover documentation** – Information about ingredients and references
- **Document symbols** – Navigate recipe structure quickly
- **Go to definition** – Jump to referenced recipes

## Basic Usage

Start the LSP server:
```bash
cook lsp
```

The server communicates over stdin/stdout using the standard LSP protocol. You typically don't run this command directly—your editor starts it automatically.

## Editor Integration

### Visual Studio Code

Install the [Cooklang extension](https://marketplace.visualstudio.com/items?itemName=cooklang.cooklang) from the VS Code marketplace. The extension automatically uses `cook lsp` when available.

**Manual Configuration:**

Add to your `settings.json`:
```json
{
  "cooklang.languageServer.path": "cook",
  "cooklang.languageServer.args": ["lsp"]
}
```

### Neovim

Using [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig):

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.cooklang then
  configs.cooklang = {
    default_config = {
      cmd = { 'cook', 'lsp' },
      filetypes = { 'cooklang' },
      root_dir = lspconfig.util.root_pattern('.git', 'config'),
      settings = {},
    },
  }
end

lspconfig.cooklang.setup{}
```

### Vim with CoC

Add to your `coc-settings.json`:
```json
{
  "languageserver": {
    "cooklang": {
      "command": "cook",
      "args": ["lsp"],
      "filetypes": ["cooklang"],
      "rootPatterns": [".git", "config"]
    }
  }
}
```

### Emacs

Using [lsp-mode](https://emacs-lsp.github.io/lsp-mode/):

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration
    '(cooklang-mode . "cooklang"))

  (lsp-register-client
    (make-lsp-client
      :new-connection (lsp-stdio-connection '("cook" "lsp"))
      :activation-fn (lsp-activate-on "cooklang")
      :server-id 'cooklang-lsp)))
```

Using [eglot](https://github.com/joaotavora/eglot):

```elisp
(add-to-list 'eglot-server-programs
  '(cooklang-mode . ("cook" "lsp")))
```

### Helix

Add to your `languages.toml`:
```toml
[[language]]
name = "cooklang"
scope = "source.cooklang"
file-types = ["cook"]
language-servers = ["cooklang-lsp"]

[language-server.cooklang-lsp]
command = "cook"
args = ["lsp"]
```

### Sublime Text

Using [LSP package](https://github.com/sublimelsp/LSP):

Add to LSP settings:
```json
{
  "clients": {
    "cooklang": {
      "enabled": true,
      "command": ["cook", "lsp"],
      "selector": "source.cooklang"
    }
  }
}
```

### Zed

Add to your `settings.json`:
```json
{
  "lsp": {
    "cooklang": {
      "binary": {
        "path": "cook",
        "arguments": ["lsp"]
      }
    }
  }
}
```

## Features

### Diagnostics

The LSP server reports syntax errors and warnings in real-time:

```
Line 5: Invalid ingredient syntax - missing quantity
Line 8: Timer format should be ~{time%unit}
Line 12: Referenced recipe not found: ./Missing Recipe.cook
```

### Completions

Auto-completion triggers in various contexts:

- **Ingredients** – After typing `@`, suggests known ingredients
- **Cookware** – After typing `#`, suggests equipment
- **Timers** – After typing `~`, suggests time formats
- **References** – Suggests recipe files for `@./` paths
- **Metadata** – Suggests common metadata keys in frontmatter

### Hover Information

Hover over elements to see details:

- **Ingredients** – Quantity, unit, and any notes
- **Timers** – Formatted duration
- **Recipe references** – Preview of referenced recipe
- **Metadata** – Description of metadata fields

### Document Symbols

Navigate recipe structure:

- Sections and steps
- Ingredients list
- Cookware list
- Timers
- Metadata fields

### Semantic Tokens

Enhanced syntax highlighting for:

- Ingredients (with quantity and unit distinction)
- Cookware
- Timers
- Comments
- Metadata keys and values
- Recipe references

## Troubleshooting

### Server Not Starting

Verify the cook command is in your PATH:
```bash
which cook
cook --version
```

### Connection Issues

Check server logs by running manually:
```bash
cook lsp 2>/tmp/cooklang-lsp.log
```

### Editor Not Detecting Server

Ensure your editor:
1. Recognizes `.cook` files as Cooklang
2. Has LSP client support enabled
3. Is configured with the correct command path

### Debugging

Enable verbose logging:
```bash
RUST_LOG=debug cook lsp 2>/tmp/cooklang-lsp-debug.log
```

## Protocol Details

The server implements LSP specification features:

| Feature | Support |
|---------|---------|
| textDocument/didOpen | Yes |
| textDocument/didChange | Yes |
| textDocument/didClose | Yes |
| textDocument/completion | Yes |
| textDocument/hover | Yes |
| textDocument/definition | Yes |
| textDocument/documentSymbol | Yes |
| textDocument/semanticTokens | Yes |
| textDocument/publishDiagnostics | Yes |

### Transport

- **Protocol**: JSON-RPC 2.0
- **Transport**: stdin/stdout
- **Encoding**: UTF-8

## See Also

- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [Cooklang Syntax Reference](https://cooklang.org/docs/spec/)
- [Editor Integrations](https://cooklang.org/cli/editors/)
