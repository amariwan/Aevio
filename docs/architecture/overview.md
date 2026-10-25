---
id: arch.overview
title: System Architecture Overview
category: architecture
domain: platform
status: implemented
source_paths:
  - crates/aevio-core/src/lib.rs
  - apps/desktop/tauri/src/lib.rs
  - apps/web/src/
  - packages/
related_docs:
  - docs/architecture/monorepo.md
  - docs/architecture/rust-core.md
  - docs/architecture/tauri.md
  - docs/architecture/react.md
platforms: [macos, windows, ios, watchos]
verification_commit: working-tree
---

# System Architecture Overview

## Style

Modular monolith. Rust owns all durable state (SQLite). React is a view that holds route, transient UI, and the last snapshot Rust pushed. Rust pushes events on change. No remote service; local-first.

## Layers

1. **Domain / Application** — `apps/api` (crate `aevio-core`): pure business logic, repositories, migrations. Shared by Tauri app and CLI.
2. **HTTP Services** — `apps/server` and `apps/server-postgres`: Axum applications composed under `src/app/{config,views,core,shared,services}`.
3. **Infrastructure / Adapters** — `apps/desktop/tauri`: Tauri commands, capture, notifications, tray, updater, RPC, native OS.
4. **View** — `apps/web` (Next.js 16 static export): React 19 components, hooks, TanStack Query.
5. **Native AI** — Swift sidecar (`apps/desktop/tauri/swift`): on-device ML, weak-linked, optional.
6. **Clients** — `apps/cli`, `apps/ios`, `apps/watch`.

## Dependency direction

`web` → Tauri IPC → `tauri` → `aevio-core` → SQLite. `cli` → `aevio-core` or RPC to running app. iOS/watch are separate Swift apps.

## Hexagonal boundaries

Ports (traits) in `aevio-core` define capabilities; adapters implement them per platform. See `docs/decisions/001-modular-monolith.md`.

## State flow

Rust emits events (`activity_snapshot`, `usage_snapshot`, etc.) on mutation; React subscribes via `useTauriEvent`. React never writes DB directly.

## Money

Integer USD cents in Rust (`invoices/money.rs`). Invoice PDFs composed in Rust (`invoices/pdf.rs`, printpdf), displayed via pdf.js. Never compute totals in TS.

## Concurrency

SQLite bundled, single writer; transactions at repository boundaries.
