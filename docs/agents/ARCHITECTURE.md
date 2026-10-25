---
id: agents.system-architecture
title: Aevio Multi-Agent System Architecture
category: agents
domain: agents
status: implemented
source_paths: [AGENTS.md, docs/architecture/overview.md, docs/architecture/monorepo.md, docs/agents/retrieval.md]
related_docs: [docs/agents/AGENT_CATALOG.md, docs/agents/ROUTING.md, docs/agents/CONTEXT_ENGINEERING.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: []
---

# Aevio Multi-Agent System Architecture

The system has one primary orchestrator and five bounded specialists. Single-agent execution is the default. Delegation is justified only by specialization, context isolation, or independent read-only parallelism.

```text
request
  -> aevio-orchestrator
     -> scoped retrieval (docs FTS + Graphify + GitNexus)
     -> one specialist (default)
        -> optional independent read-only specialists
     -> serialized implementation
     -> quality/security gate
     -> docs/index refresh and evidence report
```

The repository remains the source of truth. Rust owns durable state in `apps/api` and `apps/desktop/tauri`; React is the view in `apps/web`; the CLI and server applications consume the shared core; iOS/watchOS remain separate clients. These boundaries are documented in `docs/architecture/overview.md`, `docs/architecture/monorepo.md`, and `AGENTS.md`.

Native adapters:

- Codex: `.codex/agents/*.toml` and root/nested `AGENTS.md`.
- Agent Skills-compatible clients: `.agents/skills/aevio-engineering-system/SKILL.md`.
- Claude/Copilot: use their native custom-agent/instruction formats; do not copy Codex TOML as if it were portable.

Security boundary: project agents inherit the parent runtime's permissions; their TOML cannot create a stronger sandbox. Logical least privilege is therefore enforced by routing, explicit scope, no-secret rules, and review gates. The project does not change global Codex settings.
