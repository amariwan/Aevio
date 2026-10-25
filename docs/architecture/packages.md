---
id: arch.packages
title: Shared TypeScript Packages
category: architecture
domain: platform
status: implemented
source_paths: [packages/]
related_docs: [docs/architecture/monorepo.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Shared TypeScript Packages

- `packages/api-client` — typed client for Tauri commands / RPC.
- `packages/config` — shared TS config (tsconfig bases).
- `packages/types` — shared domain types.
- `packages/ui` — shared UI primitives (shadcn-based).
- `packages/web` — shared web utilities.

Keep domain logic in Rust; TS packages are view/transport helpers only.
