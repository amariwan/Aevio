---
id: agents.context-routing
title: Context Routing
category: agents
domain: agents
status: implemented
source_paths: [scripts/aevio-knowledge.mjs]
related_docs: [docs/agents/retrieval.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Context Routing

Task-aware routing. Load only relevant context; expand on demand.

| Task | Priority sources |
|---|---|
| Understand feature | feature docs, architecture, data model |
| Fix bug | relevant feature doc, symbols via GitNexus, tests |
| Implement feature | architecture rules, related feature docs, IPC if touching |
| Refactor | GitNexus impact, architecture rules, tests |
| Review code | rules, file-size, relevant feature docs |
| Regression | git history, tests, events |
| Schema change | data model, migrations, IPC |
| IPC change | tauri commands, ipc, events |
| Native OS change | platforms, architecture (platform modules) |
| Architecture eval | overview, rules, decisions |

Rust domain refactoring prioritizes domain docs, dependent symbols, public interfaces, constraints, tests. UI styling avoids DB/AI details.
