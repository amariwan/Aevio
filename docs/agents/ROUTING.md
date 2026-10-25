---
id: agents.routing
title: Aevio Agent Routing and Delegation
category: agents
domain: agents
status: implemented
source_paths: [AGENTS.md, .codex/agents/aevio-orchestrator.toml, docs/agents/context-routing.md]
related_docs: [docs/agents/AGENT_CATALOG.md, docs/agents/CONTEXT_ENGINEERING.md, docs/agents/EVALUATION.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: []
---

# Routing

Route deterministically by the primary artifact and risk:

| Signal | Route | Required first context |
|---|---|---|
| architecture, cross-app, boundary, refactor design | `aevio-architecture` | architecture docs + Graphify + GitNexus impact |
| Rust, SQLite, schema, migration, IPC, event, CLI | `aevio-backend-persistence` | domain docs + data model + callers/tests |
| React, route, component, Tailwind, shadcn, Tauri consumer | `aevio-frontend-native` | feature docs + types/API/hooks + UI rules |
| bug, review, regression, security, privacy, test, performance | `aevio-quality-security` | diff + impact + relevant tests/logs |
| docs, lookup, retrieval, agent, skill, routing, drift | `aevio-docs-retrieval` | source-priority docs + exact source + index state |
| AI sidecar, Swift, protocol version, ML categorization, rules-only fallback | `aevio-ai-sidecar` | AI module docs + protocol constants + impacted symbols |
| invoices, money, cents, PDF, printpdf, invoice editor | `aevio-invoices` | invoice domain docs + money rules + PDF contracts |
| CLI, rpc, command surface, install-path, COMMANDS.md | `aevio-cli-rpc` | CLI docs + command surface + RPC contracts |
| release, version bump, CI, packaging, signed DMG | `aevio-release-ops` | release docs + workflow files + version manifests |
| data migration, schema evolution, import/export, backup/restore | `aevio-data-migrations` | data model + migrations + IPC contracts |
| platform-specific native code, tray, shortcuts, notifications, window control | `aevio-platform-native` | platform docs + capability matrix + impacted symbols |
| accessibility, i18n, responsive UI, inclusive UX | `aevio-accessibility-i18n` | UI rules + i18n patterns + impacted components |
| performance, profiling, benchmarks, N+1, bundle size, startup | `aevio-performance` | performance docs + hot paths + impacted symbols |
| updater, auto-update, latest.json, install.sh | `aevio-sync-updater` | release/update docs + impacted files |
| test strategy, coverage, flaky tests, test design | `aevio-testing` | relevant tests + contracts + impacted symbols |
| server-postgres, sync, multi-device, auth, membership | `aevio-server-sync` | sync docs + data model + API contracts |
| iOS, watchOS, Swift app, Apple platform | `aevio-ios-watch` | app docs + platform constraints + impacted symbols |
| capture, breaks, pulse, usage, work intelligence, activity tracking | `aevio-capture-activity` | capture docs + platform permissions + impacted symbols |
| coach, energy, productivity, wellness | `aevio-coach-energy` | feature docs + impacted symbols |
| auth, login, permissions, capabilities, ACL, access control | `aevio-auth-permissions` | auth docs + capability matrix + impacted symbols |
| watch bridge, WatchConnectivity, desktop-watch sync | `aevio-watch-bridge` | bridge docs + impacted symbols |
| tray, widget, status bar, menu | `aevio-tray-widget` | tray/widget docs + impacted symbols |
| mixed or unclear | `aevio-orchestrator`, then one owning specialist | AGENTS.md + bounded task context |

Delegation record:

```text
scope: exact paths/symbols and excluded paths
goal: one sentence
context: retrieved docs/graph evidence and estimated tokens
acceptance: observable checks
risk: LOW | MEDIUM | HIGH | CRITICAL | UNKNOWN
stop: condition that ends the specialist pass
output: paths, findings, verification, unresolved assumptions
```

Rules: one writer per file; no circular handoffs; no agent may spawn another without the orchestrator's explicit bounded contract; independent read-only work may run in parallel; implementation, migrations, and verification are ordered.
