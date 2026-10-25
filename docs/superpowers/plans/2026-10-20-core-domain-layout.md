# Rust Core Domain Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize the existing `aevio-core` code into domain-oriented modules while preserving all existing behavior and public Rust paths.

**Architecture:** Keep `apps/api` as the `aevio-core` library and keep Tauri runtime code in `apps/desktop/tauri`. Move core-owned persistence and domain operations into focused module directories, using compatibility re-exports in `lib.rs`, `models.rs`, and legacy module paths while Tauri and CLI consumers continue to compile.

**Tech Stack:** Rust 2021, Cargo workspace, rusqlite, serde, chrono.

**Spec:** `docs/superpowers/specs/2026-10-20-backend-domain-structure-design.md`

## Global Constraints

- `apps/api` remains a portable library; do not add `crates/aevio-core/src/main.rs`.
- Preserve existing behavior, serialized shapes, database schema, and externally used Rust paths through compatibility re-exports.
- Do not move Tauri startup, webview/window management, tray behavior, native capture, native energy sampling, OS APIs, or desktop event publishing into the core.
- Do not change SQLite schema, IPC/RPC behavior, frontend structure, or user experience.
- Do not create empty modules only to imitate the example tree.
- Do not bump the patch version; this is an internal refactor with no user-facing behavior change.
- The current repository has no existing `HEAD` and its project files are untracked; do not create a commit or stage unrelated files.

## Execution Order

Run this foundational plan before the invoice, break, and energy extraction plans. Each extraction plan is separately testable after the Core module facades exist.

## File Ownership Map

- `crates/aevio-core/src/lib.rs`: crate facade and compatibility exports only.
- `crates/aevio-core/src/activity/`: activity capture state, live snapshots, and shared store definition.
- `crates/aevio-core/src/entries/`: entry types, construction, queries, and entry store operations.
- `crates/aevio-core/src/projects/`, `clients/`, and `categories/`: domain types and storage operations.
- `crates/aevio-core/src/bootstrap/`: migrations, data paths, and app storage state.
- `crates/aevio-core/src/ipc/`: local transport, protocol, and RPC implementations.
- `crates/aevio-core/src/ai/`, `capture/`, `timers/`, `reports/`, and `settings/`: existing domain logic organized as folder-backed modules.
- `crates/aevio-core/src/models.rs`: stable re-exports for callers using the old shared type paths.

---

### Task 1: Establish the Core Baseline

**Files:**
- Test: `apps/api/Cargo.toml`
- Test: `apps/desktop/tauri/Cargo.toml`
- Test: `apps/cli/Cargo.toml`

**Interfaces:**
- Consumes: current workspace state.
- Produces: a recorded passing test baseline for the core and Tauri consumers.

- [ ] **Step 1: Run the core test suite**

Run: `cargo test --manifest-path apps/api/Cargo.toml`
Expected: PASS before any moves.

- [ ] **Step 2: Run the Tauri test suite**

Run: `cargo test --manifest-path apps/desktop/tauri/Cargo.toml`
Expected: PASS before any moves.

- [ ] **Step 3: Run the CLI test suite**

Run: `cargo test --manifest-path apps/cli/Cargo.toml`
Expected: PASS before any moves.

### Task 2: Group Bootstrap and IPC Infrastructure

**Files:**
- Create: `crates/aevio-core/src/bootstrap/mod.rs`
- Move: `crates/aevio-core/src/migrations.rs` to `crates/aevio-core/src/bootstrap/migrations.rs`
- Move: `crates/aevio-core/src/paths.rs` to `crates/aevio-core/src/bootstrap/paths.rs`
- Move: `crates/aevio-core/src/state.rs` to `crates/aevio-core/src/bootstrap/state.rs`
- Create: compatibility modules `crates/aevio-core/src/migrations.rs`, `crates/aevio-core/src/paths.rs`, `crates/aevio-core/src/state.rs`
- Move: `crates/aevio-core/src/ipc.rs` to `crates/aevio-core/src/ipc/mod.rs`
- Move: `crates/aevio-core/src/protocol.rs` to `crates/aevio-core/src/ipc/protocol.rs`
- Move: `crates/aevio-core/src/rpc.rs` to `crates/aevio-core/src/ipc/rpc.rs`
- Create: compatibility modules `crates/aevio-core/src/protocol.rs`, `crates/aevio-core/src/rpc.rs`
- Modify: `crates/aevio-core/src/lib.rs`

**Interfaces:**
- Consumes: the current `migrations`, `paths`, `state`, and `ipc` modules.
- Produces: `bootstrap::{migrations, paths, state}` and `ipc::{protocol, rpc}` as implementation modules, with unchanged legacy public paths.

- [ ] **Step 1: Add the bootstrap module and wire the implementation files**

Create `bootstrap/mod.rs` with:

```rust
pub mod migrations;
pub mod paths;
pub mod state;
```

Keep the top-level modules declared in `lib.rs` during the move:

```rust
pub mod bootstrap;
pub mod migrations;
pub mod paths;
pub mod state;
pub mod ipc;
pub mod protocol;
pub mod rpc;
```

- [ ] **Step 2: Add legacy re-export facades**

Replace the moved implementation in each old module with the matching re-export:

```rust
pub use crate::bootstrap::migrations::*;
```

Use the same pattern for `paths` and `state`, changing only the module name.

- [ ] **Step 3: Move the IPC transport module without changing its platform selectors**

Move `ipc.rs` to `ipc/mod.rs`. Keep `ipc/unix.rs` and `ipc/windows.rs` in their current `ipc/` directory and verify the existing `#[cfg]` declarations still resolve from the new module path.

- [ ] **Step 4: Move protocol and RPC implementations under IPC**

Move `protocol.rs` and `rpc.rs` to `ipc/protocol.rs` and `ipc/rpc.rs`, then declare them in `ipc/mod.rs`:

```rust
pub mod protocol;
pub mod rpc;
```

Keep the original root module paths with facades:

```rust
pub use crate::ipc::protocol::*;
```

Use the corresponding `rpc` path in `rpc.rs`, and declare both child modules in `ipc/mod.rs`.

- [ ] **Step 5: Run focused crate tests**

Run: `cargo test --manifest-path apps/api/Cargo.toml`
Expected: PASS with the old paths still available.

### Task 3: Create the Entries Domain

**Files:**
- Create: `crates/aevio-core/src/entries/mod.rs`
- Move: `crates/aevio-core/src/entry_builder.rs` to `crates/aevio-core/src/entries/builder.rs`
- Create: `crates/aevio-core/src/entries/query.rs`
- Move: `crates/aevio-core/src/capture.rs` to `crates/aevio-core/src/capture/mod.rs`
- Create: compatibility module `crates/aevio-core/src/entry_builder.rs`
- Modify: `crates/aevio-core/src/lib.rs`

**Interfaces:**
- Consumes: current entry-builder functions and the crate-root `EntryFilter`, `Clause`, `time_entry_from_row`, and `query_entries`.
- Produces: folder-backed `capture` and `entries::{builder, query}` modules; existing root and `entry_builder` exports remain callable.

- [ ] **Step 1: Declare the new entries module**

Create `entries/mod.rs`:

```rust
pub mod builder;
pub mod query;
```

Add `pub mod entries;` to `lib.rs`.

Move `capture.rs` to `capture/mod.rs`; keep `pub mod capture;` unchanged in `lib.rs`.

- [ ] **Step 2: Move builder implementation and retain its legacy path**

Move the complete implementation and its tests from `entry_builder.rs` to `entries/builder.rs`. Replace the old file body with:

```rust
pub use crate::entries::builder::*;
```

- [ ] **Step 3: Move query types and functions into `entries/query.rs`**

Move `EntryFilter`, `Clause`, their private helpers, `time_entry_from_row`, and `query_entries` out of `lib.rs`. Re-export the existing public root names:

```rust
pub use entries::query::{
    query_entries, time_entry_from_row, Clause, EntryFilter, APP_KEY, MAX_QUERY_ROWS, NONE,
};
```

- [ ] **Step 4: Run Core tests**

Run: `cargo test --manifest-path apps/api/Cargo.toml`
Expected: PASS; no SQL or filter behavior changes.

### Task 4: Separate Domain Types and ActivityStore Responsibilities

**Files:**
- Create: `crates/aevio-core/src/categories/mod.rs`
- Create: `crates/aevio-core/src/categories/types.rs`
- Create: `crates/aevio-core/src/categories/store.rs`
- Create: `crates/aevio-core/src/clients/mod.rs`
- Create: `crates/aevio-core/src/clients/types.rs`
- Create: `crates/aevio-core/src/clients/store.rs`
- Create: `crates/aevio-core/src/activity/mod.rs`
- Create: `crates/aevio-core/src/activity/store.rs`
- Move: `crates/aevio-core/src/projects.rs` to `crates/aevio-core/src/projects/mod.rs`
- Create: `crates/aevio-core/src/projects/types.rs`
- Create: `crates/aevio-core/src/projects/store.rs`
- Create: `crates/aevio-core/src/entries/types.rs`
- Create: `crates/aevio-core/src/entries/store.rs`
- Create: `crates/aevio-core/src/activity/types.rs`
- Create: `crates/aevio-core/src/activity/snapshots.rs`
- Modify: `crates/aevio-core/src/models.rs`
- Modify: `crates/aevio-core/src/lib.rs`
- Move: `crates/aevio-core/src/activity.rs` to the new `activity/` implementation modules

**Interfaces:**
- Consumes: the existing `ActivityStore` methods and all public model types in `models.rs`.
- Produces: domain-owned type and persistence modules plus a stable `models` re-export facade and `activity::ActivityStore`.

- [ ] **Step 1: Move the existing activity implementation as a no-behavior-change module conversion**

Convert `activity.rs` into `activity/mod.rs`, keeping all code in that module initially. Add `pub mod activity;` to `lib.rs` and run the Core tests before extracting methods.

- [ ] **Step 2: Extract the ActivityStore definition and shared storage helpers**

Move `ActivityStore`, connection setup, and shared SQL helpers into `activity/store.rs`. Re-export the type from `activity/mod.rs`:

```rust
mod snapshots;
mod store;
pub mod types;
pub use store::ActivityStore;
```

Declare `types` and `store` in `categories/mod.rs`, `clients/mod.rs`, and `projects/mod.rs`; declare `types` and `store` in `entries/mod.rs`. Keep existing activity regression/test submodules declared as they are today. Use narrow `pub(crate)` accessors for shared connection/store state required by sibling domain modules; never expose the SQLite connection as a public field.

- [ ] **Step 3: Move domain models and preserve `models`**

Move category types to `categories/types.rs`, client types to `clients/types.rs`, project types to `projects/types.rs`, entry types to `entries/types.rs`, activity segments to `activity/types.rs`, and classification/suggestion types to the existing `ai` module. Re-export types from their domain modules and keep `models.rs` as a facade so imports such as `aevio_core::models::TimeEntry` remain valid. Add `pub mod categories;` and `pub mod clients;` to `lib.rs`; use domain module imports internally instead of routing new code through `models.rs`.

- [ ] **Step 4: Move category operations**

Move `list_categories`, `create_category`, `update_category`, and `delete_category` into `categories/store.rs` as `impl ActivityStore` methods. Preserve SQL, validation, and return types.

Run: `cargo test --manifest-path apps/api/Cargo.toml`
Expected: PASS.

- [ ] **Step 5: Move client operations**

Move `list_clients`, `create_client`, `update_client`, and `delete_client` into `clients/store.rs` as `impl ActivityStore` methods. Preserve archive and invoice-pinning behavior.

Run: `cargo test --manifest-path apps/api/Cargo.toml`
Expected: PASS.

- [ ] **Step 6: Move project CRUD**

Move `list_projects`, `create_project`, `update_project`, and `delete_project` into `projects/store.rs` as `impl ActivityStore` methods. Preserve atomic deletion rules and all current project tests.

Run: `cargo test --manifest-path apps/api/Cargo.toml`
Expected: PASS.

- [ ] **Step 7: Move time-entry reads**

Move `list_time_entries`, `time_entries`, and `get_entry_detail` into `entries/store.rs`. Keep the query/filter implementation in `entries/query.rs`; preserve ordering, limits, and joins.

Run: `cargo test --manifest-path apps/api/Cargo.toml`
Expected: PASS.

- [ ] **Step 8: Move time-entry mutations**

Move update, bulk update, approve, reject, unapprove, rule resolution, delete, and manual-entry methods into `entries/store.rs`. Keep lock/overlap helpers local to the narrowest module that uses them.

Run: `cargo test --manifest-path apps/api/Cargo.toml`
Expected: PASS.

- [ ] **Step 9: Move entry rebuild and activity snapshot operations**

Move time-entry rebuild methods into `entries/store.rs`. Move snapshot/tick/live-state and app listing methods into `activity/snapshots.rs`; preserve event payload types and all existing activity tests.

Run: `cargo test --manifest-path apps/api/Cargo.toml`
Expected: PASS.

### Task 5: Convert Existing Flat Domain Modules to Directories

**Files:**
- Move: `crates/aevio-core/src/reports.rs` to `crates/aevio-core/src/reports/mod.rs`
- Move: `crates/aevio-core/src/settings.rs` to `crates/aevio-core/src/settings/mod.rs`
- Move: `crates/aevio-core/src/timers.rs` to `crates/aevio-core/src/timers/mod.rs`
- Modify: `crates/aevio-core/src/lib.rs`

**Interfaces:**
- Consumes: the existing public modules and tests.
- Produces: folder-backed modules with unchanged public item paths.

- [ ] **Step 1: Convert the flat files to directory modules**

Move each file to its corresponding `mod.rs`. Declare the same module names in `lib.rs`:

```rust
pub mod projects;
pub mod reports;
pub mod settings;
pub mod timers;
```

- [ ] **Step 2: Keep tests beside the implementation**

Move each module's inline tests with its implementation. Do not alter assertions or expected errors as part of a file move.

- [ ] **Step 3: Run Core tests**

Run: `cargo test --manifest-path apps/api/Cargo.toml`
Expected: PASS.

### Task 6: Update Architecture Documentation and Verify Consumers

**Files:**
- Modify: `README.md`
- Test: `apps/api/Cargo.toml`
- Test: `apps/desktop/tauri/Cargo.toml`
- Test: `apps/cli/Cargo.toml`

**Interfaces:**
- Consumes: the completed module layout.
- Produces: accurate documentation and passing Core/Tauri consumers.

- [ ] **Step 1: Update the README architecture diagram**

Show `apps/api` as the portable Rust core, `apps/desktop/tauri` as the runtime/platform shell, and the domain folders completed by this plan. Keep the existing statements about durable state and event direction accurate; the invoice, break, and energy plans update their own ownership notes after those extractions.

- [ ] **Step 2: Format the repository**

Run: `bun run fix`
Expected: formatting and safe lint fixes complete without unrelated file changes.

- [ ] **Step 3: Run Core and Tauri tests**

Run: `cargo test --manifest-path apps/api/Cargo.toml && cargo test --manifest-path apps/desktop/tauri/Cargo.toml && cargo test --manifest-path apps/cli/Cargo.toml`
Expected: Core, Tauri, and CLI suites PASS.

- [ ] **Step 4: Run canonical verification**

Run: `bun run verify`
Expected: PASS.
