# Aevio Architecture Correction and Full Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. The user explicitly forbids commits, branches, PRs, pushes, and deployments, so task checkpoints are verified working-tree states instead of commits.

**Goal:** Complete the approved Aevio target architecture so one implementation owns each business rule, the desktop remains fully standalone, and one Axum server supports SQLite and PostgreSQL through replaceable adapters.

**Architecture:** Preserve the current contracts and behavior while moving code across dependency boundaries in vertical, testable slices. Pure types and invariants move first, persistence and runtime adapters second, and compatibility re-exports remain only until every caller has moved. The final dependency direction is adapters -> services/application -> core, with persistence/infrastructure implementing ports and no framework or database dependency in `aevio-core`.

**Tech Stack:** Rust 2024, Tauri 2.11, Axum 0.8, SeaORM 2.0.4, sea-orm-sync 2.0.4, SQLite, PostgreSQL, React 19, Next.js 16, TypeScript, Bun, Turbo.

**Spec:** `docs/plan/AEVIO_MASTERPLAN_2026.md`

## Global Constraints

- Preserve all existing uncommitted changes and user data; never reset or synchronize schemas on open.
- Keep SQLite `user_version`, WAL, foreign keys, busy timeout, NULL semantics, indexes, timestamps, app-data paths, bundle identifier, IPC names, event payloads, CLI behavior, Swift sidecar protocol, iOS, and watchOS compatibility.
- Standalone desktop must require no account, server, network, Redis, or Docker.
- PostgreSQL is the default self-hosted database; server SQLite supports exactly one writer instance.
- SeaORM is the only production business-persistence API; explicit migration, PRAGMA, fixture, and diagnostic SQL exceptions stay documented.
- No raw colors or new frontend component system; shared UI uses existing shadcn components and registered semantic tokens.
- Production files changed by this plan must not exceed 500 physical lines; split by responsibility when touched.
- No new dependencies unless an existing library cannot satisfy a demonstrated requirement.
- No native WebView/app-drive/dev-screenshot run without an explicit user request.
- No commits, branches, PRs, pushes, deployments, destructive data operations, or production migrations.

## Review Focus

- Existing SQLite databases at every historical `user_version` open without reset and preserve rows, constraints, and timestamp units.
- A missing or unavailable server never blocks desktop startup, local time tracking, projects, tasks, invoices, or Coach.
- The same sync operation retried concurrently remains idempotent on both SQLite and PostgreSQL, while a changed payload with the same ID conflicts.
- Workspace/resource lookups cannot cross tenants through guessed identifiers, service tokens, or stale offline roles.
- IPC, CLI JSON, HTTP v1, Swift sidecar, iOS, and watchOS consumers retain exact public field names and error behavior.

---

### Task 1: Consolidate the Axum application and database adapter boundary

**Files:**
- Modify: `crates/aevio-http/src/*`
- Modify: `apps/api/src/app/*`
- Move/merge: `apps/server-postgres/src/app/core/database/*` into `crates/aevio-persistence/src/server/postgres/*`
- Move/merge: `apps/api/src/app/core/database/*` into `crates/aevio-persistence/src/server/sqlite/*`
- Modify: `apps/api/Cargo.toml`, `Cargo.toml`, `Cargo.lock`
- Delete after parity: `apps/server-postgres/`
- Test: `crates/aevio-http/src/*`, `apps/api/tests/*`, migrated PostgreSQL contract tests

**Interfaces:**
- Consumes: `aevio-contracts::sync`, `aevio-services::{identity,replay}`, stable `/healthz`, `/readyz`, `/api/v1/*` behavior.
- Produces: one `aevio-server` composition root selecting a SQLite or PostgreSQL adapter from validated configuration; shared handlers depend on an application-facing server repository port.

- [ ] Add repository contract tests that compile and exercise the same handler suite against both adapters; run them and observe the missing shared adapter boundary.
- [ ] Define the minimal repository port and shared Axum state/router in `aevio-http`; keep database types out of handler signatures.
- [ ] Move SQLite and PostgreSQL implementations into `aevio-persistence::server`, preserving migrations and transaction semantics.
- [ ] Make `apps/api` the only server binary/library and select the adapter by database URL/configuration.
- [ ] Run server unit/contract tests, real PostgreSQL tests when `AEVIO_TEST_POSTGRES_URL` is available, and `bun run verify`.
- [ ] Remove `apps/server-postgres` only after every test/import/runtime path resolves through `apps/api`.

### Task 2: Establish pure core, application ports, and service ownership

**Files:**
- Modify/move: `crates/aevio-core/src/*`
- Modify: `crates/aevio-application/src/*`
- Modify: `crates/aevio-services/src/*`
- Test: per-domain unit tests in the owning crate

**Interfaces:**
- Consumes: existing public domain types, validation rules, calculations, state transitions, and errors.
- Produces: `aevio-core` with only domain primitives/invariants; `aevio-application` with ports and shared orchestration primitives; `aevio-services` as sole owner of use cases.

- [ ] Add architecture tests that fail while `aevio-core` references SQLite, SeaORM, IPC, HTTP, filesystem, networking, or async runtimes.
- [ ] Inventory every public `aevio_core::*` consumer and map it to core, application, services, persistence, infrastructure, contracts, or compatibility surface.
- [ ] Move one cohesive domain at a time, beginning with low-coupling pure modules and ending with activity/Coach/invoicing; move tests with ownership.
- [ ] Replace direct cross-domain repository access with explicit application/service interfaces only where an existing use case requires it.
- [ ] Remove compatibility re-exports after all desktop, CLI, server, and tests use the owning crate.
- [ ] Run affected unit tests after every domain and the complete Rust suite after the final domain.

### Task 3: Complete SQLite and PostgreSQL persistence ownership

**Files:**
- Move: `crates/aevio-core/src/infrastructure/database/*` to `crates/aevio-persistence/src/sqlite/*`
- Move: local migrations/bootstrap persistence from `crates/aevio-core/src/bootstrap/*`
- Modify: `crates/aevio-persistence/Cargo.toml`, repository implementations, tests
- Test: migration, compatibility, query-plan, SQLite and PostgreSQL adapter tests

**Interfaces:**
- Consumes: application repository ports and core domain types.
- Produces: synchronous SQLite adapters, asynchronous PostgreSQL adapters, explicit transaction boundaries, and data-compatible migration runners.

- [ ] Add dependency and SQL-audit tests that fail on ORM construction or business SQL outside persistence/infrastructure exceptions.
- [ ] Move entities, mappings, repositories, connection policy, migrations, and diagnostics without changing schemas or data paths.
- [ ] Replace every desktop/CLI/server ORM import with repository/application interfaces.
- [ ] Test historical SQLite fixtures, WAL/foreign keys/busy-timeout, PostgreSQL migrations, replay concurrency, tenant scoping, and restart recovery.
- [ ] Update `docs/DATABASE_COMPATIBILITY.md` with the final exception inventory and run its automated audit.

### Task 4: Move runtime providers into infrastructure

**Files:**
- Modify: `crates/aevio-infrastructure/src/*`
- Move from core: IPC transport, filesystem paths/storage, HTTP clients, configuration, AI/provider runtime, secrets/jobs/events where shared
- Modify: desktop/CLI/server composition roots

**Interfaces:**
- Consumes: application ports and contracts.
- Produces: explicit adapters for local IPC, storage, configuration, provider access, and optional network capabilities.

- [ ] Add architecture tests proving core/services do not depend on infrastructure and infrastructure contains no business decisions.
- [ ] Move adapters by capability with platform-specific implementations or honest unsupported fallbacks.
- [ ] Preserve local socket/named-pipe behavior, permissions, redaction, timeouts, sidecar protocol, and offline fallback.
- [ ] Run IPC/CLI contract tests plus macOS and Windows compile checks required by repository policy.

### Task 5: Thin Tauri and CLI adapters

**Files:**
- Modify/split: `apps/desktop/tauri/src/commands.rs` and feature command modules
- Modify: `apps/cli/src/*`, `apps/cli/COMMANDS.md`
- Test: Tauri command tests, CLI JSON/smoke tests, offline startup tests

**Interfaces:**
- Consumes: services/application use cases, persistence/infrastructure composition, stable contracts.
- Produces: mapping-only IPC and CLI handlers with stable names, payloads, exit codes, and events.

- [ ] Add boundary tests that fail when commands/handlers contain SQL, ORM queries, or duplicated business validation.
- [ ] Replace direct repository access with cohesive use-case services and split changed files above 500 lines.
- [ ] Add remote CLI mode through the generated/typed API client without weakening local direct/RPC mode.
- [ ] Verify desktop bootstrap with no server configuration and CLI local/RPC/remote compatibility.

### Task 6: Finish typed contracts and shared frontend packages

**Files:**
- Modify: `crates/aevio-contracts/src/*`
- Create/populate when justified by real consumers: `packages/features/src/*`
- Modify: `packages/api-client`, `packages/types`, `packages/ui`, `packages/config`, `apps/web/src/*`
- Test: contract snapshots/generation drift, frontend unit tests, typecheck

**Interfaces:**
- Consumes: canonical Rust transport/event/sync contracts and existing UI features.
- Produces: generated TypeScript types, one typed IPC/HTTP client, and shared feature compositions used by both applicable shells.

- [ ] Add a drift test that fails while handwritten TypeScript contracts differ from canonical Rust contracts.
- [ ] Generate or deterministically export TypeScript contracts without creating parallel models.
- [ ] Centralize command/event names and replace direct raw Tauri listeners with `useTauriEvent`.
- [ ] Extract only feature/UI code with at least two real consumers; delete duplicates after import parity.
- [ ] Run frontend unit tests and `bun run verify`.

### Task 7: Complete connected offline-first synchronization

**Files:**
- Modify: `crates/aevio-services/src/sync/*`, `crates/aevio-persistence/src/*`, `crates/aevio-contracts/src/sync/*`
- Modify: desktop connectivity composition, API adapter, CLI sync commands
- Test: offline queue, retry, conflict, tombstone, cursor, consent, recovery, tenant-isolation suites

**Interfaces:**
- Consumes: registered operation envelopes, explicit workspace sharing intent, local outbox, server inbox/revision APIs.
- Produces: atomic local changes/outbox, idempotent server application, separate upload/download cursors, visible conflicts, and safe snapshot rebuild.

- [ ] Write failing end-to-end tests for disconnect/reconnect, interrupted push/pull, stale role, changed replay payload, tombstones, expired cursor, and unshared private data.
- [ ] Implement orchestration once in services with local/server adapters supplying transactions and authorization context.
- [ ] Expose progress/conflicts through stable IPC/API/CLI contracts without silent overwrite.
- [ ] Run SQLite and PostgreSQL suites, including real concurrent operations.

### Task 8: Enforce dependency, DRY, size, and platform gates

**Files:**
- Modify/create: repository architecture audit scripts under `scripts/`
- Modify: `package.json`, `justfile`, docs schemas/index as required
- Test: Node built-in test files for every rule

**Interfaces:**
- Consumes: final workspace manifests and source tree.
- Produces: deterministic checks for forbidden dependency edges, ORM/SQL leaks, duplicate contract ownership, production file size, raw event listeners, and unsupported platform shortcuts.

- [ ] Add failing fixtures/tests for every prohibited edge and known false-positive exception.
- [ ] Implement the smallest AST/metadata/text checks that enforce the rules without scanning generated/vendor output.
- [ ] Integrate the checks into `bun run verify` and prove clean output.

### Task 9: Update architecture and operational documentation

**Files:**
- Modify: `docs/architecture/*`, `docs/domains/*`, `docs/database/*`, `docs/sync/*`, `docs/security/*`, `docs/operations/*`, `docs/INDEX.md`, `README.md`, `AGENTS.md`
- Test: `node scripts/aevio-knowledge.mjs verify`

**Interfaces:**
- Consumes: verified final source, generated contracts, migrations, tests, and runtime configuration.
- Produces: source-aligned ownership matrix, dependency graph, data/sync/security flows, deployment/backup/recovery instructions, and measured verification status.

- [ ] Remove superseded architecture claims and mark unsupported behavior honestly.
- [ ] Document each app/crate/package responsibility and every compatibility exception.
- [ ] Refresh the knowledge index and run the full documentation audit.

### Task 10: Full-system verification and final review

**Files:**
- Modify only defects found by verification/review.
- Test: all canonical and scenario-specific suites.

**Interfaces:**
- Consumes: all previous tasks.
- Produces: evidence-backed GO/NO-GO report with no implied checks.

- [ ] Run `bun run fix`, `bun run verify`, frontend unit tests, full Rust tests, SQLite compatibility tests, real PostgreSQL integration tests, API/IPC/CLI contracts, standalone/offline, sync/recovery, auth/tenant isolation, and Windows cross-clippy.
- [ ] Run `graphify update .`, refresh ProjectAtlas/GitNexus, and run GitNexus `detect-changes --scope all`; resolve partial/truncated/UNKNOWN evidence explicitly.
- [ ] Perform an independent whole-tree code review, fix Critical/Important findings with RED-GREEN tests, and rerun the complete suite.
- [ ] Report changed paths, removed duplication, architecture improvements, exact pass/fail/skip counts, and remaining risks.
