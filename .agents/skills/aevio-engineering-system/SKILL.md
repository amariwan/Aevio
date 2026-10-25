---
name: aevio-engineering-system
description: Routes repository-aware Aevio engineering work through bounded specialists, local retrieval, Graphify, GitNexus, and canonical verification. Use for architecture, implementation, bug, review, security, testing, documentation, and agent-system tasks in this repository.
license: Internal project guidance; no third-party code bundled
compatibility: Requires the Aevio repository, git, Node.js, and the installed graphify/GitNexus tooling when available.
metadata:
  owner: Aevio
  version: "1.0"
---

# Aevio engineering system

Use this skill as the portable workflow shared by Codex, Claude Code, Copilot, and other Agent Skills-compatible clients. The native adapter is `.codex/agents/`; the repository contract remains `AGENTS.md`.

## Workflow

1. Classify the request and choose one specialist from `docs/agents/ROUTING.md`.
2. Retrieve only the task slice: `node scripts/aevio-knowledge.mjs context "<task>" --budget=<n>`, then Graphify/GitNexus for relationships and impact.
3. Before editing a symbol, run GitNexus impact and record risk. Treat `UNKNOWN` as unresolved.
4. Keep ownership non-overlapping. Parallelize only independent read-only investigations; serialize writes.
5. Verify with the canonical project commands and run `graphify update .` after code changes. Report measured evidence.

The shared facade `node scripts/project-brain.mjs` exposes the same capabilities to every agent: `search`, `symbol`, `dependencies`, `impact`, `context`, `rules`, `decisions`, `tests`, and read-only MCP via `mcp`. It delegates structural facts to ProjectAtlas/GitNexus/Graphify and marks unavailable or stale adapters instead of inventing relationships.

## Context budget

Default specialist input is 4,000 estimated tokens, with 1,000 tokens for the task contract and 1,000 tokens for the final evidence summary. Expand only when the specialist names a concrete missing source.

## Handoff contract

Every delegation includes scope, excluded paths, expected output, acceptance criteria, risk, and a stopping condition. The receiving specialist returns findings or changes, exact paths/symbols, verification, unresolved assumptions, and a next handoff only when ownership requires it.

## Safety

Never print or persist credentials, tokens, cookies, or secret-bearing logs. Do not enable global multi-agent settings from project files. Destructive commands, external messages, schema resets, and native app-drive require explicit user scope.
