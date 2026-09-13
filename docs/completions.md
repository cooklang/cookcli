# Completions Command

Generate a shell completion script so your shell can tab-complete `cook` subcommands, aliases and flags.

## Usage

```
cook completions <SHELL>
```

The script is printed to stdout. Redirect it into the place your shell loads completions from, or source it from your shell configuration.

## Arguments

| Argument | Description |
|----------|-------------|
| `<SHELL>` | One of `bash`, `zsh`, `fish`, `powershell`, `elvish` |

## Installation

### Bash

```bash
# Linux
cook completions bash > /etc/bash_completion.d/cook

# macOS (Homebrew bash-completion)
cook completions bash > "$(brew --prefix)/etc/bash_completion.d/cook"

# Or, without bash-completion, source it from ~/.bashrc
echo 'source <(cook completions bash)' >> ~/.bashrc
```

### Zsh

```bash
# Into a directory on your $fpath, then restart the shell
cook completions zsh > "${fpath[1]}/_cook"

# Or into a user-owned directory
mkdir -p ~/.zfunc
cook completions zsh > ~/.zfunc/_cook
# ...and in ~/.zshrc, before `compinit`:
#   fpath=(~/.zfunc $fpath)
#   autoload -Uz compinit && compinit
```

### Fish

```bash
cook completions fish > ~/.config/fish/completions/cook.fish
```

### PowerShell

```powershell
# Append to your profile
cook completions powershell >> $PROFILE

# Or load it on demand
cook completions powershell | Out-String | Invoke-Expression
```

### Elvish

```bash
cook completions elvish >> ~/.config/elvish/rc.elv
```

## Notes

- The script is generated from the running binary, so it matches that binary exactly. Subcommands compiled out with `--no-default-features` (such as `server`, `import`, `lsp` or `update`) are missing from the script too.
- Regenerate the script after updating CookCLI so new subcommands and flags show up.
- Completion is static: it knows subcommands and flags, but does not complete recipe file names or `recipe.cook:2` scaling suffixes.
