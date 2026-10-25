# Plugin Navigation Gating Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Only installed and active Aevio plugins contribute feature pages to the main navigation.

**Architecture:** Keep the existing host-mediated plugin catalog and active-contribution API. Remove plugin-backed pages from static navigation and expose them only through active contribution snapshots; keep the Plugin Store in Settings so uninstalled plugins remain discoverable.

**Tech Stack:** React/TypeScript, Tauri 2, Rust plugin service, existing Aevio plugin contracts.

**Spec:** User request in the current conversation: installable plugin features such as Notes, Music, and Project Planning.

## Global Constraints

- Preserve all unrelated dirty work in the shared checkout.
- Use `useTauriEvent` for plugin-change refreshes.
- Do not add dependencies, schema resets, or native app-drive runs.
- New UI must use existing shadcn/ui and semantic theme tokens.

## Review Focus

- An uninstalled plugin must not contribute a sidebar entry.
- An installed-but-disabled plugin must not contribute a sidebar entry.
- Enabling a plugin must refresh navigation after `plugins-changed`.
- The Plugin Store must remain reachable when no feature plugins are active.
- Existing core routes and unrelated navigation must remain unchanged.

### Task 1: Gate static plugin-backed navigation

**Files:**
- Modify: `apps/web/src/components/layout/Sidebar.tsx`
- Modify: `crates/aevio-services/src/plugins_builtin.rs`

- [x] Remove static sidebar entries whose pages are supplied by first-party plugin contributions.
- [x] Add the Project Planning task page to the declared contribution set if it is still a core-only route.
- [x] Preserve the settings route that contains the Plugin Store.
- [x] Run focused TypeScript/Rust checks.

### Task 2: Verify plugin lifecycle behavior

**Files:**
- Test: existing plugin service tests and repository verification scripts.

- [x] Verify catalog entries remain discoverable while `installed` is absent.
- [x] Verify active contributions are empty until installation, permissions, and activation complete.
- [x] Run `bun run fix`, `bun run verify`, and focused plugin Rust tests.
- [x] Run `graphify update .` and inspect the final diff for unrelated changes.
