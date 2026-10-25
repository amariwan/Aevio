---
id: eng.rules
title: Engineering Rules
category: engineering
domain: platform
status: implemented
source_paths: [AGENTS.md]
related_docs: [docs/engineering/file-size.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Engineering Rules

- SOLID, KISS, DRY, YAGNI.
- Separation of Concerns: UI / application / domain / infrastructure strictly separated.
- High cohesion / low coupling.
- Composition over inheritance.
- Law of Demeter.
- Tell, don't ask.
- Explicit over implicit: no hidden side effects, no implicit DB operations.
- No business logic in presentation adapters.
- No SQL in UI or Domain.
- No circular feature dependencies.
- Explicit error handling (no swallowed errors).
- Dependency direction: domain ← application ← infrastructure ← UI.
- Max 500 lines per handwritten production source file.
- New UI uses shadcn/ui only.
- Money in integer USD cents in Rust; never compute totals in TS.
- Rust owns durable state; React never writes DB directly.
- User-visible reminders use native OS notifications on every supported desktop platform.
- Cross-platform by design via cfg_attr platform modules.
