---
id: agents.drift
title: Documentation Drift Detection
category: agents
domain: agents
status: implemented
source_paths: [scripts/aevio-knowledge.mjs]
related_docs: [docs/agents/source-priority.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Documentation Drift Detection

Docs-as-Code: changes to public APIs, IPC contracts, DB schema, module boundaries, business rules, event contracts, config, platform support, retrieval behavior trigger a doc impact check.

`aevio-knowledge verify` checks: frontmatter schema, duplicate IDs, broken internal links, missing source references, invalid cross-refs. Drift = source changed but doc's verification_commit / source_paths stale. Report unresolved cases; do not overwrite ADRs.
