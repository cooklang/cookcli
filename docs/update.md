# Update Command

The `update` command allows you to automatically update CookCLI to the latest version from GitHub releases.

## Overview

CookCLI can check for new releases and automatically download and install updates. The update process:
- Checks the latest release on GitHub
- Downloads the appropriate binary for your platform
- Replaces the current executable with the new version
- Preserves all your configurations and data

## Basic Usage

Update to the latest version:
```bash
cook update
```

Check for updates without installing:
```bash
cook update --check-only
```

Force update even if already on latest version:
```bash
cook update --force
```

## Command Options

### `--check-only`
Only checks if a new version is available without downloading or installing it.

**Example:**
```bash
$ cook update --check-only
Current version: 0.16.0
Checking for updates...
New version available: 0.17.0
Run 'cook update' to install the latest version.
```

### `--force`
Forces the update process even if you're already on the latest version. Useful for reinstalling the current version or troubleshooting.

**Example:**
```bash
$ cook update --force
Current version: 0.16.0
Checking for updates...
New version available: 0.16.0
Downloading and installing version 0.16.0...
Successfully updated to version 0.16.0
```

## Platform Support

The update command automatically detects your platform and downloads the appropriate binary:

| Platform | Architecture | Binary Name |
|----------|-------------|-------------|
| macOS | Intel (x86_64) | cook-x86_64-apple-darwin |
| macOS | Apple Silicon (ARM64) | cook-aarch64-apple-darwin |
| Linux | x86_64 (glibc) | cook-x86_64-unknown-linux-gnu |
| Linux | x86_64 (musl) | cook-x86_64-unknown-linux-musl |
| Linux | i686 (musl) | cook-i686-unknown-linux-musl |
| Linux | ARM64 (musl) | cook-aarch64-unknown-linux-musl |
| Linux | ARM (musl) | cook-arm-unknown-linux-musleabihf |
| Windows | x86_64 | cook-x86_64-pc-windows-msvc |
| Windows | i686 | cook-i686-pc-windows-msvc |
| Windows | ARM64 | cook-aarch64-pc-windows-msvc |
| FreeBSD | x86_64 | cook-x86_64-unknown-freebsd |

## Permissions

The update command tries to replace the current executable in-place. Depending on where CookCLI is installed, you may need appropriate permissions:

### User Installation
If CookCLI is installed in your user directory (e.g., `~/.local/bin/`), the update should work without additional permissions:
```bash
cook update
```

### System Installation
If CookCLI is installed in a system directory (e.g., `/usr/local/bin/`), you may need to run the update with sudo:
```bash
sudo cook update
```

### Permission Errors
If the update fails due to permission issues, you'll see an error message with instructions for manual installation:
```
Error: Permission denied updating /usr/local/bin/cook
Please run with sudo or manually download from:
https://github.com/cooklang/cookcli/releases
```

## Version Checking in Doctor

The `cook doctor` command automatically checks for updates as part of its health checks:

```bash
$ cook doctor
Running all doctor checks...

=== Version Check ===
🆕 A new version (0.17.0) is available!
   Run 'cook update' to install the latest version.

=== Recipe Validation ===
...
```

## Troubleshooting

### Update Fails
If the automatic update fails:
1. Check your internet connection
2. Verify you have the necessary permissions
3. Try running with `--force` flag
4. Manually download from [GitHub Releases](https://github.com/cooklang/cookcli/releases)

### Finding Your Installation
To find where CookCLI is installed:
```bash
which cook
```

### Verifying the Update
After updating, verify the new version:
```bash
cook --version
```

## Manual Installation

If automatic updates don't work for your setup, you can always manually download the latest release:

1. Visit [CookCLI Releases](https://github.com/cooklang/cookcli/releases)
2. Download the appropriate binary for your platform
3. Extract the archive:
   - For `.tar.gz`: `tar xzf cook-*.tar.gz`
   - For `.zip`: `unzip cook-*.zip`
4. Move the binary to your desired location
5. Make it executable (Unix/Linux/macOS): `chmod +x cook`

## Security

Updates are downloaded over HTTPS directly from GitHub releases. Each release includes SHA256 checksums for verification. The update process uses the `rustls` TLS implementation for secure connections.

## Examples

### Regular Update Workflow
```bash
# Check current version
$ cook --version
cook 0.16.0 - in food we trust

# Check for updates
$ cook update --check-only
Current version: 0.16.0
Checking for updates...
New version available: 0.17.0
Run 'cook update' to install the latest version.

# Install the update
$ cook update
Current version: 0.16.0
Checking for updates...
New version available: 0.17.0
Downloading and installing version 0.17.0...
Successfully updated to version 0.17.0
Please restart cook to use the new version.

# Verify the update
$ cook --version
cook 0.17.0 - in food we trust
```

### Using Aliases
The update command can also be invoked using its alias:
```bash
cook u              # Short alias for update
cook u --check-only # Check for updates
```

## See Also

- [`cook doctor`](doctor.md) - Check recipe collection and version status
- [GitHub Releases](https://github.com/cooklang/cookcli/releases) - View all releases and changelogs