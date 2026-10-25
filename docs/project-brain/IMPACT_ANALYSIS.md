---
id: project-brain.impact-analysis
title: Project Brain Impact Analysis
category: engineering
domain: project-brain
status: implemented
source_paths: [scripts/project-brain.mjs, .agents/skills/gitnexus-impact-analysis/SKILL.md, AGENTS.md]
related_docs: [docs/project-brain/KNOWLEDGE_GRAPH.md, docs/project-brain/AGENT_INTEGRATION.md, docs/agents/ROUTING.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: [impact, affectedTests]
---

# Impact analysis

`node scripts/project-brain.mjs impact <symbol>` calls GitNexus upstream impact with tests. Without a symbol it analyzes the working tree through GitNexus `detect-changes`. Reports include:

- direct/transitive graph output and confidence;
- changed owners and cross-boundary review requirement;
- database/migration and security-sensitive path flags;
- affected test paths and freshness state;
- unresolved conditions, especially stale indexes and `UNKNOWN` risk.

No-edge results are never treated as safe when the index is stale or unresolved. Refresh with `node scripts/project-brain.mjs update` before relying on a report.
