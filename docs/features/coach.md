---
id: feat.coach
title: Coach (Personal Intelligence)
category: features
domain: coach
status: implemented
source_paths: [crates/aevio-core/src/coach/, apps/desktop/tauri/src/coach.rs, apps/web/src/features/coach/, docs/features/ai-coach/]
related_docs: [docs/features/ai-coach/overview.md, docs/features/decision-engine.md, docs/features/energy-breaks.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Coach

See `docs/features/ai-coach/` for detailed docs (overview, architecture, workflows, data model, AI orchestration, privacy, testing). Desktop-only, local SQLite, optional on-device model, explicit user actions for every change.

High-level: editable goals/capacity/notes, task breakdown, Just Start, daily priority suggestions, progress & reviews, focus sessions, weekly aggregates, brain dump, routine templates. Rules work offline; on-device proposals optional and labeled.
