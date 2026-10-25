---
id: docs.index
title: Aevio Knowledge Index
category: agents
domain: platform
status: implemented
source_paths:
  - docs/
related_docs:
  - docs/README.md
  - AGENTS.md
platforms:
  - macos
  - windows
  - ios
  - watchos
verification_commit: working-tree
---

# Aevio Knowledge Index (Agent-Friendly)

Machine-friendly directory of all documentation. Use the knowledge CLI for search and scoped context retrieval.

## Retrieval entry points

- `node scripts/aevio-knowledge.mjs search "<query>"` — full-text search over docs.
- `node scripts/aevio-knowledge.mjs context "<task>"` — task-aware context bundle.
- `node scripts/aevio-knowledge.mjs feature <domain>` — feature documentation bundle.
- `node scripts/aevio-knowledge.mjs index` — rebuild the local FTS index.
- `node scripts/aevio-knowledge.mjs verify` — validate metadata, links, IDs.
- `node scripts/aevio-knowledge.mjs eval` — run retrieval evaluation.

## Document catalog

### Architecture
- [Overview](architecture/overview.md) — system, boundaries, data flow
- [Database Architecture](DATABASE_ARCHITECTURE.md) — persistence ownership and target boundaries
- [SeaORM Migration](SEAORM_MIGRATION.md) — migration decision, audit, and completion boundary
- [SQL Access Audit](SQL_AUDIT.md) — baseline, current counts, and justified exceptions
- [Monorepo](architecture/monorepo.md) — workspace layout
- [Rust Core](architecture/rust-core.md) — aevio-core domains
- [Tauri Integration](architecture/tauri.md) — commands, events, state
- [React / Next.js](architecture/react.md) — view layer
- [Shared Packages](architecture/packages.md) — TS packages
- [CLI](architecture/cli.md) — aevio binary
- [On-device AI](architecture/ai-sidecar.md) — Swift sidecar
- [iOS](architecture/ios.md), [watchOS](architecture/watchos.md)

### Features (domains)
- [Current Feature Inventory](features/current-inventory.md) — complete current product capability inventory and verification boundaries
- [Activity & Capture](features/activity-capture.md)
- [Time Tracking & Entries](features/time-tracking.md)
- [Coach](features/coach.md)
- [Decision Engine](features/decision-engine.md)
- [Energy & Breaks](features/energy-breaks.md)
- [Automation](features/automation.md)
- [Finance & Invoices](features/finance.md)
- [Work Management](features/work-management.md)
- [Integrations](features/integrations.md)
- [Notifications](features/notifications.md)
- [Widgets](features/widgets.md)
- [Settings](features/settings.md)
- [State Synchronization](features/state-sync.md)

### Platforms
- [macOS](platforms/macos.md), [Windows](platforms/windows.md), [Linux](platforms/linux.md), [iOS](platforms/ios.md), [watchOS](platforms/watchos.md)

### Engineering
- [Rules](engineering/rules.md), [File Size](engineering/file-size.md), [Testing](engineering/testing.md), [CI/CD](engineering/ci-cd.md), [Releases](engineering/releases.md)
- [Persistence Guidelines](PERSISTENCE_GUIDELINES.md) — repository and transaction rules

### Reference
- [IPC Contracts](reference/ipc.md), [Events](reference/events.md), [Data Model](reference/data-model.md), [Tauri Commands](reference/tauri-commands.md), [Frontmatter Schema](reference/frontmatter-schema.md)
- [Database Compatibility](DATABASE_COMPATIBILITY.md) — schema, locking, and exception policy

### Decisions
- [ADR Template](decisions/000-template.md), [Modular Monolith](decisions/001-modular-monolith.md)

### Agents
- [System Architecture](agents/ARCHITECTURE.md), [Agent Catalog](agents/AGENT_CATALOG.md), [Routing](agents/ROUTING.md), [Context Engineering](agents/CONTEXT_ENGINEERING.md), [Evaluation](agents/EVALUATION.md), [Maintenance](agents/MAINTENANCE.md)
- Existing retrieval references: [Retrieval Architecture](agents/retrieval.md), [Context Routing](agents/context-routing.md), [Source Priority](agents/source-priority.md), [Documentation Drift](agents/drift.md)

### Project Brain
- [Architecture](project-brain/ARCHITECTURE.md), [Knowledge Graph](project-brain/KNOWLEDGE_GRAPH.md), [Impact Analysis](project-brain/IMPACT_ANALYSIS.md), [Agent Integration](project-brain/AGENT_INTEGRATION.md), [Benchmarks](project-brain/BENCHMARKS.md), [Operations](project-brain/OPERATIONS.md)
