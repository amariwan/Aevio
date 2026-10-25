---
id: arch.react
title: React / Next.js View Layer
category: architecture
domain: platform
status: implemented
source_paths: [apps/web/src/]
related_docs: [docs/architecture/overview.md, docs/ROUTE_MAP.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# React / Next.js View Layer

Next.js 16 static export (Turbopack) consumed by the Tauri webview. React 19.

## Rules

- No business logic in JSX. No DB/OS/IPC details beyond typed hooks.
- Presentation components ≤ 150 lines.
- Subscribe to Tauri events via `useTauriEvent`.
- Money: never compute totals in TS; display Rust-provided cents.
- Styling: Tailwind 4 with semantic tokens in `src/styles/app.css`. New UI uses shadcn/ui only.

## State

TanStack Query for server (Rust) state; Zustand for transient UI. Route state via Next.js router / nuqs.
