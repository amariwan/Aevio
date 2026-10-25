---
id: dec.modular-monolith
title: Modular Monolith Boundaries
category: decisions
domain: platform
status: accepted
source_paths: [crates/aevio-core/src/lib.rs]
related_docs: [docs/architecture/overview.md, docs/architecture/rust-core.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Modular Monolith Boundaries

Decision: single Rust crate `aevio-core` with domain modules; no microservices. Hexagonal ports/adapters. Dependency direction domain←app←infra←UI.
