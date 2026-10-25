---
id: eng.testing
title: Testing
category: engineering
domain: platform
status: implemented
source_paths: [scripts/verify.sh, AGENTS.md]
related_docs: [docs/engineering/ci-cd.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Testing

- Frontend unit tests (Node built-in runner): `node --test scripts/*.test.mjs`.
- Rust tests: `cargo test --manifest-path apps/desktop/tauri/Cargo.toml`.
- Specimen invoice PDFs: `AEVIO_INVOICE_SAMPLES=<dir> cargo test ... invoices::tests::renders`.
- `bun run verify` does NOT run tests; run separately.
- No test frameworks added on agent initiative.
