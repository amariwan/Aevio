# Sync Architecture

## Protocol

- Push: `POST /api/v1/sync/push` with a validated `SyncPushRequest`
  (device id, up to 500 operations, registered entity types/operations only).
- Pull: `GET /api/v1/sync/pull?workspaceId=...&cursor=...&limit=...`.
- Registered entity type: `timeline` (`upsert`, `delete`). Unregistered types
  and operations are rejected with `INVALID_REQUEST` before storage.

## Invariants

- Revisions are workspace-scoped and advanced transactionally, never by
  timestamp.
- A push stores the domain change, the revision and the outbox entry in one
  transaction.
- Idempotency: an operation id is the primary key. Replaying the same id with
  identical content is a no-op; the same id with different content is reported
  in `conflicts` (HTTP 200 per operation), never a silent overwrite.
  The stored fingerprint is recomputed from persisted columns, never taken from
  a client hash.
- Base revisions: a push whose `base_revision` does not match the latest
  revision for the same entity is a conflict.
- Tombstones: `delete` operations remain in the operation log and are replayed
  to any device, including devices that never saw the original `upsert`.
- Cursor expiry: a client whose cursor is older than the retained log must
  rebuild from a snapshot (`pull` with cursor 0). The server retains the full
  operation log today; pruning, when introduced, will advertise a retention
  floor and set `snapshotRequired` on the pull response.

## Adapters

- SQLite server: single-instance, additive bootstrap, in-process transaction.
- PostgreSQL server: transactional with `pg_advisory_xact_lock` on schema init
  and a versioned `schema_migrations` ledger.
- Both delegate validation to `aevio-contracts`, policy to `aevio-services`,
  and persistence to SeaORM repositories.
