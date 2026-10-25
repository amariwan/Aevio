---
id: project-brain.architecture
title: Project Brain Architecture
category: architecture
domain: project-brain
status: implemented
source_paths: [scripts/project-brain.mjs, AGENTS.md, docs/agents/ARCHITECTURE.md]
related_docs: [docs/project-brain/KNOWLEDGE_GRAPH.md, docs/project-brain/AGENT_INTEGRATION.md, docs/project-brain/OPERATIONS.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: [search, symbolLookup, dependencyTrace, impact, relevantContext]
---

# Project Brain architecture

Project Brain is a local-first facade, not a new index. It composes existing derived sources:

```text
agent -> project-brain.mjs -> {aevio-knowledge, ProjectAtlas, GitNexus, Graphify}
                              -> structured result + provenance + freshness
```

Structural facts come from ProjectAtlas and GitNexus. Documentation and ADR retrieval comes from `aevio-knowledge`. Graphify is used for architecture relationships. LLM inference is never promoted to a verified dependency.

The facade has no production dependency and no background service. Results are branch-/HEAD-aware and cached only under ignored `.project-brain/cache/`. The source of truth remains repository source, tests, ADRs, and Git history in that order.
