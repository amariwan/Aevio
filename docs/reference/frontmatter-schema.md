---
id: ref.frontmatter-schema
title: Frontmatter Schema
category: reference
domain: agents
status: implemented
source_paths: [docs/schemas/frontmatter.schema.json]
related_docs: [docs/agents/retrieval.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Frontmatter Schema

All structured docs use YAML frontmatter validated against `docs/schemas/frontmatter.schema.json`. Required: id, title, category, domain, status, source_paths, related_docs, platforms, verification_commit. Optional: owner, symbols.
