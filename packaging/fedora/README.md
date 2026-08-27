# Fedora / RPM packaging

This directory contains everything needed to build RPMs of CookCLI for Fedora
(and other RPM-based distributions):

- `cookcli.spec` — the RPM spec. It builds from the release source tarball
  with vendored crates, fully offline, and installs the `cook` binary plus
  bash/zsh/fish completions.
- `build-rpm.sh` — one-shot helper that produces the source tarball, the
  vendored-crate tarball, and the RPMs from a checkout.

## How the package is built

Two sources go into the build:

| Source | Content |
| --- | --- |
| `cook-<version>-source.tar.gz` | The release source tarball (`git archive` + the two npm-generated web assets). This is the same artifact attached to GitHub releases, so no Node.js is needed at build time. |
| `cookcli-<version>-vendor.tar.xz` | `cargo vendor` output plus a `.cargo/config.toml` that redirects crates.io to it, so `%build` runs with `--offline` as a build root requires. |

The spec builds with `--no-default-features --features server,import,lsp,sync`:

- **`self-update` is disabled on purpose.** A packaged binary must not replace
  itself from GitHub releases — the package manager owns the file. `cook
  update` is therefore absent; users update with `dnf upgrade`.
- The **`sync`** feature (CookCloud login/logout) bundles SQLite via
  libsqlite3-sys' `bundled` feature, which `cooklang-sync-client` hardcodes.
  If your target distribution forbids bundled libraries, rebuild with
  `rpmbuild --without sync`.

## Building locally

```bash
npm install && npm run build-css && npm run build-js   # once, for the web assets
./packaging/fedora/build-rpm.sh
```

The script uses `git archive HEAD`, so commit your changes first (or pass a
downloaded `cook-<version>-source.tar.gz` as an argument to package that
release). RPMs and the SRPM land in `dist/rpm/`.

Inside a Fedora container:

```bash
docker run --rm -it -v "$PWD:/src:Z" -w /src fedora:latest bash -c '
  dnf install -y rpm-build gcc git tar xz curl &&
  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal &&
  source "$HOME/.cargo/env" &&
  ./packaging/fedora/build-rpm.sh'
```

## CI

- `.github/workflows/rpm.yaml` builds RPMs on demand (manual dispatch, or on
  changes to this directory) so packaging changes can be tested without
  cutting a release.
- The release workflow builds RPMs in Fedora containers for every GitHub
  release and attaches them to the release, next to the tarballs. One build
  uses the oldest maintained Fedora release so the resulting RPM installs on
  older systems too (glibc is forward compatible).

## Fedora submission notes

Getting CookCLI into the official Fedora repositories requires more than this
spec: every Rust crate dependency must become its own `rust-<crate>` package
(the [Fedora Rust packaging guidelines][fedora-rust] do not allow vendored
sources), and the `License` tag must be expanded to cover all of them. The
COPR route is far lighter:

1. Create a COPR project, e.g. `copr create <user>/cookcli --chroot fedora-rawhide-x86_64 ...`.
2. Upload `cookcli.spec` plus both source tarballs (or point COPR at a
   monitoring script that regenerates them per release).
3. `copr build <user>/cookcli cookcli.spec`.

This spec keeps the same feature set and file layout a Fedora package would
have, so it is a good starting point for a proper
[package review][fedora-review] if someone wants to carry it into the distro.

[fedora-rust]: https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/
[fedora-review]: https://docs.fedoraproject.org/en-US/package-maintainers/policy/optional_reviews/
