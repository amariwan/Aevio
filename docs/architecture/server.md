---
id: arch.server
title: Aevio HTTP Server Architecture
category: architecture
domain: platform
status: implemented
source_paths:
  - apps/api/src/app
  - crates/aevio-http/src
  - crates/aevio-services/src
  - crates/aevio-contracts/src
related_docs:
  - docs/architecture/overview.md
  - docs/architecture/rust-core.md
  - docs/architecture/masterplan-progress.md
platforms: [macos, windows]
verification_commit: working-tree
---

# Aevio HTTP Server Architecture

The two HTTP backends (`aevio-server` on SQLite, `aevio-server-postgres` on
PostgreSQL) follow the same composition model, adapted from `base-fastApi` to
Rust and Axum. `main.rs` is only the process entrypoint; application composition
lives under `src/app/`.

## Responsibilities

- `config` loads and validates environment configuration once at startup,
  including the service-token scope.
- `state` holds the per-process state: database handle, deployment token,
  optional workspace, registration flag and resolved service scope.
- `authz` turns a bearer credential into an `Actor` and authorizes one workspace
  operation. It contains transport mechanics only; the policy itself lives in
  `aevio-services::identity`.
- `views` owns Axum route registration and HTTP request/response adaptation,
  split into `health`, `auth`, `workspaces` and `sync`.
- `core` contains technical capabilities and persistence adapters.
- `shared` re-exports the stable protocol and error contracts; it contains no
  handlers or SQL.
- `services` contains only the password/token helpers shared by the handlers.

SQLite and PostgreSQL keep their database adapters separate but share the same
module layout, the same error contract and the same policy crates. The local
`aevio-core` crate remains the durable desktop domain core and is not duplicated
by the HTTP layer.

## Dependency direction

`main` -> `app::views` -> `app::authz` -> `aevio-services` policy and
`app::core` persistence. `shared` is a contract-only layer and must not depend
on handlers or SQL. Database initialization and historical schema bootstrap
remain in the database adapter; request handlers never execute SQL.

## Shared contracts and policies

`crates/aevio-contracts` owns the v1 sync envelopes, the registered-operation
limits, push validation and the canonical payload fingerprint. Both adapters,
the local timeline and the desktop client consume those types.

`crates/aevio-services` owns the workspace role hierarchy, service-token scope,
session lifetime, membership-change invariants and replay classification. It has
no HTTP or ORM dependency, and both adapters delegate to it instead of
re-implementing the rules.

`crates/aevio-http` owns the single HTTP error contract:

```json
{ "error": { "code": "FORBIDDEN", "message": "workspace role is not sufficient" } }
```

Codes are stable uppercase identifiers (`UNAUTHENTICATED`, `FORBIDDEN`,
`NOT_FOUND`, `INVALID_REQUEST`, `CONFLICT`, `REGISTRATION_DISABLED`,
`INTERNAL`) and map one-to-one onto HTTP status codes; internal failures are
logged by the server and returned as `internal server error` so database or path
details never reach the client. Both adapters return this shape.

## Service tokens

`AEVIO_SERVER_TOKEN` is a deployment credential, not a super user. It becomes a
scoped `Actor::Service`:

- `AEVIO_SERVER_TOKEN_WORKSPACES` (fallback: `AEVIO_SERVER_WORKSPACE`) names the
  workspaces it may use. An empty scope is a startup error.
- `AEVIO_SERVER_TOKEN_PERMISSIONS` defaults to `read,write`; `admin` must be
  requested explicitly. A service actor is never a workspace owner.
- When `AEVIO_SERVER_WORKSPACE` is set, the token scope cannot exceed it.

## Remaining work

The separation is implemented for both adapters; the remaining gaps are
verification and operations rather than structure: real PostgreSQL integration
tests, backup/restore and health/readiness capabilities, domain-parity proof for
the remote client, generated TypeScript contracts and dependency gates. See
[the migration evidence ledger](masterplan-progress.md).
