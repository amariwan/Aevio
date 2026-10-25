---
id: project-brain.agent-integration
title: Project Brain Agent Integration
category: agents
domain: project-brain
status: implemented
source_paths: [scripts/project-brain.mjs, .codex/agents/aevio-orchestrator.toml, .agents/skills/aevio-engineering-system/SKILL.md]
related_docs: [docs/agents/AGENT_CATALOG.md, docs/agents/ROUTING.md, docs/project-brain/ARCHITECTURE.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: [TOOLS, toolDefinitions]
---

# Agent integration

Shared capabilities are available through the facade and MCP tool names:

`project_search`, `symbol_lookup`, `dependency_trace`, `impact_analysis`, `relevant_context`, `architecture_rules`, `decision_history`, `affected_tests`, and `brain_status`.

Codex/Agent Skills agents call the CLI directly. Other MCP clients can run `node scripts/project-brain.mjs mcp` as a local stdio server. The server is read-only and inherits the parent process permissions. The orchestrator remains responsible for routing and delegation; specialists receive only the bounded result they need.
