---
id: arch.cli
title: CLI (aevio)
category: architecture
domain: platform
status: implemented
source_paths: [apps/cli/src/]
related_docs: [docs/architecture/rust-core.md, docs/reference/tauri-commands.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# CLI (aevio)

Binary `aevio` (clap 4, clap_complete). Commands documented in `apps/cli/COMMANDS.md`.

## Modes

- Talks to a running app over RPC (`apps/desktop/tauri/src/rpc.rs`) when available.
- Otherwise opens data directly through `aevio-core`.

## MCP

`apps/cli/src/mcp.rs` exposes an MCP server surface.

## Commands

See `apps/cli/COMMANDS.md` for the full list and the Tauri command behind each.
