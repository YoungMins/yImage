#!/usr/bin/env bash
#
# Release helper for yImage.
#
# Usage:
#   scripts/release.sh 0.2.0
#
# What this does:
#   1. Validates the working tree is clean and the tag doesn't already exist.
#   2. Rewrites the version number in Cargo.toml and installer/yImage.iss.
#   3. Refreshes Cargo.lock so the pinned yimage entry matches.
#   4. Pauses so you can edit CHANGELOG.md for the new version.
#   5. Commits, tags, and prints the exact commands to publish.
#
# After you push the tag, .github/workflows/release.yml picks it up and
# builds the Windows installer + portable zip on GitHub Actions.

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "usage: $0 <version>"
    echo "example: $0 0.2.0"
    exit 1
fi

VERSION="$1"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "error: version must look like X.Y.Z (got: $VERSION)" >&2
    exit 1
fi

# Repo root — script may be invoked from anywhere.
cd "$(git rev-parse --show-toplevel)"

if [ -n "$(git status --porcelain)" ]; then
    echo "error: working tree has uncommitted changes. Commit or stash first." >&2
    git status --short >&2
    exit 1
fi

if git rev-parse "v$VERSION" >/dev/null 2>&1; then
    echo "error: tag v$VERSION already exists." >&2
    exit 1
fi

echo "==> Bumping yImage to v$VERSION"

# Rewrite Cargo.toml inside the first [package] block so a future workspace
# table or patched dependency with a "version = ..." line can't get caught
# by a naive global replace.
awk -v v="$VERSION" '
    /^\[package\]/ { in_pkg = 1 }
    /^\[/ && !/^\[package\]/ { in_pkg = 0 }
    in_pkg && /^version[[:space:]]*=[[:space:]]*"/ && !done {
        sub(/"[^"]+"/, "\"" v "\"")
        done = 1
    }
    { print }
' Cargo.toml > Cargo.toml.new
mv Cargo.toml.new Cargo.toml

# Inno Setup AppVersion default (overridable at compile time via /DAppVersion).
# Works on both GNU sed and BSD sed thanks to -i.bak + rm.
sed -i.bak -E "s|#define AppVersion \"[^\"]*\"|#define AppVersion \"$VERSION\"|" \
    installer/yImage.iss
rm -f installer/yImage.iss.bak

# Refresh Cargo.lock so the yimage entry matches the new version.
# --offline first; if that fails (no cached index) fall back to network.
cargo update -p yimage --offline 2>/dev/null || cargo update -p yimage

echo "==> Updated Cargo.toml, installer/yImage.iss, Cargo.lock"
echo ""
echo "==> Open CHANGELOG.md and fill in the [$VERSION] section, then come back."
read -rp "Press Enter once CHANGELOG.md is ready (or Ctrl+C to abort)... " _

git add Cargo.toml Cargo.lock installer/yImage.iss CHANGELOG.md
git commit -m "chore: release v$VERSION"
git tag -a "v$VERSION" -m "v$VERSION"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"

echo ""
echo "==> Tagged v$VERSION on branch $BRANCH"
echo ""
echo "To publish (runs the GitHub Actions release workflow):"
echo "    git push origin $BRANCH"
echo "    git push origin v$VERSION"
echo ""
echo "Or push both at once:"
echo "    git push --atomic origin $BRANCH v$VERSION"
