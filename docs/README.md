---
id: docs.readme
title: Aevio Documentation
category: architecture
domain: platform
status: implemented
source_paths:
  - AGENTS.md
  - README.md
  - apps/
  - packages/
related_docs:
  - docs/INDEX.md
  - docs/architecture/overview.md
platforms:
  - macos
  - windows
  - ios
  - watchos
verification_commit: working-tree
---

# Aevio Documentation

Aevio is a local-first desktop productivity and time-tracking application built as a Rust modular monolith wrapped in Tauri 2, with a React/Next.js view and optional on-device AI.

## Start here

- Humans: read `docs/README.md` (this file) then follow links.
- Agents: read `docs/INDEX.md` for the machine-friendly knowledge directory, then `AGENTS.md` for invariants and retrieval rules.

## Canonical documents (preserved)

- [`PROJECT_ANALYSIS.md`](PROJECT_ANALYSIS.md) — full repository analysis (German, v0.9.29).
- [`TAURI_COMMANDS.md`](TAURI_COMMANDS.md) — exhaustive Tauri command reference.
- [`ROUTE_MAP.md`](ROUTE_MAP.md) — web route map.
- [`DATA_MODEL.md`](DATA_MODEL.md) — database schema and migrations.

## Structure

| Directory | Purpose |
|---|---|
| `architecture/` | System boundaries, monorepo, Rust core, Tauri, React, packages, CLI, AI |
| `features/` | Domain feature documentation |
| `platforms/` | OS-specific capability matrices |
| `engineering/` | Rules, testing, CI/CD, releases, file-size limits |
| `reference/` | IPC contracts, events, data model, schemas, commands |
| `operations/` | Running, debugging, troubleshooting |
| `decisions/` | Architecture Decision Records |
| `agents/` | Retrieval system, context routing, evaluation, source priority |

## Source of truth

Current verified source code and tests outrank documentation. When a document conflicts with code, report the contradiction and correct stale docs. See `docs/agents/SOURCE_PRIORITY.md`.
