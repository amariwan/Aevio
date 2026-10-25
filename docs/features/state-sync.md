---
id: feat.state-sync
title: State Synchronization
category: features
domain: state
status: implemented
source_paths: [apps/desktop/tauri/src/lib.rs, apps/web/src/hooks/useTauriEvent.ts]
related_docs: [docs/architecture/tauri.md, docs/architecture/react.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# State Synchronization

Rust owns durable state; React holds route/transient UI/last snapshot. Rust pushes events on change; React subscribes via `useTauriEvent`.
