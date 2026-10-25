---
id: agents.catalog
title: Aevio Agent Catalog
category: agents
domain: agents
status: implemented
source_paths: [.codex/agents/, .agents/skills/aevio-engineering-system/SKILL.md, AGENTS.md]
related_docs: [docs/agents/ARCHITECTURE.md, docs/agents/ROUTING.md, docs/agents/MAINTENANCE.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: []
---

# Agent catalog

| Agent | Owns | Does not own | Default model/permission policy |
|---|---|---|---|
| `aevio-orchestrator` | classification, retrieval, sequencing, handoffs | domain implementation by itself when a specialist is sufficient | inherit parent; logical read-only until scoped |
| `aevio-architecture` | boundaries, design, impact, decisions | isolated UI polish or routine formatting | inherit parent; read-only analysis |
| `aevio-backend-persistence` | Rust, SQLite, SeaORM, IPC, CLI, native capability boundary | React business logic and SQL outside infrastructure | inherit parent; assigned Rust/docs/tests only |
| `aevio-frontend-native` | React, Next.js, shadcn, semantic tokens, Tauri consumers | durable state, SQL, native implementation | inherit parent; assigned frontend/docs/tests only |
| `aevio-quality-security` | review, tests, security, privacy, performance | unrelated rewrites and speculative frameworks | inherit parent; read-only/test-first |
| `aevio-docs-retrieval` | docs, indexes, agent system, retrieval evaluation | application refactors | inherit parent; assigned docs/config only |
| `aevio-ai-sidecar` | Swift sidecar, ML categorization, protocol version, fallback | React UI, durable state, SQL | inherit parent; assigned AI/sidecar/docs/tests only |
| `aevio-invoices` | money (cents), PDF composition, invoice contracts | TS totals, HTML invoice imitation | inherit parent; assigned invoice/Rust/docs/tests only |
| `aevio-cli-rpc` | CLI commands, rpc, install-path, COMMANDS.md | duplicated domain logic | inherit parent; assigned CLI/docs/tests only |
| `aevio-release-ops` | version bumps, CI, packaging, release docs | speculative CI, test frameworks | inherit parent; assigned release/docs/config only |
| `aevio-data-migrations` | schema migrations, import/export, backup/restore | schema reset, data loss | inherit parent; assigned migration/docs/tests only |
| `aevio-platform-native` | per-OS native modules, tray, shortcuts, notifications | macOS-only default, silent emulation | inherit parent; assigned platform/docs/tests only |
| `aevio-accessibility-i18n` | a11y, i18n, responsive UI | raw colors, hardcoded strings | inherit parent; assigned frontend/docs/tests only |
| `aevio-performance` | profiling, benchmarks, N+1, bundle size | speculative optimization | inherit parent; assigned code/docs/tests only |
| `aevio-sync-updater` | auto-update, latest.json, install.sh | unverified update paths | inherit parent; assigned config/docs/tests only |
| `aevio-testing` | test matrix, coverage, flaky tests | test framework additions | inherit parent; assigned tests/docs only |
| `aevio-server-sync` | Postgres sync backend, auth, membership | desktop SQLite, React UI | inherit parent; assigned server/docs/tests only |
| `aevio-ios-watch` | iOS/watch Swift apps | desktop Rust/React | inherit parent; assigned iOS/watch/docs/tests only |
| `aevio-capture-activity` | capture, breaks, pulse, usage, work intelligence | privacy leaks, unverified telemetry | inherit parent; assigned capture/docs/tests only |
| `aevio-coach-energy` | coach and energy features | speculative models | inherit parent; assigned coach/energy/docs/tests only |
| `aevio-auth-permissions` | auth, permissions, capabilities, ACL | privilege escalation, secrets | inherit parent; assigned auth/docs/tests only |
| `aevio-watch-bridge` | watch bridge, WatchConnectivity | unverified payloads | inherit parent; assigned bridge/docs/tests only |
| `aevio-tray-widget` | tray, widget, status bar, menu | platform-specific default path | inherit parent; assigned tray/widget/docs/tests only |

Every TOML agent includes purpose, responsibilities, exclusions, activation, boundaries, tools, permission policy, context budget, I/O contract, quality gates, and escalation rules. The shared skill prevents those rules from diverging.

Existing global agents such as `planner`, `architect`, `code-reviewer`, and `security-reviewer` were not copied into the repository. They remain available as generic fallbacks; Aevio agents win when repository boundaries or retrieval conventions matter.
