---
id: project-brain.knowledge-graph
title: Project Brain Knowledge Graph
category: architecture
domain: project-brain
status: implemented
source_paths: [graphify-out/graph.json, scripts/project-brain.mjs, docs/agents/retrieval.md]
related_docs: [docs/project-brain/ARCHITECTURE.md, docs/project-brain/IMPACT_ANALYSIS.md, docs/project-brain/OPERATIONS.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: []
---

# Knowledge graph adapters

| Adapter | Facts | Freshness | Authority |
|---|---|---|---|
| GitNexus | symbols, callers/callees, execution flows, VCS impact | indexed commit/branch | structural code graph |
| ProjectAtlas | persisted text, symbols, static relations, local MCP | local SQLite scan generation | deterministic repository index |
| Graphify | module/community/architecture relationships | graph output/update timestamp | architecture navigation |
| aevio-knowledge | docs, ADRs, frontmatter, bounded context | content hashes/index run | documentation retrieval |

The facade labels provenance and confidence for each result. Missing or stale adapters produce `unresolved` warnings. Graphify inferred edges are architecture clues, not proof of code dependencies. Graphiti was evaluated as an optional temporal graph service but is not installed or required; Git history plus ADRs provide the current local decision-history fallback without adding a competing store.
