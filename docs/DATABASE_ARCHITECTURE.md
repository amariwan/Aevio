---
id: arch.database
title: Database Architecture
category: architecture
domain: persistence
status: implemented
source_paths: [crates/aevio-persistence/src/bootstrap, crates/aevio-persistence/src/activity, crates/aevio-core/src/]
related_docs: [docs/SEAORM_MIGRATION.md, docs/PERSISTENCE_GUIDELINES.md, docs/DATABASE_COMPATIBILITY.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Database Architecture

## Current verified state

`aevio-core` owns the local SQLite database. The migrated boundaries use
SeaORM-sync 2.0.4 repositories, including work intelligence, agent context,
project-rule attachment, project statistics/rules/preview, and the primary
entry-list read path. The AI worker's
queue/embedding/calibration path also uses SeaORM repositories. Activity
entry mutations, structural history, deletion undo/redo, and closed-entry
description updates now use typed SeaORM repositories; the old AI SQL store and
legacy activity history are test-only compatibility fixtures. Activity rebuilds
and Coach operations are repository-backed. Invoice reads and the complete invoice lifecycle are repository-backed;
the legacy invoice SQL is test-fixture-only. Report rollups and exports are now
repository-backed as well. Category/project/client catalog CRUD and its
invoice-protected client-delete transaction are repository-backed; the
legacy catalog code is test-fixture-only.
Task and label CRUD is repository-backed, including the task-label transaction
and soft-delete semantics. Personal Coach task CRUD, daily plans, focus,
reminders, adaptive commands, and brain writes are repository-backed; the old
Coach repository tree remains test-only for compatibility fixtures.
The writer is protected by the existing process lock and store mutexes; the
activity reader is intentionally separate.
The schema is created and upgraded by the historical migrations in
`crates/aevio-persistence/src/bootstrap/migrations.rs`.

All production public stores now use the repository boundary. `rusqlite`
remains only in test fixtures and the historical SQLite bootstrap/connection
adapters.

## Target boundary

```text
Tauri commands / CLI RPC
        -> application services
        -> domain-owned persistence ports
        -> infrastructure/database repositories
        -> sea-orm-sync 2.0.x (rusqlite backend)
        -> existing SQLite file
```

Entities are infrastructure representations. IPC models and domain models remain
separate. Repositories own SeaORM conditions, relations, pagination, and row
conversion. Historical schema migrations remain versioned SQL because they are
data-preserving migration artifacts, not business queries.

## Concurrency invariant

The first implementation uses `sea-orm-sync`, not async SeaORM. Each connection
must preserve the existing WAL, foreign-key, busy-timeout, and process-lock
policy. No repository may hold a database transaction across an external sidecar
or network call.

## Ownership map

| Area | Target repository boundary | Risk |
|---|---|---|
| Activity, entries, segments, history | activity/entries repositories | critical |
| Tasks, labels, task-label links | `TaskRepository` | migrated CRUD; activity capture remains open |
| Foreground apps and exclusions | `AppRepository` | migrated catalog/exclusion reads; capture writes remain open |
| Projects, clients, timers | work repositories | high |
| Invoices | finance repositories | critical |
| Reports | `ReportRepository` | migrated read/aggregation path |
| AI, coach, automation | AI/coach repositories | high |
| Settings and schema bootstrap | settings/database infrastructure | high |
| Staffing, workspace, energy, governance, finance | supporting repositories | migrated |
| Server sync | `apps/api/src/db.rs` and `Repository` | migrated request persistence; schema bootstrap remains SQL |

The standalone SQLite sync server keeps only its database path in Axum state
because the synchronous SeaORM rusqlite connection is intentionally not
`Send + Sync`. Each request opens a SeaORM-sync connection; request-local push
transactions preserve the previous atomicity while WAL, foreign keys, busy
timeout, and synchronous-mode PRAGMAs remain enabled. The PostgreSQL sync
server uses the asynchronous SeaORM 2.0.4 PostgreSQL driver and a shared
connection pool; its request handlers call the typed repository in
`apps/server-postgres/src/db.rs`.

