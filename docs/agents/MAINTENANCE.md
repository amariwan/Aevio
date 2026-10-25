---
id: agents.system-maintenance
title: Aevio Agent System Maintenance
category: agents
domain: agents
status: implemented
source_paths: [.codex/agents/, .agents/skills/aevio-engineering-system/SKILL.md, docs/agents/EVALUATION.md, docs/schemas/frontmatter.schema.json]
related_docs: [docs/agents/ARCHITECTURE.md, docs/agents/AGENT_CATALOG.md, docs/agents/CONTEXT_ENGINEERING.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: []
---

# Maintenance

Add an agent only when a recurring repository task has a stable input, procedure, output, and measurable benefit. Prefer extending an existing specialist. Keep the native adapter in `.codex/agents/`, shared behavior in the Agent Skills-compatible skill, and catalogs/routing in `docs/agents/`.

Change procedure:

1. Run the retrieval/architecture impact check and define ownership/exclusions.
2. Add or update the native agent plus catalog/routing/evaluation docs.
3. Validate TOML, frontmatter, links/source paths, `aevio-knowledge eval`, and the benchmark.
4. Run `graphify update .` only when source/code relationships changed; record `git diff --check` and relevant project verification.

Retire an agent when its responsibility is covered by another agent, its activation is ambiguous, or evaluation shows no quality/speed benefit. Remove its native file, routing entry, catalog row, and fixtures together; preserve historical decisions in git rather than dead prompt text.

External practices were researched from official OpenAI Codex AGENTS/subagents docs, the Agent Skills specification, GitHub Copilot instructions, Anthropic's public skills repository, MCP specification, and Superpowers. Adopt only concepts compatible with Aevio; review license and maintenance status before vendoring code.
