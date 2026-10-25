# Aevio development commands.
# Requires: bun, cargo, (optional) just.

default:
    @just --list

verify:
    bun run verify

fix:
    bun run fix

test:
    cargo test --workspace
    node --test scripts/*.test.mjs

test-postgres:
    cargo test -p aevio-server --tests

fmt:
    cargo fmt --all

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

docs-verify:
    node scripts/aevio-knowledge.mjs verify
