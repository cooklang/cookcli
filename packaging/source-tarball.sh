#!/usr/bin/env bash
# Build the release source drop, cook-<version>-source.tar.gz: a `git archive`
# of HEAD plus the two npm-generated web assets. GitHub's own tag archives
# leave those out — they are gitignored — and build.rs refuses to build
# without them, so packagers on platforms with no prebuilt lightningcss or
# esbuild could not produce them at all (issue #445).
#
# Usage:
#   packaging/source-tarball.sh <output-dir>
#
# Requires the compiled web assets (npm ci && npm run build-css && npm run
# build-js). Uncommitted changes are not included. Prints the tarball's path.
#
# Used by the release workflow for the attached asset and by
# packaging/fedora/build-rpm.sh, so both ship the same thing.
set -euo pipefail

out=${1:?usage: $0 <output-dir>}
out=$(realpath -m "$out")
repo=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo"

version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
prefix="cookcli-$version"
assets=(static/css/output.css static/js/editor.bundle.js)

for asset in "${assets[@]}"; do
    if [[ ! -f $asset ]]; then
        echo "missing $asset — run: npm ci && npm run build-css && npm run build-js" >&2
        exit 1
    fi
done

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# `git archive` rather than a plain `tar .`, so nothing untracked
# (node_modules/, target/) can leak in; the generated assets go on top.
git archive --format=tar --prefix="$prefix/" HEAD | tar -x -C "$work"
for asset in "${assets[@]}"; do
    install -D "$asset" "$work/$prefix/$asset"
done

mkdir -p "$out"
tarball="$out/cook-$version-source.tar.gz"
tar -czf "$tarball" -C "$work" "$prefix"

# The whole point of this tarball is those assets, and one missing them fails
# only much later, in someone else's build. The listing goes to a file rather
# than into `grep -q`, which would exit early, SIGPIPE the tar, and trip
# `pipefail` on a healthy tarball.
tar -tzf "$tarball" > "$work/contents.txt"
for asset in "${assets[@]}"; do
    grep -qx "$prefix/$asset" "$work/contents.txt" \
        || { echo "source tarball is missing $asset" >&2; exit 1; }
done

echo "$tarball"
