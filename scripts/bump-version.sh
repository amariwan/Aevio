#!/usr/bin/env bash
# Bumps the version in package.json, apps/desktop/tauri/tauri.conf.json, and
# apps/desktop/tauri/Cargo.toml together, so the three can never drift. See
# RELEASING.md for the full release procedure.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

if [[ $# -ne 1 ]]; then
  echo "usage: $0 X.Y.Z" >&2
  exit 1
fi

VERSION="$1"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "error: version must be plain semver X.Y.Z (got: $VERSION)" >&2
  exit 1
fi

set_json_version() {
  local file="$1"
  # Only the top-level "version" field (the first match), so nested
  # dependency version strings elsewhere in the file are left alone.
  perl -0pi -e 's/("version":\s*")[^"]*(")/${1}'"$VERSION"'${2}/' "$file"
}

set_json_version package.json
set_json_version apps/desktop/tauri/tauri.conf.json

for manifest in apps/desktop/tauri/Cargo.toml apps/cli/Cargo.toml apps/api/Cargo.toml; do
  perl -0pi -e 's/(^version\s*=\s*")[^"]*(")/${1}'"$VERSION"'${2}/m' "$manifest"
done

echo "bumped package.json, Tauri config, and Rust package versions to $VERSION"
