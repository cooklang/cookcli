# Update Command

Update CookCLI to the latest version.

## Usage

```
cook update [OPTIONS]
```

Alias: `cook u`

## Options

| Option | Description |
|--------|-------------|
| `--check-only` | Check for updates without installing |
| `--force` | Force update even if already on the latest version |

## Examples

```bash
# Update to latest version
cook update

# Check for updates without installing
cook update --check-only

# Force reinstall
cook update --force
```

## Notes

- Downloads from GitHub releases over HTTPS
- Automatically detects your platform and architecture, and installs the
  matching `cook-<version>-<os>-<arch>[-<libc>]` archive after checking it
  against its `.sha256`
- CookCLI 0.38 and older look for the archives under their former names
  (`cook-x86_64-unknown-linux-musl.tar.gz`, …), so their `cook update` stops at
  "No asset found". Download the new version once from the
  [releases page](https://github.com/cooklang/cookcli/releases); from then on
  `cook update` works again
- May require `sudo` if installed in a system directory (e.g., `/usr/local/bin/`)
- Verify with `cook --version` after updating
