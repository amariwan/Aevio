---
id: operations.backup-recovery
title: Backup and Restore
category: operations
domain: operations
status: implemented
source_paths: [crates/aevio-persistence/src/bootstrap/migrations.rs, crates/aevio-persistence/src/database/connection.rs, crates/aevio-persistence/postgres-adapter/src/migrations.rs]
related_docs: [docs/operations/DEPLOYMENT.md, docs/DATABASE_COMPATIBILITY.md, docs/architecture/masterplan-progress.md]
platforms: [macos, linux]
verification_commit: working-tree
---

# Backup and Restore

Two stores need different procedures: the local SQLite database of the desktop
app and the PostgreSQL database of the self-hosted server. Both must be restored
into a database that the current binary accepts without a schema reset.

## Local SQLite (desktop and single-instance server)

The app uses WAL. Copying only `activity.db` while the app is running can miss
committed pages that still live in `activity.db-wal`, so take a consistent
snapshot either while the app is closed or through SQLite itself:

```sh
# Recommended: consistent copy while the app is running.
sqlite3 "$DATA_DIR/activity.db" "VACUUM INTO '/backups/activity-$(date +%Y%m%d).db'"

# Alternative: close the app, then copy the database and, if present, the WAL.
cp "$DATA_DIR/activity.db" /backups/
```

`$DATA_DIR` is the app data directory containing `activity.db`. A restored file
must keep `user_version`; the app refuses to guess and never resets a schema on
open (`docs/DATABASE_COMPATIBILITY.md`).

Verify a snapshot before trusting it:

```sh
sqlite3 /backups/activity-20261021.db "PRAGMA integrity_check; PRAGMA user_version;"
```

Expected: `ok` and the version the current app expects.

## PostgreSQL (self-hosted server)

Use the custom dump format, which restores selectively and compresses:

```sh
pg_dump --format=custom --no-owner --file=/backups/aevio-$(date +%Y%m%d).dump "$AEVIO_SERVER_DATABASE_URL"
```

Restore into a fresh database, never over the live one:

```sh
createdb aevio_restore
pg_restore --no-owner --dbname=aevio_restore /backups/aevio-20261021.dump
```

The dump includes `schema_migrations`. After restore, start the server once: it
applies only migrations that are missing, so a restored database is a no-op and
its rows are untouched.

## Restore rehearsal record

Executed on 2026-10-21 against a disposable `postgres:17-alpine` container
(`aevio-pg-it`, database `aevio_it`), with the versioned migration ledger in
place:

1. `pg_dump --format=custom --no-owner` produced a 25,417-byte dump.
2. `pg_restore` into a new `aevio_restore` database succeeded with exit code 0.
3. Restored rows matched the source: 95 workspaces, 72 accounts, 41 sync
   operations.
4. `schema_migrations` contained exactly `1:initial_sync_schema`.
5. Starting `aevio-server-postgres` against `aevio_restore` bound its listener
   and left `applied_at` at `1792605691913` — the migration was correctly
   treated as already applied.
6. The rehearsal database and dump were removed afterwards.

Repeat this rehearsal after any migration that changes storage layout, and
record the new row counts here.

## What is not covered

- Offsite retention, encryption at rest, and key management are deployment
  policy, not application behavior.
- A restore test does not prove application-level consistency of every domain;
  run the contract suites after a restore in a staging environment.
