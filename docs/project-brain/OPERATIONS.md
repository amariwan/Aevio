---
id: project-brain.operations
title: Project Brain Operations
category: operations
domain: project-brain
status: implemented
source_paths: [scripts/project-brain.mjs, .gitignore, docs/project-brain/IMPACT_ANALYSIS.md]
related_docs: [docs/project-brain/ARCHITECTURE.md, docs/project-brain/KNOWLEDGE_GRAPH.md, docs/project-brain/BENCHMARKS.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: [update, freshness]
---

# Operations

Common commands:

```sh
node scripts/project-brain.mjs status
node scripts/project-brain.mjs update
node scripts/project-brain.mjs context "<task>"
node scripts/project-brain.mjs impact <symbol>
node scripts/project-brain.mjs mcp
```

`update` refreshes the existing docs index, ProjectAtlas scan, and Graphify output. GitNexus analysis remains explicit because it can be expensive and must be scoped to the repository state. `.projectatlas/` and `.project-brain/` are local derived state and ignored. Branch and dirty-state fingerprints prevent cache reuse across incompatible working trees.

If an adapter is unavailable, results remain structured with `unresolved`/availability details. Never silently substitute LLM-inferred dependencies. Inspect `status`, refresh the affected adapter, and report the limitation.
