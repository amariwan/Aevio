---
id: agents.context-engineering
title: Aevio Agent Context Engineering
category: agents
domain: agents
status: implemented
source_paths: [scripts/aevio-knowledge.mjs, docs/agents/retrieval.md, docs/agents/context-routing.md, graphify-out/manifest.json]
related_docs: [docs/agents/ARCHITECTURE.md, docs/agents/EVALUATION.md, docs/agents/MAINTENANCE.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: [cmdContext, cmdEval]
---

# Context engineering

Use progressive disclosure:

1. `AGENTS.md` and the task contract.
2. `node scripts/aevio-knowledge.mjs context "<task>" --budget=4000`.
3. Graphify query/path/explain for module relationships.
4. GitNexus context/impact for symbols, callers, risk, and execution flows.
5. Exact source/tests only after the index identifies the slice.

For agent-compatible access use `node scripts/project-brain.mjs context "<task>"`, `impact`, `symbol`, `dependencies`, `tests`, or the read-only `mcp` command. This facade returns branch, HEAD, dirty state, adapter freshness, confidence, unresolved uncertainty, and source provenance with every result.

Default budget is 4,000 estimated retrieved tokens per specialist, plus 1,000 tokens for task constraints and 1,000 for the result. The `aevio-knowledge` builder currently estimates tokens as characters divided by four and caps each document excerpt; this is a deterministic budget guard, not a provider billing measurement.

Do not load whole `graphify-out/`, lockfiles, generated output, dependencies, or secret-bearing files. Treat stale indexes and `UNKNOWN` graph risk as unresolved. Source/tests outrank docs; docs outrank derived indexes. Return evidence paths and compact findings instead of raw command output.

The 70–80% reduction target is experimental. Measure it against the repository's current retrieval dataset and report recall, selected context size, latency, and failures. Never optimize token count by dropping required contracts or tests.
