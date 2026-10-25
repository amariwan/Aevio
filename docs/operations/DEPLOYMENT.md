---
id: operations.deployment
title: Server Deployment
category: operations
domain: operations
status: implemented
source_paths: [apps/api/src/app/views/mod.rs, apps/api/src/app/config.rs, apps/api/src/app/core/database/mod.rs]
related_docs: [docs/operations/BACKUP_RECOVERY.md, apps/api/README.md, docs/architecture/masterplan-progress.md]
platforms: [macos, linux]
verification_commit: working-tree
---

# Server Deployment

Aevio has two server adapters over the same `/api/v1` contract:

| Adapter | Storage | Suitable for |
| --- | --- | --- |
| `aevio-server` | SQLite (`sea-orm-sync`) | exactly one process, small self-hosted installs |
| `aevio-server-postgres` | PostgreSQL (`sea-orm` async) | normal self-hosted deployments, backups, multi-instance reads |

The desktop client never needs either server. Deploy one only when team
collaboration is wanted.

## PostgreSQL server

Required environment:

| Variable | Meaning |
| --- | --- |
| `AEVIO_SERVER_DATABASE_URL` | `postgres://user:password@host/database` |
| `AEVIO_SERVER_TOKEN` | service token, at least 16 characters |
| `AEVIO_SERVER_TOKEN_WORKSPACES` (or `AEVIO_SERVER_WORKSPACE`) | comma-separated workspace ids the token may touch |
| `AEVIO_SERVER_TOKEN_PERMISSIONS` | subset of `read,write,admin`; default `read,write` |
| `AEVIO_SERVER_BIND` | listen address, default `127.0.0.1:47832` |

Optional:

- `AEVIO_SERVER_ADMIN_EMAIL` + `AEVIO_SERVER_ADMIN_PASSWORD` bootstrap an owner
  account (both or neither).
- `AEVIO_SERVER_ALLOW_REGISTRATION=true` enables self-registration.
- `AEVIO_SERVER_WORKSPACE` restricts the process to one workspace and caps the
  service token scope.
- `AEVIO_SERVER_WORKSPACE_NAME` names the bootstrapped workspace.

Startup applies the versioned schema migrations once, then binds the listener.
A database that already holds the historical schema is adopted without touching
its rows (`schema_migrations` is created and version 1 is recorded). See
`apps/server-postgres/src/app/core/database/migrations.rs`.

```sh
AEVIO_SERVER_DATABASE_URL='postgres://aevio:secret@127.0.0.1/aevio' \
AEVIO_SERVER_TOKEN='replace-with-a-long-random-token' \
AEVIO_SERVER_TOKEN_WORKSPACES='workspace-main' \
AEVIO_SERVER_BIND='127.0.0.1:47832' \
cargo run --release -p aevio-server-postgres
```

Terminate TLS at a reverse proxy; the server itself speaks plain HTTP and is
meant to run on a private interface. Do not expose the port publicly without
TLS and a firewall rule.

## Exactly one SQLite server instance

The SQLite adapter is a single-writer store. Running two processes against the
same file leaves sync cursors and outbox processing inconsistent. Use
PostgreSQL when more than one process must serve requests.

## Health checks

`GET /healthz` proves that the HTTP process is alive. `GET /readyz` additionally
pings the configured SQLite or PostgreSQL storage and returns `503` with
`{"status":"unavailable",...}` when the dependency cannot serve requests.
Orchestrators must use `/readyz` for traffic admission and `/healthz` only for
process-liveness decisions.

## Compose sketch

```yaml
services:
  postgres:
    image: postgres:17-alpine
    environment:
      POSTGRES_DB: aevio
      POSTGRES_USER: aevio
      POSTGRES_PASSWORD: ${AEVIO_DB_PASSWORD:?set in .env}
    volumes:
      - aevio-postgres:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U aevio -d aevio"]
      interval: 10s
      timeout: 5s
      retries: 5

  server:
    image: aevio-server-postgres:local
    environment:
      AEVIO_SERVER_DATABASE_URL: postgres://aevio:${AEVIO_DB_PASSWORD}@postgres/aevio
      AEVIO_SERVER_TOKEN: ${AEVIO_SERVER_TOKEN:?set in .env}
      AEVIO_SERVER_TOKEN_WORKSPACES: workspace-main
      AEVIO_SERVER_BIND: 0.0.0.0:47832
    depends_on:
      postgres:
        condition: service_healthy
    ports:
      - "127.0.0.1:47832:47832"

volumes:
  aevio-postgres:
```

The image build and the reverse-proxy recipe are deployment packaging, not part
of the verified gate suite. Backup and restore are covered in
`docs/operations/BACKUP_RECOVERY.md`.
