---
id: arch.seaorm-migration
title: SeaORM Migration
category: architecture
domain: persistence
status: implemented
source_paths: [apps/api/src, apps/desktop/tauri/src, apps/cli/src]
related_docs: [docs/DATABASE_ARCHITECTURE.md, docs/DATABASE_COMPATIBILITY.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# SeaORM Migration

## Decision

The compatible stable release available for this workspace is SeaORM `2.0.4`.
The selected execution model is `sea-orm-sync 2.0.4` with its rusqlite driver.
This preserves synchronous store APIs, the existing Tauri threads, the CLI
offline path, and the single-writer process lock. An async rewrite is not
justified for this desktop-local database.

Official references:

- [SeaORM database and runtime configuration](https://www.sea-ql.org/SeaORM/docs/install-and-config/database-and-async-runtime/)
- [SeaORM 2.0 synchronous backend](https://www.sea-ql.org/blog/2025-12-12-sea-orm-2.0/)
- [sea-orm-sync 2.0.4](https://docs.rs/crate/sea-orm-sync/2.0.4)

## Audit snapshot: 2026-10-21 (after the current migrated boundaries)

The current recursive text scan finds 44 Rust files naming `rusqlite` and 59
files containing SQLite call-site patterns. These counts intentionally include
test fixtures and historical migration/compatibility adapters; they are not a
count of production queries. The migrated boundaries (`timers`, workspace content, timeline,
automation, break decisions, staffing, energy telemetry, governance, finance,
work intelligence, and the primary entry-list read path) no longer contain
productive raw SQL. The production query audit leaves only the documented
bootstrap/connection adapters and test-only fixtures on the direct SQLite path.
The PostgreSQL sync server is also repository-backed with SeaORM 2.0.4; its
only SQL is the versioned schema ledger in
`apps/server-postgres/src/app/core/database/migrations.rs`. Migration 1 creates
the original tables, an existing unversioned deployment is adopted without
touching rows, and every later schema change is an append-only version recorded
in `schema_migrations`. `apps/server-postgres/tests/postgres_migration_contract.rs`
runs the fresh, adoption, and concurrent-start cases against real PostgreSQL in
an isolated schema.

The repeatable query-plan benchmark is
`infrastructure::database::query_plan_tests::typed_entry_listing_keeps_the_indexed_plan_and_benchmark_baseline`.
On the 2026-10-21 run it seeded 50,000 entries, executed 30 iterations, and
reported raw SQLite 1.55 ms versus SeaORM 4.30 ms for the same 17-row range.
Both plans use `idx_time_entries_started_at`; the absolute typed query time is
below 5 ms in this fixture, while the driver-overhead difference remains an
explicit performance risk for future profiling.

The first complete slices are `timers` and workspace content
(`documents`, `notes`, `comments`, `attachments`, `mentions`, `cycles`, and
`initiatives`). Timer listing uses
two typed queries rather than one query per timer; pause, reset, and delete
preserve their multi-write atomicity through SeaORM transactions. The Tauri
commands open the configured database through the shared infrastructure
adapter; no SeaORM connection is stored in `AppState`, preserving Tauri's
`Send + Sync` state contract.

The complete timeline boundary now uses SeaORM for segment projection,
publication/unpublication, devices, sync-state, outbox, cursors, and remote
events. Push/pull application is one SeaORM transaction; the Tauri commands no
longer pass a rusqlite connection into timeline code.

Migration version 28 additively ensures the historical `timers` and
`timer_segments` tables with `CREATE TABLE IF NOT EXISTS`. It does not reset,
recreate, or synchronize existing schema/data.

The automation boundary uses typed repositories for routines, reports,
dashboards, run history, daily workflow metrics, and the unclassified-entry AI
queue operation. Daily metrics load overlapping entries once and preserve the
legacy clipping, NULL, status, and invoice semantics in typed Rust code.

Break decisions use the shared SeaORM database adapter in production. The
existing rusqlite break tests remain isolated fixtures under `cfg(test)` and
continue to exercise the historical schema and ordering contract.

Energy telemetry uses a typed `energy_samples` entity and repository for the
background sampler, history, retention, and reset operations. Sampling still
runs on its own SeaORM-sync connection, while Tauri commands serialize user
initiated operations through the existing activity writer lock.

Work Intelligence uses typed repositories for relationship listing and
creation, entity validation, graph neighbors, project children, parent links,
labels, and pending-entry context. RPC entry suffix resolution and the agent
context preview use the same infrastructure boundary. Project discovery now
loads project names, active hint patterns, dismissals, and capture segments
through `ProjectDiscoveryRepository`. Project statistics, project rules, hint
preview, and dismissal writes now use SeaORM repositories as well. Existing graph limits, deleted-row
filtering, duplicate relationship behavior, and short transaction boundaries
are preserved.

The AI worker's queue scheduling, retry/backoff state, fallback requeue,
embedding upsert/read, approved-entry kNN pool, rule loading, calibration
samples, model artifacts, and learning reset now use SeaORM repositories.
Worker tests use a temporary SQLite file so the SeaORM and legacy read
connections observe the same data. `crates/aevio-core/src/ai/store.rs` remains compiled
only for compatibility tests and is not part of the production module graph.

The standalone sync server now uses `apps/api/src/db.rs` for accounts,
access tokens, workspaces, memberships, and sync operations. Its request
handlers no longer contain SQL or rusqlite row mappings. Because the sync
driver is not `Send + Sync`, Axum state stores only the database path and each
request opens a SeaORM-sync connection; push remains one repository
transaction. The server's additive schema bootstrap and required PRAGMAs are
documented historical infrastructure exceptions.

The Tauri and offline RPC entry-list read path now uses an
`EntryQueryRepository`. It preserves `none`/NULL filters, pending versus
approved semantics, app/domain fallback, title/description search, descending
start ordering, and the hard list limit. Production report rollups and exports
now use `ReportRepository`, including app-segment clipping, distinct entry
counts, status/billable totals, dynamic filters, and export name mapping. The
legacy report SQL remains only in test fixtures.

Work-management tasks and labels now use `TaskRepository` for CRUD, project
existence validation, relational label replacement, and soft deletion. Task
keyword normalization, NULL fields, status ordering, and the task-label
transaction remain compatible with the legacy store. The old task SQL is
retained only in `cfg(test)` fixtures.

The EntryQuery repository owns single-entry reads, detail segments, entry-event
inserts, and the configured production detail projection. Activity rebuilds now
use `ActivityRebuildRepository` and a SeaORM transaction for typed segment
projection, entry upserts, stale-entry cleanup, and classification queueing.
Entry and catalog deletions now use `TypedDeletion` in the production build,
including SeaORM transaction boundaries and typed undo/redo commands. Manual
entry mutations, approval/rejection/unapproval, and Split/Merge history now use
typed repositories and SeaORM transactions with typed undo/redo snapshots;
Split also preserves inherited suggestions and Merge preserves dependent AI/event
metadata. The remaining activity boundary is limited to historical migration
and compatibility infrastructure; the old history and AI SQL implementations
are test-only and closed-entry description generation is repository-backed.

Activity segment creation/closing, review flags, snapshot/tick aggregation,
startup recovery, retention cleanup, and daily rollups now use the typed
`ActivityRepository` in the production path. Integration connection listing
and upsert use `IntegrationRepository`. Existing capture behavior and the
single-writer lock are unchanged; the in-memory rusqlite implementation is
retained only by test fixtures while the remaining Activity mutations are
converted as a transactionally complete slice.

Foreground-app listing, user overrides, and exclusion checks now use
`AppRepository`. The capture state machine remains in `ActivityStore`, but its
production exclusion lookup no longer prepares a rusqlite statement.

Catalog reads and CRUD for categories, projects, and clients now use
`CatalogRepository` in production. Its client-delete transaction preserves the
invoice guard, project detachment, soft-delete timestamp, and atomicity. The
rusqlite catalog implementations remain only for in-memory regression fixtures.

Invoice list/detail/billable-entry reads and finalized-PDF reads now use the
typed invoice entities and `InvoiceRepository`. Draft resolution/save,
finalization (including the number sequence and PDF snapshot), payment
transitions, voiding, and draft/void deletion use typed SeaORM operations and
 transactions. The old invoice SQL remains only in `cfg(test)` fixtures.

The legacy personal Coach task IPC now uses `CoachTaskRepository` for list,
create, update, completion/reminder changes, and deletion. Its old rusqlite
adapter is compiled only for compatibility tests. Daily plans, brain writes,
focus, reminders, adaptive commands, and weekly projections now use typed
repositories and SeaORM transactions; the old coach repository tree remains
test-only for compatibility fixtures. Coach memory/audit metadata has a
repository-backed path where already wired.

The largest production groups are:

| Group | Representative paths | Migration risk |
|---|---|---|
| Activity and entries | `crates/aevio-core/src/activity/**`, `entries/**` | critical |
| Reports | `crates/aevio-core/src/reports/query.rs`, `infrastructure/database/repositories/reports.rs` | migrated read/aggregation path; legacy SQL is test-only |
| AI and coach | `crates/aevio-core/src/ai/**`, `coach/repository/**` | high; only the daily queue operation is migrated |
| Invoices | `apps/desktop/tauri/src/invoices/**` | critical |
| Projects and timers | `crates/aevio-core/src/projects/**`, `timers/**` | high |
| Energy and work intelligence | `apps/desktop/tauri/src/work_intelligence.rs` | migrated |
| Staffing and governance | `crates/aevio-core/src/staffing.rs`, `crates/aevio-core/src/governance.rs` | migrated; schema artifacts remain historical |
| Bootstrap | `crates/aevio-core/src/bootstrap/migrations.rs` | historical SQL exception |
| Server crates | `apps/server/**`, `apps/server-postgres/**` | sync server request persistence migrated; schema bootstrap remains SQL |

## Completion boundary

The migration is implemented. Every production store is off
`&rusqlite::Connection`; direct SQLite access is limited to the documented
adapter/bootstrap exceptions, the query-plan comparison is recorded and
reviewed, and the regression suite passes against existing database files.

No dependency-only or unused entity layer is acceptable as completion evidence.

## Required sequence

1. Introduce the database connection adapter and preserve PRAGMAs/locking.
2. Generate and review entities from the existing schema; do not synchronize
   or mutate the production schema automatically.
3. Migrate one responsibility boundary at a time, starting with projects and
   timers, then entries/activity, reports, finance/invoices, and AI/coach.
4. Replace public connection parameters with repository/service boundaries.
5. Remove each old rusqlite implementation immediately after its replacement.
6. Run compatibility, concurrency, query-plan, unit, CLI, and canonical verify
   gates after every boundary.

