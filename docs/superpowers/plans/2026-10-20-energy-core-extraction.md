# Energy Persistence Core Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move energy sample models, SQLite persistence/querying, and summary calculations into `aevio-core`, leaving measurement and sampling in the Tauri runtime.

**Architecture:** Tauri continues to sample process and battery data using platform APIs and emits the existing event. It submits typed samples to the Core, which owns the database operations and summary calculations. The Core has no Tauri or native platform dependency.

**Tech Stack:** Rust 2021, Cargo, rusqlite, serde, chrono, Tauri 2.

**Spec:** `docs/superpowers/specs/2026-10-20-backend-domain-structure-design.md`

## Global Constraints

- `apps/api` remains a portable library; do not add `crates/aevio-core/src/main.rs`.
- Preserve existing behavior, serialized shapes, database schema, and externally used Rust paths through compatibility re-exports.
- Do not move Tauri startup, webview/window management, tray behavior, native capture, native energy sampling, OS APIs, or desktop event publishing into the core.
- Do not change SQLite schema, IPC/RPC behavior, frontend structure, or user experience.
- The Core owns energy sample persistence and queries; Tauri must not execute energy SQL directly.
- Do not bump the patch version; this is an internal refactor with no user-facing behavior change.
- The current repository has no existing `HEAD` and its project files are untracked; do not create a commit or stage unrelated files.

## File Ownership Map

- `crates/aevio-core/src/energy/models.rs`: serialized energy sample/summary and measurement input types.
- `crates/aevio-core/src/energy/store.rs`: all SQLite reads/writes, history queries, retention, and reset operations.
- `crates/aevio-core/src/energy/summary.rs`: pure summary/aggregation and impact threshold calculations.
- `apps/desktop/tauri/src/energy.rs`: process/battery sampling, native reads, time source, and event publishing; no energy SQL.
- `apps/desktop/tauri/src/commands.rs`: unchanged public command boundary forwarding through the Tauri energy adapter.

---

### Task 1: Establish Energy Test Baseline

**Files:**
- Test: `apps/desktop/tauri/src/energy.rs`
- Test: `apps/desktop/tauri/Cargo.toml`

**Interfaces:**
- Consumes: current energy calculations, SQL, and platform sampler.
- Produces: passing baseline before extracting portable code.

- [ ] **Step 1: Run the energy tests**

Run: `cargo test --manifest-path apps/desktop/tauri/Cargo.toml energy::`
Expected: PASS before moving energy behavior.

### Task 2: Add Core Energy Types, Store, and Summary API

**Files:**
- Create: `crates/aevio-core/src/energy/mod.rs`
- Create: `crates/aevio-core/src/energy/models.rs`
- Create: `crates/aevio-core/src/energy/store.rs`
- Create: `crates/aevio-core/src/energy/summary.rs`
- Modify: `crates/aevio-core/src/lib.rs`
- Test: `crates/aevio-core/src/energy/`

**Interfaces:**
- Consumes: current `EnergySample`, `EnergySummary`, `BatteryInfo`, `impact_level_for_power`, `get_summary`, `query_history`, `purge_older_than`, and `reset_history`.
- Produces: `aevio_core::energy` types and database operations with unchanged input/output shapes.

- [ ] **Step 1: Declare the module and move the existing energy tests**

Declare `pub mod energy;` in `crates/aevio-core/src/lib.rs`, create `energy/mod.rs`, `models.rs`, `store.rs`, and `summary.rs`, and move the existing tests from `apps/desktop/tauri/src/energy.rs` into `crates/aevio-core/src/energy/summary.rs`. Retain exact boundary assertions for `THRESHOLD_LOW_WATTS` and `THRESHOLD_MEDIUM_WATTS`, empty-history output, and rounded aggregate values. Use `crate::migrations::run_migrations(&mut conn)` and `crate::timers::now_epoch_ms()` in the moved test. Update the summary test to call `store::get_summary(&conn, 7, now, BatteryInfo::default(), None)` and replace its sample-seeding SQL with two `store::insert_sample` calls carrying the same timestamps and measurement values as the existing fixture.

- [ ] **Step 2: Run the new tests and confirm they fail before implementation**

Run: `cargo test --manifest-path apps/api/Cargo.toml energy::`
Expected: compile failure for the not-yet-implemented types and functions referenced by the moved tests.

- [ ] **Step 3: Implement portable energy models and calculations**

Move `EnergySample`, `EnergySummary`, `BatteryInfo`, energy thresholds, conversion constants, and summary calculation into Core modules. Add `EnergySampleInput` with these exact fields and types:

```rust
pub struct EnergySampleInput {
    pub sampled_at: u64,
    pub duration_ms: u32,
    pub cpu_time_ms: u32,
    pub energy_nj: u64,
    pub power_watts: f64,
    pub impact_level: String,
    pub ai_active: bool,
    pub on_battery: bool,
    pub battery_level: Option<f64>,
}
```

The pure summary function in `energy/summary.rs` is:

```rust
pub fn summarize(
    samples: &[EnergySample],
    days: u32,
    battery: BatteryInfo,
    latest_power_override: Option<f64>,
) -> EnergySummary;
```

The summary calculation accepts already measured battery state and current-power override; it does not call native APIs or read the clock in the Core. `store::get_summary` loads the bounded time window, calls this function, and returns `Result<EnergySummary, String>`.

- [ ] **Step 4: Implement Core persistence and query operations**

Move energy sample insertion, history query, retention purge, and reset SQL into `energy/store.rs`. Export these exact signatures and retain SQL text, row mapping, default limits, ordering, and error formatting:

```rust
pub fn insert_sample(conn: &Connection, sample: &EnergySampleInput) -> Result<(), String>;
pub fn get_summary(
    conn: &Connection,
    days: u32,
    now_ms: u64,
    battery: BatteryInfo,
    latest_power_override: Option<f64>,
) -> Result<EnergySummary, String>;
pub fn query_history(
    conn: &Connection,
    since_ms: Option<u64>,
    limit: Option<u32>,
) -> Result<Vec<EnergySample>, String>;
pub fn purge_older_than(conn: &Connection, days: u32, now_ms: u64) -> Result<usize, String>;
pub fn reset_history(conn: &Connection) -> Result<(), String>;
```

- [ ] **Step 5: Run Core energy tests**

Run: `cargo test --manifest-path apps/api/Cargo.toml energy::`
Expected: PASS.

### Task 3: Convert Tauri Energy Sampling to the Core API

**Files:**
- Modify: `apps/desktop/tauri/src/energy.rs`

**Interfaces:**
- Consumes: `aevio_core::energy::{EnergySample, EnergySampleInput, EnergySummary, BatteryInfo, DEFAULT_HISTORY_DAYS, impact_level_for_power, get_summary, insert_sample, query_history, purge_older_than, reset_history}`.
- Produces: unchanged frontend event payloads and command results; no energy SQL in the Tauri crate.

- [ ] **Step 1: Re-export the Core energy data types used by Tauri**

Replace local `EnergySample`, `EnergySummary`, and `BatteryInfo` definitions with imports or compatibility re-exports from `aevio_core::energy`. Import/re-export `DEFAULT_HISTORY_DAYS` and `impact_level_for_power` for existing command/sampler call sites. Keep native `ProcessRusage` in Tauri. Retain the current Tauri `get_summary(conn, days, battery, override)` signature as an adapter that calls Core with `now_epoch_ms()` and `battery.unwrap_or_else(read_battery_info)`; keep `query_history`, `purge_older_than`, and `reset_history` available through Core re-exports.

- [ ] **Step 2: Keep hardware sampling in Tauri**

Retain `AppHandle`, AI runtime activity sampling, process `rusage`, battery reads, cadence, delta calculation, energy estimation, and event publishing in `apps/desktop/tauri/src/energy.rs`.

- [ ] **Step 3: Replace direct energy SQL with Core calls**

Replace the sampler's SQL insert with the Core API:

```rust
if let Err(error) = insert_sample(
    &conn,
    &EnergySampleInput {
        sampled_at: now_ms,
        duration_ms,
        cpu_time_ms,
        energy_nj: delta_energy_nj,
        power_watts,
        impact_level: impact.to_string(),
        ai_active,
        on_battery: battery.on_battery,
        battery_level: battery.battery_percent,
    },
) {
    eprintln!("failed to persist energy sample: {error}");
    continue;
}
```

Keep the existing summary wrapper and make it call the Core API using `now_epoch_ms()` and `battery.unwrap_or_else(read_battery_info)`. Replace history, retention, and reset SQL with Core calls. Keep command signatures unchanged; their existing calls go through the Tauri adapter. Remove every direct energy SQL statement from Tauri code.

- [ ] **Step 4: Run Tauri energy tests**

Run: `cargo test --manifest-path apps/desktop/tauri/Cargo.toml energy::`
Expected: PASS; the energy behavior tests now run in Core, and this command compiles the Tauri energy adapter.

### Task 4: Verify Energy Ownership and Behavior

**Files:**
- Test: `apps/api/Cargo.toml`
- Test: `apps/desktop/tauri/Cargo.toml`
- Test: `apps/cli/Cargo.toml`
- Modify: `README.md`

**Interfaces:**
- Consumes: Core energy persistence/calculation and Tauri sampler/event adapter.
- Produces: verified unchanged energy history and event behavior.

- [ ] **Step 1: Confirm Tauri no longer owns energy SQL**

Run: `rg -n 'energy_samples' apps/desktop/tauri/src/energy.rs apps/desktop/tauri/src/commands.rs`
Expected: no SQL statements touching `energy_samples`.

- [ ] **Step 2: Format**

Run: `bun run fix`
Expected: PASS.

- [ ] **Step 3: Run Core and Tauri tests**

Run: `cargo test --manifest-path apps/api/Cargo.toml && cargo test --manifest-path apps/desktop/tauri/Cargo.toml && cargo test --manifest-path apps/cli/Cargo.toml`
Expected: Core, Tauri, and CLI suites PASS.

- [ ] **Step 4: Document energy ownership**

Update the README architecture section to show energy persistence and summary calculations in `apps/api`, with process/battery sampling and event publishing in Tauri.

- [ ] **Step 5: Run canonical verification**

Run: `bun run verify`
Expected: PASS.
