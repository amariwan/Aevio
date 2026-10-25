# Invoice Core Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move invoice domain rules, persistence, finalization, PDF rendering, and archived PDF retrieval into `aevio-core`, leaving only desktop export UI and filesystem writes in Tauri.

**Architecture:** The core owns invoice data models, SQL transactions, integer-cent arithmetic, profile/logo data, and PDF bytes. Tauri commands remain as adapters that call the core and emit existing events; the save-file dialog and writing to the selected path stay in the Tauri crate.

**Tech Stack:** Rust 2021, Cargo, rusqlite, serde, chrono, printpdf, image, Tauri 2.

**Spec:** `docs/superpowers/specs/2026-10-20-backend-domain-structure-design.md`

## Global Constraints

- `apps/api` remains a portable library; do not add `crates/aevio-core/src/main.rs`.
- Preserve existing behavior, serialized shapes, database schema, and externally used Rust paths through compatibility re-exports.
- Do not move Tauri startup, webview/window management, tray behavior, native capture, native energy sampling, OS APIs, or desktop event publishing into the core.
- Do not change SQLite schema, IPC/RPC behavior, frontend structure, or user experience.
- Keep finalization and archived PDF bytes in the Core; keep file dialogs and writing selected paths in Tauri.
- Do not bump the patch version; this is an internal refactor with no user-facing behavior change.
- The current repository has no existing `HEAD` and its project files are untracked; do not create a commit or stage unrelated files.

## File Ownership Map

- `crates/aevio-core/src/invoices/`: invoice types, SQL transactions, profile/logo data, integer-cent arithmetic, PDF rendering, and invoice tests.
- `apps/desktop/tauri/src/invoices/mod.rs`: compatibility re-exports for existing command imports.
- `apps/desktop/tauri/src/commands.rs`: Tauri command signatures, event publication, save dialog, and writing exported PDF bytes to the selected path.
- `apps/api/Cargo.toml`: portable invoice rendering/image dependencies.

---

### Task 1: Establish Invoice Test Baseline

**Files:**
- Test: `apps/desktop/tauri/src/invoices/tests.rs`
- Test: `apps/desktop/tauri/Cargo.toml`

**Interfaces:**
- Consumes: current Tauri invoice module.
- Produces: passing behavioral baseline before moving it.

- [ ] **Step 1: Run the existing invoice tests**

Run: `cargo test --manifest-path apps/desktop/tauri/Cargo.toml invoices::tests`
Expected: PASS; record any existing failure before moving code.

### Task 2: Move Invoice Logic and Tests to the Core

**Files:**
- Create: `crates/aevio-core/src/invoices/mod.rs`
- Move: `apps/desktop/tauri/src/invoices/money.rs` to `crates/aevio-core/src/invoices/money.rs`
- Move: `apps/desktop/tauri/src/invoices/pdf.rs` to `crates/aevio-core/src/invoices/pdf.rs`
- Move: `apps/desktop/tauri/src/invoices/profile.rs` to `crates/aevio-core/src/invoices/profile.rs`
- Move: `apps/desktop/tauri/src/invoices/tests.rs` to `crates/aevio-core/src/invoices/tests.rs`
- Move: `apps/desktop/tauri/src/invoices/testdata/sample-logo.png` to `crates/aevio-core/src/invoices/testdata/sample-logo.png`
- Modify: `crates/aevio-core/src/lib.rs`
- Modify: `apps/api/Cargo.toml`

**Interfaces:**
- Consumes: current `invoices::{list, get, billable_entries, quote, render_draft, stored_pdf, save_draft, finalize, set_paid, void, delete_draft, delete_void, file_name}` APIs.
- Produces: the same public API at `aevio_core::invoices::*`, with existing tests run in the Core crate.

- [ ] **Step 1: Declare the Core invoice module**

Add this to `crates/aevio-core/src/lib.rs`:

```rust
pub mod invoices;
```

- [ ] **Step 2: Move the implementation and invoice tests**

Move the complete invoice implementation, private helper modules, tests, and fixture to the `crates/aevio-core/src/invoices/` paths above. Keep the function names, arguments, return types, SQL transaction order, and assertions unchanged.

- [ ] **Step 3: Add invoice-only dependencies to `apps/api/Cargo.toml`**

Add the exact versions/features currently used by the Tauri manifest for `printpdf` and `image`. Do not change versions or features while moving them.

- [ ] **Step 4: Run Core invoice tests**

Run: `cargo test --manifest-path apps/api/Cargo.toml invoices::`
Expected: PASS with all former Tauri invoice tests now running against `aevio-core`.

### Task 3: Retain Tauri Command and File Export Adapters

**Files:**
- Modify: `apps/desktop/tauri/src/invoices/mod.rs`
- Modify: `apps/desktop/tauri/src/commands.rs`
- Modify: `apps/desktop/tauri/Cargo.toml`
- Test: `apps/desktop/tauri/src/lib.rs`

**Interfaces:**
- Consumes: `aevio_core::invoices::*`.
- Produces: unchanged Tauri command names and payloads, forwarding domain operations to the Core.

- [ ] **Step 1: Replace the Tauri invoice implementation with compatibility re-exports**

Keep the Tauri path used by `commands.rs` and make it forward to the Core:

```rust
pub use aevio_core::invoices::*;
```

Remove local invoice implementation and test declarations only after the Core test suite passes.

- [ ] **Step 2: Keep export UI in Tauri and forward PDF bytes**

Keep the `export_invoice_pdf` command, save dialog, cancellation behavior, and selected-path write in `commands.rs`. Fetch preview or archived bytes from `aevio_core::invoices`; do not duplicate SQL or PDF rendering in Tauri.

- [ ] **Step 3: Remove dependencies only if no Tauri code still uses them**

Search Tauri sources for `printpdf` and `image`. Remove either dependency from `apps/desktop/tauri/Cargo.toml` only if no remaining Tauri module imports it.

- [ ] **Step 4: Run Tauri invoice tests**

Run: `cargo test --manifest-path apps/desktop/tauri/Cargo.toml invoices::`
Expected: PASS; the Core owns invoice tests, and this command compiles the Tauri invoice adapters.

### Task 4: Verify the Invoice Extraction

**Files:**
- Test: `apps/api/Cargo.toml`
- Test: `apps/desktop/tauri/Cargo.toml`
- Test: `apps/cli/Cargo.toml`
- Modify: `README.md`

**Interfaces:**
- Consumes: Core invoice APIs and Tauri export adapters.
- Produces: verified invoice ownership and no behavior changes.

- [ ] **Step 1: Format**

Run: `bun run fix`
Expected: PASS.

- [ ] **Step 2: Run Core and Tauri tests**

Run: `cargo test --manifest-path apps/api/Cargo.toml && cargo test --manifest-path apps/desktop/tauri/Cargo.toml && cargo test --manifest-path apps/cli/Cargo.toml`
Expected: Core, Tauri, and CLI suites PASS.

- [ ] **Step 3: Document invoice ownership**

Update the README architecture section to show invoice domain persistence and PDF rendering in `apps/api`, with only the desktop save dialog and selected-path write in Tauri.

- [ ] **Step 4: Run canonical verification**

Run: `bun run verify`
Expected: PASS.
