---
id: agents.retrieval
title: Agent Retrieval Architecture
category: agents
domain: agents
status: implemented
source_paths: [scripts/aevio-knowledge.mjs]
related_docs: [docs/agents/context-routing.md, docs/agents/source-priority.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Agent Retrieval Architecture

Local-first. Layers:

A. **Exact doc retrieval** — SQLite FTS5 over Markdown, BM25 ranking. Terms, paths, identifiers, symbols, titles, doc IDs, keywords.
B. **Graphify** — reuse existing graphify-out/ for architecture/import/module relationships. Do not rebuild unless needed.
C. **GitNexus** — symbol context, callers/deps, execution flows, impact, risk. Freshness-check before trusting.
D. **Optional semantic** — local embedding adapter (not mandatory; no heavyweight vector DB).
E. **Context builder** — aggregates, dedupes, ranks, attributes sources, validates freshness, estimates size, token budgeting, graceful fallback.

Indexes are derived artifacts; docs are canonical. Incremental indexing via content hashes.

CLI: `node scripts/aevio-knowledge.mjs {search,context,feature,index,verify,eval}`.
