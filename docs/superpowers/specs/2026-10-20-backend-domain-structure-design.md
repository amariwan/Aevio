# Rust Backend Domain Structure

## Status

Design approved in chat on 2026-10-20. Implementation has not started.

## Context

Aevio currently has two Rust crates with distinct responsibilities:

- `apps/api` is the `aevio-core` library. It owns portable data access,
  SQLite migrations, domain operations, and the local IPC/RPC contract.
- `apps/desktop/tauri` is the desktop application. It owns Tauri startup,
  commands, windows, native capture and energy sampling, the Swift sidecar,
  and desktop-only event handling.

The core has useful domain modules, but several concerns are still grouped in
large files. In particular, `activity.rs` combines activity capture state,
category/client/project operations, and time-entry operations; `models.rs`
collects types from multiple domains; and the entry query helpers live at the
crate root. Invoices and break reminders currently live in the Tauri crate,
even though parts of those subsystems are independent of Tauri.

## Goals

- Organize backend Rust code by domain under `apps/api/src`.
- Keep `aevio-core` as a portable library and preserve the boundary between
  domain logic and the Tauri desktop shell.
- Extract invoice, break, and energy domain logic and persistence into the core.
- Preserve existing behavior, serialized shapes, database schema, and
  externally used Rust paths through compatibility re-exports.
- Keep tests beside the domain code they protect.

## Non-goals

- Do not turn `apps/api` into an executable or add `crates/aevio-core/src/main.rs`.
- Do not move Tauri startup, webview/window management, tray behavior, native
  capture, native energy sampling, OS APIs, or desktop event publishing into
  the core.
- Do not change SQLite schema, IPC/RPC behavior, frontend structure, or user
  experience.
- Do not create empty modules only to imitate the example tree.

## Proposed Structure

```text
crates/aevio-core/src/
├── lib.rs                    # crate facade and stable re-exports
├── activity/                 # capture state, activity storage, snapshots
├── ai/                       # existing rules, arbitration, calibration, store
├── capture/                  # portable capture sample types
├── entries/                  # entry construction, querying, entry operations
├── projects/                 # project operations, discovery, import, hints
├── clients/                  # client domain types and operations
├── categories/               # category domain types and operations
├── timers/                   # timer domain and persistence
├── reports/                  # queries, rollups, and exports
├── invoices/                 # invoice domain, money, profile, PDF rendering
├── settings/                 # settings types, validation, persistence
├── breaks/                   # portable break state machine and persistence
├── energy/                   # energy models, calculations, and persistence
├── ipc/                      # local transport and protocol/RPC integration
├── bootstrap/                # storage paths, state setup, migrations
└── models.rs                 # compatibility re-exports during the move
```

Exact internal filenames will be settled in the implementation plan after
examining visibility constraints and existing tests. The responsibilities and
crate boundary above are the design contract.

`platform` is intentionally not a top-level core module: OS-specific
implementations remain in `apps/desktop/tauri`. The existing platform-specific
IPC transport remains encapsulated under `ipc`. `energy` contains platform-neutral models, aggregation/calculation, and
SQLite access. Sampling, native battery/process queries, and event publishing
stay in the desktop crate.

## Domain Boundaries and Data Flow

The webview and CLI continue to call the Tauri command/RPC layer as they do
today. That layer delegates durable operations to `aevio-core`; the core
continues to own SQLite operations and domain invariants. Tauri continues to
own operating-system integrations and publishes events for the webview.

Invoice operations (invoice models, integer-cent calculations, profile
handling, finalization, SQLite operations, PDF byte rendering, and archived
PDF retrieval) move to `apps/api`. Desktop file dialogs and writing an
exported PDF to a user-selected path remain in Tauri. For breaks, the state
machine and SQLite store move to `apps/api`;
Tauri continues to drive the engine from app lifecycle/activity ticks and
manages reminder windows, event publication, and platform effects. Only pure
energy models, aggregation, calculations, and sample persistence/querying
move; sampling and native telemetry stay in Tauri. The Tauri sampler calls
the core energy API rather than executing SQL itself.

Client and category operations are separated from the current activity
implementation without changing their database ownership. Entry queries and
builder logic move into `entries`; existing `ActivityStore` behavior remains
available to callers.

## Compatibility and Error Behavior

- Keep `aevio-core` as the Cargo package/library and retain the current
  `lib.rs` public entry points.
- Re-export moved types/functions from the paths currently used by
  `apps/desktop/tauri` and `apps/cli` where practical.
- Keep serialized names, SQL statements, transaction boundaries, error
  messages, and IPC/RPC payloads unchanged.
- Make no silent fallbacks or success-shaped error changes as part of the
  refactor.

## Documentation and Validation

Update the README's “Under the hood” diagram to show the core/Tauri split and
the new domain layout. No version bump is required because this is an internal
refactor with no user-facing behavior change.

Validation should include:

1. `bun run fix`
2. `cargo test --manifest-path apps/api/Cargo.toml`
3. `cargo test --manifest-path apps/desktop/tauri/Cargo.toml`
4. `bun run verify`

The implementation is complete only when these checks pass and existing
Tauri/CLI callers continue to compile against the compatibility facade.

## Risks and Mitigations

- **Cross-domain `ActivityStore` methods:** splitting a large implementation
  can expose Rust privacy and ownership constraints. Keep shared storage
  invariants centralized, use narrow module visibility, and preserve the
  existing store API.
- **Invoice dependency ownership:** PDF rendering and logo handling may
  require moving Rust dependencies from the Tauri crate to `aevio-core`.
  Move only dependencies required by invoice behavior and retain Tauri-only
  file dialogs and path writes in the desktop crate.
- **Break engine integration:** the pure state machine currently interacts
  with Tauri-owned runtime state. Separate pure decisions/effects from the
  Tauri driver; do not move window or event behavior into the core.
- **Public-path churn:** compatibility re-exports keep desktop and CLI
  callers stable while internal modules move.
- **Scope expansion:** if an operation cannot be separated from a native
  adapter without changing behavior, leave that adapter in Tauri and expose
  only a portable boundary from the core.
- **Energy sampler ownership:** the sampler remains a Tauri runtime task, but
  it must persist and query samples through core APIs; it must not keep direct
  SQL statements in the Tauri crate.
