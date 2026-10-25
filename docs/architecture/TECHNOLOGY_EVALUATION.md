# Technology Evaluation

## PowerSync (isolated POC decision)

Decision: **Do not adopt PowerSync.** Keep the transactional outbox / cursor
protocol.

Findings:

- PowerSync is delivered as a sync *service* plus client SDKs (Flutter, React
  Native, JavaScript/TypeScript, Kotlin, Swift). There is no Rust SDK, so it
  cannot be embedded in the Tauri desktop's Rust core or in the CLI without a
  JavaScript bridge, which would split the sync ownership.
- Self-hosting requires the PowerSync Service (Docker, source-available
  license), adding an operational dependency that contradicts the
  single-binary / compose-lightweight goal.
- The existing protocol already provides the required guarantees: typed
  operations, idempotency via recomputed fingerprints, base-revision conflicts,
  tombstones, cursor resumption, tenant isolation and a real PostgreSQL
  backend. Replacing it would discard verified behaviour for no proven gain.
- PowerSync remains an option for a future web-only client if the browser
  client is decoupled from the Rust core; that decision is deferred until a
  web client exists and is tracked as a separate ADR.

Conclusion: the current outbox/cursor mechanism stays. No migration.
