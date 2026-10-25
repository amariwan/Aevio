---
id: arch.database-compatibility
title: Database Compatibility
category: reference
domain: persistence
status: implemented
source_paths: [crates/aevio-persistence/src/bootstrap/migrations.rs, crates/aevio-persistence/src/bootstrap/migrations.rs]
related_docs: [docs/DATABASE_ARCHITECTURE.md, docs/SEAORM_MIGRATION.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Database Compatibility

The migration must open the existing Aevio SQLite file in place. It must not
reset, recreate, or silently upgrade the schema. `user_version` remains the
compatibility gate used by the app and CLI.

The following are deliberate exceptions to the application-query ban:

- historical, versioned schema migrations in `crates/aevio-persistence/src/bootstrap/migrations.rs`;
- SQLite PRAGMAs required for foreign keys, WAL, synchronous mode, and schema
  version checks;
- test fixtures that create isolated in-memory schemas;
- EXPLAIN QUERY PLAN diagnostics and compatibility probes.
- Migration 28 ensures `timers` and `timer_segments` without changing existing
  rows, the retired `accumulated_ms` column, the `t<n>` ID format, or the
  `settings.timer_next_id` counter.
- Migration 29 adds the durable `music_tracks` catalog and its sort index. It
  seeds the built-in focus catalog idempotently through the SeaORM repository;
  existing rows and user-selected playback state are not rewritten.
- The former migration 25, which created `component_hub_favorites`, is retired
  with the component hub feature: fresh databases no longer create the table,
  and existing databases keep it untouched as inert data.

These exceptions are not business persistence implementations and must remain
isolated from domain/application modules.

Compatibility verification must cover fresh databases, every supported legacy
schema version, NULL values, approved-entry locks, interval overlaps, invoice
finalization, restart recovery, and simultaneous CLI/desktop access.

Current status: music catalog, timer, workspace, timeline, automation,
break-decision, staffing, energy, governance, finance, and the primary entry
list read path, project reporting/rules reads, and report rollup/export reads
are SeaORM-backed. The AI worker queue, embeddings, kNN pool,
rules, and calibration operations are also repository-backed. The standalone
sync server's auth, workspace-role, and sync push/pull request paths now use
typed SeaORM entities and its `Repository`; only its additive schema bootstrap
remains raw SQL. The PostgreSQL server follows the same repository boundary
with async SeaORM and applies its schema through the versioned
`schema_migrations` ledger in
`apps/server-postgres/src/app/core/database/migrations.rs`. Existing rusqlite
compatibility tests remain isolated regression fixtures; there are no remaining
production legacy boundaries.

## Direct SQLite touch-point inventory (2026-10-21)

A recursive scan for `rusqlite` and raw SQL call sites leaves exactly these
non-test locations. Each one is an approved infrastructure exception; none is a
business persistence implementation.

| Location | Why it is raw SQL | Guard |
| --- | --- | --- |
| `crates/aevio-persistence/src/bootstrap/migrations.rs` | historical versioned schema migrations and `user_version` steps, including additive plugin installation history at schema version 31, durable plugin jobs at schema version 32, workspace composition preferences at schema version 33, additive plugin job leases/retry state at schema version 34, the device namespace migration for plugin storage at schema version 35, durable plugin workflow runs at schema version 36, durable plugin event deliveries at schema version 37, plugin update policies at schema version 38, and the mediated local music file path at schema version 39 | compatibility tests in `bootstrap/migrations_tests.rs` |
| `crates/aevio-persistence/src/bootstrap/migrations.rs` | open flags, `user_version` check, required PRAGMAs | migration and CLI offline tests |
| `crates/aevio-persistence/src/database/connection.rs` | `foreign_keys`, `busy_timeout`, `journal_mode` PRAGMAs on connect | connection regression tests |
| `crates/aevio-core/src/infrastructure/database/query_plan_tests.rs` | EXPLAIN QUERY PLAN benchmark harness | `#[cfg(test)]` module |
| `apps/api/src/app/core/database/mod.rs` | single-instance SQLite server's additive bootstrap and PRAGMAs, including workspace plugin desired state | server contract tests |
| `apps/server-postgres/src/app/core/database/migrations.rs` | versioned PostgreSQL schema migrations | `tests/postgres_migration_contract.rs` |

Everything else that names `rusqlite` is compiled only for tests:

- API: `activity/{store.rs,history.rs,deletions.rs,deletions/tests.rs,store_tests.rs}`, `activity/store/{entries.rs,tasks.rs}`, `ai/store.rs` plus `ai/store_artifacts.rs` (the parent module is `#[cfg(test)]`), `coach/repository/*` and `coach/tests.rs`, `entries/query.rs`, `projects/service.rs`, `reports/query.rs`, and the `#[cfg(test)]` parts of `infrastructure/database/repositories/*`.
- Desktop: `breaks/store.rs`, `breaks/mod.rs`, `invoices/{mod.rs,profile.rs,tests.rs}`, and `ai/worker.rs` keep rusqlite fixtures only; production paths call the `*_with_database` / `BreakRepository` functions.
- `crates/aevio-core/src/lib.rs::err` is `#[cfg(test)]`.

Reproduce the inventory with:

```sh
grep -rn "rusqlite" --include="*.rs" apps/api/src apps/desktop/tauri/src apps/cli/src crates
```

A hit is a violation unless it is in the table above, is inside a
`#[cfg(test)]` module, or is a documentation comment. When adding persistence,
use the SeaORM repositories instead of reopening this list.


Catalog category/project/client reads and writes use `CatalogRepository`; the
client-delete transaction still refuses invoiced clients, detaches projects,
and soft-deletes the client atomically. Existing in-memory catalog fixtures
continue to exercise the old schema directly under `cfg(test)`.

Invoice list/detail/billable-entry/finalized-PDF reads preserve the existing
ordering, NULL handling, client/project ownership, and timestamp units through
`InvoiceRepository`. Draft resolution/save, finalization, payment, void, and
invoice deletion writes use typed SeaORM operations and preserve the existing
timestamp/NULL semantics. The legacy invoice transaction tests remain as
regression coverage for the same lifecycle.

Coach task CRUD preserves existing ordering, nullable timestamps, status
transitions, reminder reset behavior, and error messages through
`CoachTaskRepository`. Legacy task fixtures remain available under
`cfg(test)`.

Task, label, and task-label tables remain historical migration artifacts. Their
production CRUD runs through `TaskRepository`; the legacy rusqlite CRUD is
test-only. The repository preserves task ordering, keyword JSON normalization,
nullable fields, soft deletion, and atomic label replacement.

Single time-entry reads, detail projections, and event inserts use
`EntryQueryRepository` in the configured production path. Activity segment
lifecycle operations, snapshot/tick aggregation, startup recovery, retention
cleanup, rollups, and closed-entry description generation use typed
repositories. Entry mutations, approval/history operations, and deletion undo
now use typed SeaORM transaction paths; the old rusqlite ActivityStore and
history implementations are test-only fixtures. Integration metadata uses
`IntegrationRepository`.

The `apps` table keeps its existing integer flags and timestamp units. Listing,
override updates, and exclusion matching use `AppRepository`; capture-side
state transitions and historical test fixtures remain unchanged.

The migrated workspace and timer boundaries preserve their existing timestamp
units, ordering, nullable fields, initiative link table, timer ID format, and
timer counter. The compatibility boundary is implemented and remains protected
by the historical migration tests, CLI offline tests, and the full workspace
suite.

