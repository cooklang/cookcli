#!/usr/bin/env bash
# Build cookcli RPMs from this checkout (or from a release source tarball).
#
# Usage:
#   packaging/fedora/build-rpm.sh [source-tarball]
#
# Without arguments the source tarball is created from HEAD with `git archive`
# (uncommitted changes are not included) plus the two npm-generated web assets,
# mirroring the `cook-<version>-source.tar.gz` asset attached to GitHub
# releases. Pass a downloaded release tarball to package that version instead.
#
# Requires: git, cargo, rpmbuild, tar, xz, and the compiled web assets
# (npm install && npm run build-css && npm run build-js).
#
# Output: RPMs and a SRPM under <repo>/dist/rpm/.
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
provided=${1:-}
[[ -z $provided ]] || provided=$(realpath "$provided")
cd "$repo"

version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
source_name="cook-${version}-source.tar.gz"
vendor_name="cookcli-${version}-vendor.tar.xz"
work=$(mktemp -d "$repo/rpmbuild.XXXXXX")
work=$(realpath "$work")
topdir="$work/topdir"
out=$(realpath -m dist/rpm)
trap 'rm -rf "$work"' EXIT

for tool in git cargo rpmbuild tar xz; do
    command -v "$tool" >/dev/null || { echo "missing tool: $tool" >&2; exit 1; }
done

mkdir -p "$topdir"/{BUILD,RPMS,SOURCES,SPECS,SRPMS} "$work/src"

# --- Source0: the release-style source tarball -------------------------------
if [[ -n $provided ]]; then
    [[ -f $provided ]] || { echo "no such file: $provided" >&2; exit 1; }
    cp "$provided" "$topdir/SOURCES/$source_name"
else
    # git archive, so nothing untracked (node_modules/, target/) leaks in; the
    # two generated assets are then copied on top, like the release workflow.
    git archive --format=tar --prefix="cookcli-$version/" HEAD | tar -x -C "$work/src"
    for asset in static/css/output.css static/js/editor.bundle.js; do
        if [[ ! -f $asset ]]; then
            echo "missing $asset — run: npm install && npm run build-css && npm run build-js" >&2
            exit 1
        fi
        install -D "$asset" "$work/src/cookcli-$version/$asset"
    done
    tar -czf "$topdir/SOURCES/$source_name" -C "$work/src" "cookcli-$version"
fi
srcdir="$work/src/cookcli-$version"
if [[ ! -d $srcdir ]]; then
    tar -xzf "$topdir/SOURCES/$source_name" -C "$work/src"
fi
for asset in static/css/output.css static/js/editor.bundle.js; do
    [[ -f "$srcdir/$asset" ]] || { echo "source tarball is missing $asset" >&2; exit 1; }
done

# --- Source1: vendored crates -------------------------------------------------
# Vendor into the extracted tree so the lockfile and the vendored sources
# always agree, then bundle vendor/ + .cargo/ for `%setup -a 1`.
echo ">> vendoring crates (network required)..."
(cd "$srcdir" && cargo vendor --locked vendor >/dev/null)
mkdir -p "$srcdir/.cargo"
cat > "$srcdir/.cargo/config.toml" <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"

[net]
offline = true
EOF
tar -cJf "$topdir/SOURCES/$vendor_name" -C "$srcdir" vendor .cargo

# --- rpmbuild ------------------------------------------------------------------
sed "s/^Version:.*/Version: $version/" \
    packaging/fedora/cookcli.spec > "$topdir/SPECS/cookcli.spec"
rpmbuild -ba --define "_topdir $topdir" "$topdir/SPECS/cookcli.spec"

mkdir -p "$out"
cp -v "$topdir"/RPMS/*/*.rpm "$topdir"/SRPMS/*.src.rpm "$out"
echo
echo "Packages and sources in $out:"
ls -l "$out"
