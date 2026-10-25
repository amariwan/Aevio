---
id: arch.persistence-guidelines
title: Persistence Guidelines
category: engineering
domain: persistence
status: implemented
source_paths: [apps/api/src, apps/desktop/tauri/src, apps/cli/src]
related_docs: [docs/DATABASE_ARCHITECTURE.md, docs/SEAORM_MIGRATION.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Persistence Guidelines

- Domain and application code must not contain SQL strings or SeaORM query
  construction.
- Tauri commands and CLI handlers call application services, never repositories
  directly and never database connections.
- SeaORM entities are persistence models only; map them to domain and IPC types.
- Use typed columns and conditions for filters, joins, sorting, and pagination.
- Use explicit transactions for multi-write invariants. Never include network,
  sidecar, filesystem, or user-interaction work inside a database transaction.
- Preserve NULL semantics explicitly with `Option<T>` and SeaQuery expressions.
- Batch writes use SeaORM batch operations where they preserve existing ordering
  and error behavior.
- Historical migrations, required SQLite PRAGMAs, and query-plan diagnostics may
  use SQL. They must be isolated, reviewed, and listed in the compatibility doc.
- Never call schema synchronization against a production database as a side
  effect of opening the app.
- Add a regression test before changing approval locks, overlap checks,
  invoice finalization, history/undo, or activity folding.

