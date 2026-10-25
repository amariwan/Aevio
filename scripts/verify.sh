#!/usr/bin/env bash
# Single entry point for lint/format/typecheck/clippy. Agents and humans
# should run this (not ad hoc oxlint/cargo invocations) before calling a task
# done. Pass --fix to apply safe auto-fixes instead of just checking.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

FIX=false
if [[ "${1:-}" == "--fix" ]]; then
  FIX=true
fi

ROOT="$(pwd)"
cd "$ROOT/apps/web"

echo "==> oxlint + oxfmt (frontend lint + format)"
if $FIX; then
  bunx oxfmt --write .
  bunx oxlint --fix .
else
  bunx oxfmt --check .
  bunx oxlint .
fi

echo "==> tsc (typecheck)"
bunx tsc --noEmit

echo "==> docs metadata (aevio-knowledge verify)"
cd "$ROOT"
node scripts/aevio-knowledge.mjs verify
cd "$ROOT/apps/web"

echo "==> cargo fmt (backend)"
if $FIX; then
  cargo fmt --all
else
  cargo fmt --all --check
fi

echo "==> cargo clippy (backend)"
if $FIX; then
  cargo clippy --manifest-path "$ROOT/apps/desktop/tauri/Cargo.toml" --workspace --all-targets --fix --allow-dirty --allow-staged -- -D warnings
else
  cargo clippy --manifest-path "$ROOT/apps/desktop/tauri/Cargo.toml" --workspace --all-targets -- -D warnings
fi

echo "verify: all checks passed"
