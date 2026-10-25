---
id: arch.sql-audit
title: SQL Access Audit
category: architecture
domain: persistence
status: implemented
source_paths: [crates/aevio-core/src, apps/api/src, apps/desktop/tauri/src, apps/cli/src]
related_docs: [docs/SEAORM_MIGRATION.md, docs/DATABASE_ARCHITECTURE.md, docs/DATABASE_COMPATIBILITY.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# SQL Access Audit

## Snapshot

The audit covers Rust and SQL sources in the repository, excluding generated
Graphify output and dependency lockfiles.

| Measure | `HEAD` baseline | Working tree | Interpretation |
|---|---:|---:|---|
| SQL/rusqlite candidate lines | 888 | 928 | Includes tests, migrations, adapters, and diagnostics |
| Files containing candidates | 44 | 59 | Includes the same non-production categories |
| Productive business-query implementations | present | 0 | Replaced by typed repositories |

The working-tree increase is migration coverage, regression fixtures, the
SeaORM query-plan diagnostic, and the PostgreSQL schema bootstrap—not restored
business SQL.

## Deliberate SQL exceptions

| Responsibility | Sources | Allowed content | Risk/control |
|---|---|---|---|
| Local historical schema | `crates/aevio-core/src/bootstrap/migrations.rs` | Versioned SQLite DDL and data-preserving migrations | `user_version`, migration tests, no reset/sync-on-open |
| Local bootstrap | `crates/aevio-core/src/bootstrap/state.rs` | WAL/synchronous/optimize and schema-version checks | Runs before repositories; compatibility tests |
| Local connection adapter | `crates/aevio-core/src/infrastructure/database/connection.rs` | Foreign keys, busy timeout, WAL | Single configured SeaORM-sync connection |
| SQLite sync-server schema | `apps/api/src/db.rs` | Additive initial schema and PRAGMAs | Request handlers use `Repository` |
| PostgreSQL sync-server schema | `apps/server-postgres/src/db.rs` | Additive initial schema | Request handlers use async SeaORM `Repository` |
| Performance diagnostics | `crates/aevio-core/src/infrastructure/database/query_plan_tests.rs` | `EXPLAIN QUERY PLAN` and isolated fixture SQL | Test-only, no production database mutation |
| Compatibility fixtures | `**/*tests*.rs`, `#[cfg(test)]` modules | Isolated setup, assertions, legacy behavior probes | Never compiled into production paths |

## Migrated responsibility inventory

All production reads and writes for these responsibilities use infrastructure
entities and repositories: activity segments and rollups, entries and history,
catalog/apps/projects/clients, tasks and labels, reports, timers, workspace
content, timeline sync, automation, breaks, energy, staffing, governance,
finance, invoices, AI queue/embeddings/kNN/calibration/artifacts, Coach, local
settings, and both SQLite and PostgreSQL sync-server request paths.

The audit was verified with the full workspace test suite, `bun run verify`,
`node scripts/aevio-knowledge.mjs verify`, and `graphify update .`.
