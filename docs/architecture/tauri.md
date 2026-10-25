---
id: arch.tauri
title: Tauri Integration
category: architecture
domain: platform
status: implemented
source_paths: [apps/desktop/tauri/src/lib.rs, apps/desktop/tauri/src/commands.rs, apps/desktop/tauri/src/rpc.rs]
related_docs: [docs/reference/tauri-commands.md, docs/reference/events.md, docs/reference/ipc.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Tauri Integration

Tauri 2 app. Rust core (commands, state, events) + webview (React). External binaries: Swift sidecar.

## Commands

All Tauri commands are `pub fn` in `apps/desktop/tauri/src/commands.rs`, grouped by domain. See `docs/TAURI_COMMANDS.md` (canonical exhaustive list) and `docs/reference/tauri-commands.md`.

## Events

Rust pushes snapshots on mutation. React subscribes with `useTauriEvent` (handles stale-listener/unlisten races). Never raw `listen`.

## State

Rust holds all durable state. React holds route + transient UI + last pushed snapshot.

## RPC

`apps/desktop/tauri/src/rpc.rs` exposes IPC for the CLI to talk to a running app (Unix sockets / Windows named pipes via interprocess).

## Native adapters

Capture, notifications, tray, updater, login item, permissions, widget, pulse — each follows `cfg_attr` platform-module pattern.
