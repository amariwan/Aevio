---
id: arch.masterplan-progress
title: Architecture migration evidence and remaining work
category: architecture
domain: platform
status: experimental
source_paths:
  - Cargo.toml
  - crates/aevio-contracts
  - crates/aevio-services
  - crates/aevio-http
  - crates/aevio-services/src/timeline.rs
  - apps/api/src/app
related_docs:
  - docs/architecture/server.md
  - docs/architecture/monorepo.md
platforms: [macos, windows]
verification_commit: working-tree
---

# Masterplan migration evidence — 2026-10-21

The requested target is in `docs/plan/AEVIO_MASTERPLAN_2026.md`. This is an
incremental evidence ledger, not a declaration that the target is complete.
Existing uncommitted changes predate this migration pass and must be preserved.

## Verified initial structure

| Boundary | Existing implementation | Target gap |
| --- | --- | --- |
| Local core | `apps/api`, crate `aevio-core`, synchronous SQLite repositories | Domain and persistence still share a crate |
| Desktop | `apps/desktop/tauri` consumes local core; Next static frontend in `apps/web` | Full offline acceptance still requires explicit verification |
| CLI | `apps/cli` consumes core and local RPC | Remote-mode parity not yet audited |
| SQLite server | `apps/server`, Axum, SeaORM-sync | Auth and sync orchestration still in views/repository |
| PostgreSQL server | `apps/server-postgres`, Axum, async SeaORM | Separate orchestration; real PostgreSQL tests not evidenced |
| Web | Current `apps/web` is also the desktop view | Independent server-client feature parity not established |

Do not rename `apps/api` to the HTTP server or move the desktop to Vite merely
to match the illustrative directory tree. Build boundaries and compatibility
must be demonstrated first.

## Slice 1 — shared wire contracts and role policy

- `aevio-contracts::sync` owns the five v1 sync envelopes. The local timeline and
  the PostgreSQL adapter re-export these types, preserving import paths,
  camelCase field names, optional base revisions and timestamp units.
- `aevio-contracts::sync` additionally owns the registered-operation limits, the
  canonical payload form and the content fingerprint used for replay detection.
- `aevio-services::identity` is the single source for the role matrix, service
  token scope, session lifetime, registration rules and membership-change
  invariants. The earlier `workspaces::roles_for` compatibility shim duplicated
  the same matrix and was removed once both adapters used `identity`.
- `aevio-services::replay` classifies a push operation as new, idempotent or a
  payload conflict. The crates have no HTTP, Tauri or ORM dependencies.

## Slice 2 — one adapter implementation per protocol

- `crates/aevio-http` owns the error envelope and the status mapping. Both
  servers return `{"error":{"code":"...","message":"..."}}` with stable
  uppercase codes; unexpected failures are logged and reported as
  `internal server error`.
- Both adapters were split into `config`, `state`, `authz`, `views/{health,auth,
  workspaces,sync}` and `core/database`, with `shared` reduced to contract
  re-exports. Handlers validate and dispatch; they no longer carry policy.
- Service tokens are scoped: `AEVIO_SERVER_TOKEN_WORKSPACES` (fallback
  `AEVIO_SERVER_WORKSPACE`) plus `AEVIO_SERVER_TOKEN_PERMISSIONS` (default
  `read,write`). An empty scope refuses to start, a token can never exceed the
  deployment workspace, and a service actor is never an owner.
- Sessions expire 30 days after `created_at`; `revoked_at` always wins. Email
  identity is normalized and matched with SQL `lower()`, not an in-memory scan.
- Push replay recomputes the stored fingerprint from persisted columns. The same
  `operationId` with different content is reported per operation in
  `push.conflicts`, not as a request-level failure. An unparseable stored payload
  fails closed.
- The last workspace owner cannot be demoted (`409`), and an `admin` cannot
  grant `owner` (`403`). Both rules live in `aevio-services::identity`.
- Historical SQL, CLI SQL and fixtures remain the documented exceptions in
  `docs/DATABASE_COMPATIBILITY.md`; handlers and repositories contain no raw SQL.

## Slice 3 — persistence adapters and real PostgreSQL contracts

- Workspace creation, membership grants and push idempotency no longer use
  check-then-insert. `ensure_workspace` and the operation insert are
  `ON CONFLICT DO NOTHING`; `upsert_membership` is a single upsert that keeps the
  original `created_at`. Two concurrent grants can no longer fail on the
  membership primary key.
- Both adapters store a push operation through one `store_operation` helper. The
  primary key decides a concurrent duplicate, the winning row is read back, and
  the loser reports an idempotent replay or a payload conflict per operation
  instead of turning the whole push into a `500`. SeaORM 2.0.4 maps a skipped
  `INSERT ... RETURNING` to `RecordNotFound`, so the helper uses
  `exec_without_returning` plus a read-back; that read-back also supplies the
  database-assigned revision on PostgreSQL.
- The SQLite response cursor is now the revision that was actually stored, not
  the revision the attempt proposed, so a duplicate inside one push cannot
  advance the cursor past the stored rows.
- `apps/server-postgres` is a library plus a thin binary, so the router can be
  built in tests. `tests/` covers auth, memberships and sync against a real
  `postgres:17-alpine` instance; without `AEVIO_TEST_POSTGRES_URL` each test logs
  an explicit skip instead of passing silently.
- Schema bootstrap takes `pg_advisory_xact_lock` inside one transaction, because
  `CREATE TABLE IF NOT EXISTS` is not race safe in PostgreSQL.
- The PostgreSQL schema is now applied through an append-only versioned ledger
  (`schema_migrations`) in
  `apps/server-postgres/src/app/core/database/migrations.rs`. Migration 1 is the
  original schema, so an existing deployment is adopted without touching rows;
  later versions are added, never edited. The SQLite server keeps its documented
  additive bootstrap because it is a single-instance store.
- The direct SQLite touch-point inventory is written down in
  `docs/DATABASE_COMPATIBILITY.md`: six approved infrastructure locations and a
  file-by-file list of the remaining `#[cfg(test)]` fixtures. The recursive scan
  leaves no production raw-SQL boundary outside infrastructure.
- Backup/restore is documented in `docs/operations/BACKUP_RECOVERY.md`, together
  with a restore rehearsal that was actually executed. Deployment, environment
  variables and the compose sketch are in `docs/operations/DEPLOYMENT.md`.
- Both server adapters expose `/readyz`. It pings the configured storage and
  returns `503` without leaking the database error when SQLite or PostgreSQL is
  unavailable; `/healthz` remains a process-liveness endpoint.

## Final verification pass — 2026-10-21

- `bun run verify`: oxlint + oxfmt, tsc, docs metadata, cargo fmt, cargo clippy
  all green (0 errors).
- `cargo test --workspace`: all suites green (desktop 81 + api/server crates +
  service tests).
- Frontend unit tests: `node --test scripts/*.test.mjs` 31/31 pass.
- Uncommitted working-tree changes preserved (852 changed paths); no files
  restored or reverted.
- No commits, branches, PRs, pushes or deployments created.

### Residual risks (honest)

- `aevio-core` still carries SQLite/SeaORM/HTTP/runtime dependencies and the
  bulk of domain modules; full extraction into `aevio-services` /
  `aevio-persistence` / `aevio-infrastructure` remains a multi-slice
  refactor. The current crate boundaries compile and pass tests, but the
  ideal ownership split is not yet complete.
- `packages/features` is not yet present; shared UI feature composition lives
  in `apps/web` and `apps/desktop` until a real shared consumer is needed.
- Real PostgreSQL integration tests require `AEVIO_TEST_POSTGRES_URL`; they log
  an explicit skip otherwise.
- Native WebView / target-hardware checks were not executed in this pass.

### Executed evidence (2026-10-21 working tree)

- `AEVIO_TEST_POSTGRES_URL=postgres://postgres:it-secret@127.0.0.1:55432/aevio_it
  cargo test -p aevio-server-postgres --tests` → 5 unit + 7 auth + 3 membership +
  3 migration + 8 sync tests pass, run twice against the same database.
- `cargo test -p aevio-server` → 14 tests pass, including the new
  same-operation-id-twice push case.
- Restore rehearsal: a custom-format dump (25,417 bytes) restored into a new
  `aevio_restore` database with exit code 0; restored rows were 95 workspaces, 72
  accounts and 41 sync operations; `schema_migrations` held exactly
  `1:initial_sync_schema`. Starting the server against the restored database
  bound its listener and left `applied_at` unchanged, proving the migration was a
  no-op. The rehearsal database and dump were then removed.
- The two integration defects this slice fixes were observed first, not assumed:
  a duplicate `operationId` racing on the primary key returned `500`, and a
  membership grant for a workspace row that did not exist violated
  `workspace_memberships_workspace_id_fkey`.

## Evidence

After both slices: `bun run fix` and `bun run verify` pass (oxlint, oxfmt,
`tsc --noEmit`, rustfmt check, clippy with `-D warnings`, docs metadata).
`cargo test --workspace` passes 415 tests, and `node --test scripts/*.test.mjs`
passes 31 tests. New coverage includes the role matrix, service scopes, token
expiry/revocation, membership invariants, push validation limits, canonical
payload stability, fingerprint equality and inequality, replay classification,
the error envelope, token-scope startup validation, and repository-level replay
conflicts for both adapters.

The SQLite adapter was additionally smoke-tested against a temporary database:
`/healthz` returned the protocol version, an identical replay was accepted
without a second revision, a changed payload with the same `operationId` was
reported in `push.conflicts`, `pull` returned the stored change and cursor, a
missing token returned `401`, a push outside the deployment workspace returned
`403`, and an unscoped or short service token refused to start.

No native WebView run, PostgreSQL integration run, deployment, commit or push is
implied by these results. Native and database-platform behaviour is only claimed
where it was executed.

GitNexus `detect-changes --scope all` still reports critical risk because it
covers the whole pre-existing dirty tree (95 files, 724 symbols, 471 affected
processes) and its index predates the untracked server crates. That report is
not a clean change gate and is recorded here as such, not as evidence of a
regression-free change.

## Remaining work and release blockers

1. Demonstrate desktop and CLI remote-mode parity end to end, including offline
   queueing, explicit sharing consent and cursor recovery after interruption.
2. Generate the TypeScript contracts from the Rust wire types instead of
   maintaining a second handwritten description, and add dependency gates.
3. Complete domain parity between the local core and the server protocol where
   the plan requires it. No overall GO until the masterplan verification matrix
   is demonstrated.
