---
id: arch.rust-core
title: Rust Core (aevio-core)
category: architecture
domain: platform
status: implemented
source_paths: [crates/aevio-core/src/lib.rs, crates/aevio-core/src/]
related_docs: [docs/architecture/overview.md, docs/reference/data-model.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Rust Core (aevio-core)

Crate `aevio-core` (edition 2024). Owns domain logic and SQLite persistence. Shared by Tauri app and CLI.

`apps/api` is Aevio's library/domain API, not the network HTTP API. The
network entrypoints are `apps/server` and `apps/server-postgres`; their
FastAPI-inspired application composition lives under `src/app/`. Therefore
`aevio-core` intentionally has no `asgi`, `views`, or web-framework layer.

## Domain modules

`activity`, `ai`, `automation`, `bootstrap`, `breaks`, `capture`, `categories`, `clients`, `coach`, `decision`, `energy`, `entries`, `finance.rs`, `governance.rs`, `integrations`, `invoices`, `ipc`, `platform`, `projects`, `reports`, `settings`, `timers`, `workspace.rs`.

## Responsibilities

- Domain entities and invariants.
- Repository traits (ports) and SQLite adapters.
- Database migrations.
- Use-case orchestration (services).
- Shared IPC contracts (serde types).

## Mapping to the HTTP application layout

- `apps/api` provides the durable `core` capabilities and domain services.
- `apps/api/src/app/core` provides server-side technical adapters.
- `apps/api/src/app/services` composes HTTP use cases such as auth,
  workspace membership, and sync.
- `apps/api/src/app/shared` exposes stable network contracts.
- `apps/api/src/app/views` adapts Axum requests to those services.

## Boundaries

No Tauri, no OS APIs, no web framework. The target persistence boundary is
SeaORM 2.0.4 through `sea-orm-sync` for synchronous SQLite; historical schema
migrations and the low-level connection adapter remain the only rusqlite
exceptions. Platform-specific behavior lives in `platform/` and `capture/`
adapters. See `docs/SEAORM_MIGRATION.md` for the verified migration status.
