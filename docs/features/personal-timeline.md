---
id: feature.personal-timeline
title: Personal Timeline and Privacy Boundary
category: features
domain: tracking
status: implemented
source_paths: [crates/aevio-services/src/timeline.rs, apps/desktop/tauri/src/commands.rs, apps/api/src/main.rs, apps/web/src/features/timeline/]
related_docs: [docs/reference/data-model.md, docs/reference/ipc.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Personal Timeline

The personal timeline is a local projection of captured activity. It lets a
user review their own day without exposing raw activity to a workspace.

## Privacy contract

- Captured segments are private by default.
- A timeline publication is a separate row from `segments`.
- Sharing is explicit and can be revoked.
- A publication may carry a workspace id and a safe title override.
- The workspace must never infer visibility from the presence of a segment.
- Raw window titles, URLs and application activity remain personal data.

## Local data flow

```text
OS capture -> segments (private) -> timeline projection -> explicit publication
```

The desktop commands expose the local timeline, publication and revocation
operations, device inventory, sync-state contract, and authenticated remote
push/pull. The connector consumes only the publication/outbox contract; it
never uploads the raw `segments` table as a database replica.
