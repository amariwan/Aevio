# Break Domain Core Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move portable break state transitions and SQLite persistence into `aevio-core`, retaining the Tauri driver, reminder surfaces, event publishing, and desktop effects.

**Architecture:** The Core exposes the break engine and store as portable modules. The existing Tauri `breaks` module remains the runtime adapter: it reads settings and app state, drives the engine, applies effects, manages reminder windows, and publishes events through the unchanged command/event boundary.

**Tech Stack:** Rust 2021, Cargo, rusqlite, serde, chrono, Tauri 2.

**Spec:** `docs/superpowers/specs/2026-10-20-backend-domain-structure-design.md`

## Global Constraints

- `apps/api` remains a portable library; do not add `crates/aevio-core/src/main.rs`.
- Preserve existing behavior, serialized shapes, database schema, and externally used Rust paths through compatibility re-exports.
- Do not move Tauri startup, webview/window management, tray behavior, native capture, native energy sampling, OS APIs, or desktop event publishing into the core.
- Do not change SQLite schema, IPC/RPC behavior, frontend structure, or user experience.
- The Core owns break domain decisions and durable records; Tauri owns runtime effects and UI.
- Do not bump the patch version; this is an internal refactor with no user-facing behavior change.
- The current repository has no existing `HEAD` and its project files are untracked; do not create a commit or stage unrelated files.

## File Ownership Map

- `crates/aevio-core/src/breaks/engine.rs`: portable break state machine and domain types.
- `crates/aevio-core/src/breaks/store.rs`: break records and SQLite operations.
- `apps/desktop/tauri/src/breaks/mod.rs`: app-state driver, lifecycle/tick handling, and effect application.
- `apps/desktop/tauri/src/breaks/surface.rs`: reminder window management and platform-specific surface behavior.
- `apps/desktop/tauri/src/breaks/stopwatch.rs`: desktop stopwatch reminder scheduling.

---

### Task 1: Establish Break Test Baseline

**Files:**
- Test: `apps/desktop/tauri/src/breaks/engine.rs`
- Test: `apps/desktop/tauri/src/breaks/store.rs`
- Test: `apps/desktop/tauri/Cargo.toml`

**Interfaces:**
- Consumes: current Tauri break engine/store.
- Produces: passing baseline for the state machine and persistence.

- [ ] **Step 1: Run the existing break tests**

Run: `cargo test --manifest-path apps/desktop/tauri/Cargo.toml breaks::`
Expected: PASS before moving modules.

### Task 2: Move Portable Engine and Store into `aevio-core`

**Files:**
- Create: `crates/aevio-core/src/breaks/mod.rs`
- Move: `apps/desktop/tauri/src/breaks/engine.rs` to `crates/aevio-core/src/breaks/engine.rs`
- Move: `apps/desktop/tauri/src/breaks/store.rs` to `crates/aevio-core/src/breaks/store.rs`
- Modify: `crates/aevio-core/src/lib.rs`
- Modify: `apps/api/Cargo.toml` only if a required portable dependency is absent

**Interfaces:**
- Consumes: current `BreakRecord`, `BreakState`, `Command`, `Effect`, `Engine`, `Input`, `LocalTime`, `SegmentNow`, `store::BreakEntry`, and store functions.
- Produces: equivalent APIs under `aevio_core::breaks::{engine,store}`.

- [ ] **Step 1: Check that the engine and store are Tauri-independent**

Search `engine.rs` and `store.rs` for `tauri::`, `AppHandle`, `Manager`, and `Emitter`. These modules must not depend on Tauri; keep runtime-dependent functions in the Tauri adapter. Leave `stopwatch.rs` in Tauri because it participates in the desktop reminder driver.

- [ ] **Step 2: Declare the Core break module**

Create `crates/aevio-core/src/breaks/mod.rs`:

```rust
pub mod engine;
pub mod store;
```

Add `pub mod breaks;` to `crates/aevio-core/src/lib.rs`.

- [ ] **Step 3: Move implementation and tests**

Move the pure state machine, persistence functions, and their tests without changing algorithms, SQL, error strings, or assertions. Keep stopwatch scheduling in the Tauri adapter.

- [ ] **Step 4: Run Core break tests**

Run: `cargo test --manifest-path apps/api/Cargo.toml breaks::`
Expected: PASS.

### Task 3: Keep Tauri as the Break Runtime Adapter

**Files:**
- Modify: `apps/desktop/tauri/src/breaks/mod.rs`
- Keep: `apps/desktop/tauri/src/breaks/stopwatch.rs`
- Keep: `apps/desktop/tauri/src/breaks/surface.rs`
- Modify: `apps/desktop/tauri/src/commands.rs` only where module re-exports do not preserve existing paths

**Interfaces:**
- Consumes: `aevio_core::breaks::{engine,store}`.
- Produces: unchanged Tauri commands/events and unchanged reminder behavior.

- [ ] **Step 1: Re-export Core engine and store paths**

Keep compatibility paths in the Tauri module:

```rust
pub use aevio_core::breaks::{engine, store};
```

Do not re-export or move `surface.rs` into the Core.

- [ ] **Step 2: Keep the Tauri driver and effect application local**

Retain `init`, activity-tick handling, app-state locks, effect execution, window creation, chimes, event publishing, and shutdown handling in the Tauri module. Route persistent break operations through `aevio_core::breaks::store`.

- [ ] **Step 3: Run Tauri break tests**

Run: `cargo test --manifest-path apps/desktop/tauri/Cargo.toml breaks::`
Expected: PASS; engine/store tests are in the Core suite, and the Tauri command/driver adapter compiles.

### Task 4: Verify Break Behavior and Consumers

**Files:**
- Test: `apps/api/Cargo.toml`
- Test: `apps/desktop/tauri/Cargo.toml`
- Test: `apps/cli/Cargo.toml`
- Modify: `README.md`

**Interfaces:**
- Consumes: Core engine/store plus Tauri runtime adapter.
- Produces: verified unchanged break behavior.

- [ ] **Step 1: Format**

Run: `bun run fix`
Expected: PASS.

- [ ] **Step 2: Run Core and Tauri tests**

Run: `cargo test --manifest-path apps/api/Cargo.toml && cargo test --manifest-path apps/desktop/tauri/Cargo.toml && cargo test --manifest-path apps/cli/Cargo.toml`
Expected: Core, Tauri, and CLI suites PASS.

- [ ] **Step 3: Document break ownership**

Update the README architecture section to show break state/storage in `apps/api` and reminder surfaces, app lifecycle driving, effects, and event publishing in Tauri.

- [ ] **Step 4: Run canonical verification**

Run: `bun run verify`
Expected: PASS.
