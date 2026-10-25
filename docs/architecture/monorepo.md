---
id: arch.monorepo
title: Monorepo Organization
category: architecture
domain: platform
status: implemented
source_paths: [package.json, Cargo.toml, turbo.json, apps/, packages/]
related_docs: [docs/architecture/overview.md]
platforms: [macos, windows, ios, watchos]
verification_commit: working-tree
---

# Monorepo Organization

## Workspaces

Root `package.json` workspaces: `apps/*`, `packages/*`. Cargo workspace members include `apps/api`, `apps/desktop/tauri`, `apps/cli`, `apps/server`, and `apps/server-postgres`.

## Applications

- `apps/api` — `aevio-core` Rust library (shared domain + persistence).
- `apps/desktop/tauri` — Tauri 2 desktop app (Rust core + Swift sidecar build).
- `apps/web` — Next.js 16 static export (Turbopack) consumed by Tauri webview.
- `apps/cli` — `aevio` binary (clap; talks to running app via RPC or reads data directly).
- `apps/ios` — separate Swift app.
- `apps/watch` — separate Swift app.

## Shared packages

- `packages/api-client` — shared TS API client.
- `packages/config` — shared config.
- `packages/types` — shared TS types.
- `packages/ui` — shared UI.
- `packages/web` — shared web utilities.

## Tooling

Bun 1.4.3 package manager. Turbo for task orchestration. oxlint + oxfmt for frontend. cargo fmt + clippy for backend. Husky pre-commit (lint-staged).
