---
id: ref.events
title: Application Events
category: reference
domain: platform
status: implemented
source_paths: [apps/desktop/tauri/src/lib.rs, apps/web/src/hooks/useTauriEvent.ts]
related_docs: [docs/architecture/tauri.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Application Events

Rust pushes snapshots on mutation (activity, usage, coach, etc.). React subscribes via `useTauriEvent`. Event names are the contract; do not rename without updating both sides.
