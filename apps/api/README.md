# Aevio Sync Server

The sync server is a self-hosted HTTP service for shared timeline publications.
It never receives the local `segments` database; clients submit explicit
operations only. Accounts use Argon2 password hashes and access tokens are
stored only as SHA-256 hashes.

## Run locally

```sh
AEVIO_SERVER_TOKEN=change-me-with-16-chars \
AEVIO_SERVER_ADMIN_EMAIL=owner@example.com \
AEVIO_SERVER_ADMIN_PASSWORD=use-a-secret-password \
AEVIO_SERVER_WORKSPACE=demo \
cargo run -p aevio-server
```

Configuration:

- `AEVIO_SERVER_TOKEN` is required (minimum 16 characters) and authenticates
  service requests as a scoped actor, never as a super user.
- `AEVIO_SERVER_TOKEN_WORKSPACES` lists the workspaces the service token may
  use; it falls back to `AEVIO_SERVER_WORKSPACE`. An empty scope is a startup
  error.
- `AEVIO_SERVER_TOKEN_PERMISSIONS` defaults to `read,write`; add `admin`
  explicitly to allow membership changes. A service token never becomes owner.
- `AEVIO_SERVER_WORKSPACE` optionally restricts the process to one workspace.
  The token scope cannot exceed this workspace.
- `AEVIO_SERVER_ADMIN_EMAIL` and `AEVIO_SERVER_ADMIN_PASSWORD` optionally
  bootstrap an owner account. Both must be configured together.
- `AEVIO_SERVER_ALLOW_REGISTRATION=true` enables account registration; it is
  disabled by default.
- `AEVIO_SERVER_WORKSPACE_NAME` optionally names the bootstrapped workspace.
- `AEVIO_SERVER_DATABASE` defaults to `aevio-server.db`.
- `AEVIO_SERVER_DATABASE_URL` selects PostgreSQL when set to a `postgres://` or
  `postgresql://` URL. It takes precedence over `AEVIO_SERVER_DATABASE`.
- `AEVIO_SERVER_BIND` defaults to `127.0.0.1:47832`.

The protocol is versioned at `/api/v1`:

- `POST /auth/login` returns an access token for a bootstrapped account.
- `POST /auth/register` creates an account when registration is explicitly
  enabled.
- `GET /workspaces` lists workspaces for the authenticated account.
- `POST /workspaces/{id}/members` adds or changes an existing account with role
  `owner`, `admin`, `member` or `viewer`. The last owner cannot be demoted, and
  `admin` cannot grant `owner`.
- `POST /sync/push` is allowed for members, admins and owners.
- `GET /sync/pull` is allowed for all workspace members.

Errors use one shape for both persistence adapters:
`{"error":{"code":"FORBIDDEN","message":"..."}}` with stable uppercase codes.
Internal failures are logged by the server and returned as `internal server
error`.

`push` is idempotent by `operationId` and the payload fingerprint; replaying the
same operation with the same content is accepted, while a changed payload is
reported as a per-operation conflict. `pull` uses a monotonically increasing
workspace cursor. A stale `baseRevision` returns a conflict instead of silently
overwriting a change. Access tokens expire 30 days after issue; `revoked_at`
always wins.

The single `aevio-server` binary supports SQLite and PostgreSQL. The selected
database changes only the persistence adapter; routes, authorization, sync
orchestration, protocol, and error handling are shared.

The service must be placed behind HTTPS when exposed beyond localhost.
