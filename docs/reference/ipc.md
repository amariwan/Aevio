---
id: ref.ipc
title: IPC Contracts
category: reference
domain: platform
status: implemented
source_paths: [crates/aevio-core/src/ipc/, apps/desktop/tauri/src/rpc.rs]
related_docs: [docs/reference/tauri-commands.md, docs/reference/events.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# IPC Contracts

Tauri commands (frontend↔Rust) and RPC (CLI↔app). Shared serde types in `aevio-core`. See `docs/TAURI_COMMANDS.md` for exhaustive command list.
