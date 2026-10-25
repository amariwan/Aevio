---
id: ref.project-analysis
title: Project Analysis
category: reference
domain: platform
status: implemented
source_paths: []
related_docs: []
platforms: [macos, windows]
verification_commit: working-tree
---


# Aevio – Vollständige Projektanalyse & Dokumentation

> Generiert aus dem aktuellen Working-Tree-Stand (Version 0.9.29, Branch `master`).
> Umfang: Monorepo, Rust-Core, Tauri-App, Swift-Sidecar, React-Frontend, CLI, Shared Packages, Datenmodell, IPC, Features, Rize-Parity-Mapping, WIP-Status, Risiken, nächste Schritte.

---

## 1. Vision & Produkt

**Aevio** ist eine lokale, private, Open-Source-Desktop-App (AGPL-3.0) für automatisches Time-Tracking auf dem Mac – als freie Alternative zu Aevio.io / Rize.

- **Kernversprechen:** Aktivität im Hintergrund erfassen (Foreground-App, Fenstertitel via Accessibility, Browser-URL in Safari/Chromium), in reviewbare Time Entries falten, mit on-device-KI kategorisieren und projektzuordnen, daraus Analysen, Timesheets und Rechnungen ableiten.
- **Privacy:** Keine Telemetrie, keine Keystroke-Logs, keine Screenshots, keine Cloud, kein Account. Kategorisierung läuft on-device (Apple Foundation Models, NaturalLanguage, Create ML).
- **Zielgruppen:** Einzelpersonen (Fokus/Ablenkungen), Freelancer (kunden-/projektbezogene abrechenbare Zeiten, Rechnungen), Agenturen (Timesheets, Profitabilität, Kapazität), Teams (Rollen, Governance, Sichtbarkeit).

Aktuelle Version: **0.9.29**. Basis-Commit `chore: initialize Aevio repository`, plus großer uncommitted Feature-Sweep.

---

## 2. Monorepo-Struktur

```
Aevio/
├── apps/
│   ├── api/          aevio-core (Rust-Library, shared by Tauri + CLI)
│   ├── cli/          aevio Binary (clap, MCP-Server)
│   ├── desktop/tauri Tauri 2 App (Rust core + Swift sidecar build)
│   ├── web/          Next.js 16 Static Export (Turbopack) → Tauri webview
│   ├── ios/          Swift (separat, nicht im Desktop-Verify-Flow)
│   └── watch/        Swift (separat)
├── packages/
│   ├── api-client/   Shared TS API-Client
│   ├── config/       Shared Config
│   ├── types/        Shared TS Types
│   ├── ui/           Shared UI
│   └── web/          Shared Web
├── scripts/          dev-screenshot.sh, app-drive.sh, verify.sh, tests
├── docs/             Architektur, Features, Pläne, Specs, Rize-Katalog
├── graphify-out/     Knowledge-Graph (committed, dirty erwartet)
└── Cargo.toml        Workspace-Root
```

### package.json (Root)
- Workspaces: `apps/*`, `packages/*`
- Scripts: `dev`, `dev:web`, `build`, `typecheck`, `verify`, `fix`, `prepare` (husky)
- Deps: @tauri-apps, @tanstack (react-query/form/table), kbar, zod, zustand, recharts, dice-ui, @sentry/nextjs, nuqs
- PM: bun@1.4.3

### Cargo Workspace
- Root Cargo.toml → members `apps/api`, `apps/desktop/tauri`, `apps/cli`
- Edition: api 2024, tauri/cli 2021. License AGPL-3.0-only.

---

## 3. Tech-Stack

| Layer | Technologie |
|---|---|
| Desktop | Tauri 2 (tray-icon, image-png, macos-private-api), Rust |
| Webview | React 19, Next.js 16 (static export, Turbopack), Tailwind CSS 4 |
| Shared Core | Rust crate `aevio-core` (rusqlite bundled, serde, chrono, tokio, reqwest rustls, interprocess) |
| On-device ML | Swift-Sidecar: Foundation Models, NLEmbedding, Create ML (Xcode 26+). Weak-linked → Fallback ohne Apple Intelligence |
| DB | SQLite (bundled), versionierte Migrationen |
| CLI | clap 4, clap_complete, aevio-core |
| PDF | printpdf 0.8.2 (png, jpeg) |
| Images | image 0.25 (png, jpeg) |
| Notifications | tauri-plugin-notification 2 |
| IPC | interprocess (Unix sockets / Windows named pipes) |
| Qualität | oxlint, oxfmt, tsc, cargo fmt, cargo clippy (-D warnings) |

---

## 4. Plattformen

| Plattform | Status | AI-Tier |
|---|---|---|
| macOS Apple Silicon | ✅ Gebaut & getestet | T0+T1+T2 (Foundation Models) |
| macOS Intel | 🔶 Untestet | T0+T1 (kein Foundation Model) |
| Windows | 🧪 Experimentell, CI-geprüft | T0 (rules only) |
| Linux | ❌ Nicht unterstützt (Capture fehlt) | — |

OS-spezifischer Code: `cfg_attr`-Pfad-Pattern (shared types im Parent, eine Impl pro Plattform). Frontend: `src/lib/platform.ts` als `Record<Platform, T>` – nie bare "is Mac"-Boolean.

---

## 5. Architektur & Kerninvarianten

### Datenfluss
```
Rust core (apps/desktop/tauri) owns all durable state (SQLite)
  capture → segments → entry builder → time entries
  classification worker: T0 rules → T1 personal (kNN + Create ML) → T2 Foundation Model
  arbiter blends, calibrates confidence, pre-fills/auto-approves
  learning loop: verdicts → kNN, rule offers, gated retrain, isotonic calibration
  also: projects, reports/export, timers, settings, tray, coach, agent, ...
  IPC commands + pushed events
    ↕ JSON lines over stdio
Swift sidecar (Foundation Models, NLEmbedding, Create ML)
  ↕
React view (src): typed Route union, pages, useTauriEvent
```

### Kerninvarianten
1. **Rust besitzt allen durable State.** React hält nur Route, transienten UI-State, letzten Snapshot. Rust pusht Events bei Änderungen.
2. **Geld = Integer-USD-Cents** in Rust (`invoices/money.rs`).
3. **Invoice-PDFs in Rust** (`invoices/pdf.rs`, printpdf) → preview in webview, save, archive on finalize. Keine HTML-Imitation, keine TS-Totals.
4. **AI-Tier:** T0 Rules (100% confident bei Hit), T1 Personal (kNN over embeddings + Create ML classifier), T2 Foundation Model (constrained auf live category/project lists, schreibt Description, re-sampled bei Disagreement mit T1).
5. **Arbiter:** blendet Tiers, lehnt sich an personal signals bei mehr Labels. Confidence cap 90% bis 50 Outcomes; danach isotonische Kalibrierungskurve auf eigenen Verdicts. Pre-fill ab 60%, Auto-Approve nur wenn beide Felder Threshold (95% default) überschreiten und Entry unberührt.
6. **Lernschleife:** Retrain nach 20+ neuen Labels oder nächtlich, nur bei idle + AC + kein Low-Power. Neues Modell nur bei nicht-schlechterer Holdout-Performance; versioniert.
7. **Capture:** sampelt foreground window im Background, faltet in Segments. Schließt bei idle (kein Input, außer App hält Display awake für Video), sleep/lid-close. App Nap assertion für präzises Sampling bei verstecktem Fenster.
8. **Entry Builder:** Segments → Time Entries. Eine Session = ein Entry (egal wie viele Apps). Gaps ≤5min bleiben drin. Sessions <5min dropped. Berührt keine approved Entries.
9. **User-visible Reminders** müssen native OS-Notifications auf allen Plattformen sein (macOS/Windows/Linux) via shared Rust/Tauri-Abstraction + Notification-Plugin. Keine in-app-toasts als Ersatz.
10. **Alle native Funktionalität cross-platform by design:** shared Rust/Tauri capability boundary + explizite macOS/Windows/Linux-Impls oder ehrliche unsupported/fallback via `cfg_attr`-Pattern.

---

## 6. Rust Core: `apps/api` (`aevio-core`)

### Cargo.toml
- name `aevio-core`, version 0.9.29, edition 2024
- Deps: dirs, serde (+derive), rusqlite (bundled), serde_json, interprocess, uuid (v7, serde), chrono (clock, std), regex, chrono-tz, reqwest (json, rustls, system-proxy), futures-lite, tokio (rt, time)
- Unix: libc; Windows: widestring, windows (0.61, Security/Threading)

### `src/lib.rs` – Module
```
activity, ai, automation, bootstrap, breaks, capture, categories, clients, coach,
decision, energy, entries, finance, governance, integrations, invoices, ipc,
platform, projects, reports, settings, timers, workspace
```
Compatibility facades: `bootstrap::{migrations, paths, state}`, `entries::query::*`, `entries::{builder, models}`, `ipc::{protocol, rpc}`. `APP_IDENTIFIER = "com.offlinestudios.aevio"`.

### Modul-Details

#### activity
- `approval_lock.rs`, `break_regressions.rs`, `deletions/`, `history.rs`, `split_regressions.rs`, `store/` (integriert tasks.rs, integrations.rs), `store.rs`, `store_tests.rs`, `mod.rs`
- Activity-Store mit Task- und Integration-Anbindung, Deletions-Tracking, History, Approval-Locks.

#### ai
- `arbiter.rs`, `calibration.rs`, `knn.rs`, `rules.rs`, `store/`, `store_artifacts.rs`, `store.rs`, `mod.rs`
- T0/T1/T2-Tier, Isotonische Kalibrierung, kNN over embeddings, Rule-Store.

#### automation
- `automation.rs` – Workflow-Automations (neu).

#### bootstrap
- `migrations.rs` (+tests), `paths.rs`, `state.rs`, `mod.rs` – Schema-Migrationen, Pfad-Auflösung (app_data_dir), AppState.

#### breaks
- `breaks/mod.rs` – Pausen-Management.

#### capture
- `capture/mod.rs` – Foreground-Sampling, Idle/Sleep-Detection.

#### categories, clients
- CRUD für Kategorien und Clients.

#### coach
- `adaptive.rs`, `adaptive-schema.sql`, `assistance.rs`, `brain.rs`, `brain-schema.sql`, `dates.rs`, `focus.rs`, `focus-schema.sql`, `intelligence.rs`, `interventions.rs`, `legacy.rs`, `models.rs`, `policies.rs`, `repository/`, `schema.sql`, `service.rs`, `tests.rs`, `mod.rs`
- Vollständiger Productivity Coach: date-spezifische Goals/Capacity/Notes/Energy, Task-References, Estimates, Priority, Order, Completion, Postponement, Carryover, Task-Breakdown, Just Start (6 Schwierigkeiten), Daily Priority Suggestions, Progress-Tracking, 5-state Reviews, Focus-Sessions, Body Doubling, Brain Dump, Routine-Templates, Weekly Aggregates, Memory-Notes, Coach-Settings, Deletion.

#### decision
- `config.rs`, `domain.rs`, `mod.rs`, `policy.rs`, `providers.rs`, `router.rs`, `use_cases.rs`
- Bounded Rust-Domain: typed Questions (Choice, Noul, Score), Result-Validation, Routing (auto/local_only/cloud/rules_only/disabled), Policy-Evaluation. Laya (local HTTP) + Jev (REST) Adapter. Cloud opt-in, backend-only Keys, Timeouts, Retries. Tauri-Command `coach_decision`.

#### energy
- `energy/mod.rs` – Energie-Tracking (Batterie).

#### entries
- `builder.rs`, `models.rs`, `query.rs`, `mod.rs` – Segment→Entry-Folding, Models, Queries (APP_KEY, EntryFilter, MAX_QUERY_ROWS, NONE, query_entries, time_entry_from_row).

#### finance
- `finance.rs` – Finanzdaten (Rate Cards, Contracts, Capacity Plans, Dashboards).

#### governance
- `governance.rs` – Team/Department/Member-Governance.

#### integrations
- `integrations/mod.rs`, `models.rs` – Externe Integrationen.

#### invoices
- `invoices/mod.rs` – Draft/Sent/Paid/Voided, PDF, Quote, Billable Entries.

#### ipc
- `protocol.rs`, `rpc.rs`, `unix.rs`, `windows.rs`, `mod.rs` – IPC-Protokoll, RPC-Server (CLI-Kommunikation).

#### platform
- `platform/mod.rs` – Platform-Abstraktion.

#### projects
- `service.rs` (+tests), `mod.rs` – Projekte, Clients, Budgets, Due-Dates, AI-Hints, Discovery (Repo-Pfade, Forge-URLs, Editor-Titel), CSV-Import.

#### reports
- `query.rs` (+tests), `mod.rs` – Report-Definitionen & Runs.

#### settings
- `breaks.rs`, `models.rs`, `storage.rs`, `tracking.rs`, `validation.rs`, `mod.rs` – Settings (Theme, Accent, Menu-Bar, Close-Behavior, Storage, Retention, Expected Hours, Categories & AI, Breaks).

#### timers
- `store.rs`, `mod.rs` – Named Timers mit Segments, concurrent, restart-persistent.

#### workspace
- `workspace.rs` – Workspace-Modell.

---

## 7. Tauri App: `apps/desktop/tauri`

### src/
- `main.rs`, `lib.rs`, `commands.rs` (185 `pub fn`)
- Module: `activity.rs`, `ai/` (mod, sidecar, agent), `breaks/`, `capture.rs`, `capture/`, `coach/`, `energy/`, `energy.rs`, `i18n.rs`, `invoices/`, `login_item/`, `login_item.rs`, `permissions/`, `permissions.rs`, `pulse.rs`, `rpc.rs`, `summary.rs`, `tray.rs`, `updater.rs`, `usage/`, `usage.rs`, `watch_bridge.rs`, `widget.rs`

### commands.rs – Tauri-Commands (185)
Gruppiert nach Domain:
- **Watch/Docs/Notes/Comments/Attachments/Mentions:** `watch_pairing`, `list_documents`, `create_document`, `update_document`, `delete_document`, `list_notes`, `create_note`, `update_note`, `delete_note`, `list_comments`, `create_comment`, `list_attachments`, `create_attachment`, `list_mentions`
- **Cycles/Initiatives/Governance:** `list_cycles`, `create_cycle`, `list_initiatives`, `create_initiative`, `governance_snapshot`, `save_governance_*`, `authorize_governance`
- **Coach:** `coach_snapshot`, `list_coach_tasks`, `create_coach_task`, `update_coach_task`, `complete_coach_task`, `delete_coach_task`
- **Timers:** `list_timers`, `create_timer`, `start_timer`, `pause_timer`, `pause_timer_by_id`, `reset_timer`, `rename_timer`, `add_timer_segment`, `update_timer_segment`, `delete_timer_segment`, `delete_timer`
- **Activity/Usage/Capture:** `activity_snapshot`, `usage_snapshot`, `set_capture_enabled`, `set_idle_threshold`, `start_session`, `stop_session`, `mark_segment_reviewed`
- **Categories/Projects/Tasks/Labels/Integrations/Clients:** CRUD + `project_rules`, `preview_project_hints`, `discover_projects`, `dismiss_project_suggestion`, `import_projects_csv`, `project_stats`
- **Invoices:** `list_invoices`, `get_invoice`, `list_billable_entries`, `quote_invoice`, `render_invoice_preview`, `get_invoice_pdf`, `save_invoice_draft`, `finalize_invoice`, `set_invoice_paid`, `void_invoice`, `delete_draft_invoice`, `delete_void_invoice`, `get_invoice_profile`, `update_invoice_profile`, `get_invoice_logo`, `set_invoice_logo`, `clear_invoice_logo`
- **Time Entries:** `list_time_entries`, `get_entry_detail`, `update_time_entry`, `approve_time_entries`, `unapprove_time_entries`, `update_time_entries`, `reject_time_entry`, `split_time_entry`, `merge_time_entries`, `delete_time_entry`, `delete_time_entries`, `undo`, `create_time_entry`, `rebuild_time_entries`
- **Apps:** `list_apps`, `update_app`
- **AI/Agent/Work:** `work_ask`, `work_relationships_list`, `work_relationship_add`, `ai_status`, `agent_chat`, `retry_classification`, `resolve_rule_suggestion`, `ai_metrics`, `ai_retrain`, `ai_reset_learned`
- **Automation/Workflow:** `automation_snapshot`, `daily_workflow_snapshot`, `run_daily_workflow`, `create_routine`, `create_report`, `create_dashboard`, `run_routine`, `run_report`
- **Finance:** `finance_snapshot`, `list_finance_rate_cards`, `save_finance_rate_card`, `list_finance_contracts`, `save_finance_contract`, `share_finance_dashboard`, `list_finance_capacity_plans`, `save_finance_capacity_plan`, `list_finance_dashboard_shares`, `revoke_finance_dashboard_share`
- **Query/Export:** `query_time_entries`, `entry_rollup`, `export_time_entries`
- **Widget/Pulse/Summary:** `resize_widget`, `resize_pulse_panel`, `hide_pulse_panel`, `open_productivity_summary`, `hide_productivity_summary`, `open_main_window`
- **Settings:** `get_settings`, `update_settings`, `get_decision_settings`, `update_decision_settings`, `decision_health`
- **Maintenance:** `sweep_retention`
- **OS:** `launch_at_login`, `set_launch_at_login`, `check_permissions`, `storage_paths`
- **Energy:** `get_energy_summary`, `query_energy_history`, `reset_energy_history`
- **Status/Breaks:** `update_status`, `break_state`, `dismiss_stopwatch_reminder`, `start_break`, `end_break`, `snooze_break`, `skip_break`, `expand_break_reminder`, `extend_break`, `pause_break_reminders`, `list_breaks`, `resize_reminder_panel`, `open_break_settings`, `dev_sample_break_reminder`, `preview_break_chime`

### Swift Sidecar (`swift/Sources/aevio-ml`)
- `Entry.swift`, `Protocol.swift` (protocolVersion + PROTOCOL_VERSION müssen zusammen gebumpt werden)
- `Agent.swift` – Agent/Chat
- `Coach.swift`, `CoachPlanning.swift` – Coach on-device

### Capabilities (Tauri)
- `capabilities/summary.json`, `capabilities/widget.json` (neue Feature-Capabilities)

### Tauri-Config
- `tauri.conf.json`, `tauri.macos.conf.json`, `Info.plist`

---

## 8. Web-Frontend: `apps/web`

### `src/shell/App.tsx`
- Typed Route-Union-basierte Navigation (kein Router-Lib), Browser-style History (Stack + Cursor).
- Rendert Feature-Komponenten per `switch (currentRoute.name)`.
- SettingsProvider, I18nProvider, useTauriEvent, useUpdates, useBreaks.

### `src/lib/types.ts` – Route Union (25+ Varianten)
`dashboard`, `cycles`, `initiatives`, `notes`, `documents`, `issues`, `focus`, `agent`, `finance`, `coach`, `catalog`, `tasks`, `teams`, `calendar`, `timesheet`, `timers`, `apps`, `entries`, `timesheets`, `projects`, `clients`, `invoices`, `settings`.

### `src/components/layout/Sidebar.tsx`
Gruppen: **Track** (Dashboard, Focus, Agent, Coach, Calendar, My Timesheet, Apps), **Analyze** (Time Entries, Timesheets, Finance), **Work** (Catalog, Tasks, Cycles, Initiatives, Notes, Documents, Issues, Projects, Clients, Invoices), plus Teams etc.

### `src/features/` (30+)
`agent`, `ai`, `apps`, `calendar`, `catalog`, `clients`, `coach`, `collaboration`, `cycles`, `dashboard`, `documents`, `entries`, `finance`, `focus`, `initiatives`, `invoices`, `issues`, `notes`, `projects`, `pulse`, `settings`, `summary`, `tasks`, `teams`, `timers`, `timesheets`, `widget`, `workspace`.

### `src/components/ui/` (shadcn)
`field`, `native-select`, `tabs`, `textarea` (+ bestehende shadcn-Komponenten). Policy: Neue UI ausschließlich shadcn/ui.

### `src/hooks/`
`useBreaks`, `useCatalog`, `useI18n`, `useMediaQuery`, `useSettings`, `useTauriEvent`, `useUpdates`.

### `src/lib/`
`api.ts`, `dates.ts`, `entries.ts`, `format.ts`, `i18n.ts`, `ipc/events.ts`, `platform.ts`, `settings.ts`, `types.ts`.

### Styling
- Tailwind 4 Utility-Classes mit semantischen Tokens in `src/styles/app.css` (`text-fg-muted`, `border-line`, `bg-surface`, ...). Nie raw color values.
- Themes: `themes.css` + `themes/{forest,mono,ocean,sunset,violet,zinc,reference}.css`.

---

## 9. CLI: `apps/cli`

- Binary `aevio`, clap 4, aevio-core.
- `src/main.rs`, `src/render.rs`, `src/mcp.rs` (MCP-Server).
- `COMMANDS.md` – vollständige Command-Liste (muss bei Änderungen aktualisiert werden).
- Treibt Aevio vom Terminal: start app, status, list/add/edit/approve/report/export time, timers, projects, clients, categories, settings.
- Mit geschlossener App: alles außer Tracking funktioniert auf stored data.
- `aevio install-path` legt CLI auf PATH.

---

## 10. iOS / Watch (separat)
- `apps/ios/Aevio`, `apps/watch/AevioWatch` – Swift, eigene Xcode-Projekte.
- `watch_bridge.rs` verbindet Desktop mit Watch.
- Widget-Capability vorhanden.
- Nicht Teil des Desktop-Verify-Flows.

---

## 11. On-device AI Sidecar

- Kompiliert von `build.rs` mit `swift build`, shipped als Tauri `externalBin`.
- Xcode 26+ Command Line Tools bauen echten Sidecar; ohne sie staged `build.rs` einen leeren Placeholder → App läuft mit rules-only-Fallback.
- Wire-Format: `protocolVersion` in `Entry.swift` + `PROTOCOL_VERSION` in `ai/sidecar.rs` müssen zusammen gebumpt werden.
- Foundation Models weak-linked → gleiche Binary läuft ohne Apple Intelligence und meldet Fallback.

---

## 12. Datenmodell & Migrationen

- SQLite, versionierte Migrationen in `crates/aevio-core/src/bootstrap/migrations.rs` (+tests).
- Core-Entities: Activity/Segments, Time Entries, Categories, Projects, Clients, Tasks, Labels, Invoices, Timers, Settings, Coach (Goals/Tasks/Reviews/Memory/Routines), Documents, Notes, Comments, Attachments, Mentions, Cycles, Initiatives, Governance (Teams/Departments/Members), Finance (Rate Cards/Contracts/Capacity Plans/Dashboards), Energy, Breaks, Automation, Integrations, Workspace, Usage, Pulse.
- `activity/store/` gesplittet in `tasks.rs`, `integrations.rs`, `deletions/`, History.
- Coach hat eigene Schemas: `schema.sql`, `brain-schema.sql`, `focus-schema.sql`, `adaptive-schema.sql`.

---

## 13. IPC & RPC

- `ipc/protocol.rs`, `ipc/rpc.rs`, `ipc/unix.rs`, `ipc/windows.rs`.
- Unix sockets / Windows named pipes hinter einer local-socket API.
- CLI spricht über RPC mit laufender App; bei geschlossener App direkter DB-Zugriff via aevio-core.
- Peer-User-Verifikation auf Windows (security descriptors).

---

## 14. Coach (Detail)

Siehe `docs/features/ai-coach/`. Workflows:
- Editierbare date-spezifische Goals, Capacity, Notes, optionales Energy, Planning/Skip-State.
- Personal/Project-Task-References, neue personal Tasks, Estimates, Priority, Order, Completion, Postponement, explizites Carryover.
- Task-Breakdown editierbar/reviewbar; Just Start für 6 Startschwierigkeiten (Rules offline; on-device optional & gelabelt).
- Daily Priority Suggestions (Rules oder on-device), constrained auf existierende Task-Identities + verfügbare Zeit. Acceptance ändert nur Ordering; Postponement explizit.
- Daily Progress: Task-Completion, Goal-Assessment, Work-Duration, Unassigned-Time. Editierbare 5-state Reviews, konfigurierbares native Review-Angebot.
- Focus-Sessions über ActivityStore, optionaler Visual-Timer, Recovery-Context, sanftes End-Check-in, Body Doubling. Floating-Widget zeigt Goal/Task/Next-Step/Progress, Pause/Resume, konfigurierbar.
- Opt-in Hints nach häufigen Context-Changes (Beobachtung, keine Diagnose).
- Weekly lokale Aggregate, previous Week, explizite/confirmed Memory-Notes, optionale Duration-Estimates aus wiederholten exakten Task-Identities, user-confirmed Insights.
- Brain Dump, reviewbare lokale Kategorie-Suggestions, idempotente Conversion zu personal Task, editierbare Routine-Templates (max 1x/Tag auf explizite Anfrage).
- Coach-Enablement, Style, optionales on-device AI, native Reminders, Quiet Hours, Cooldowns, Timer/Widget-Options, vollständige Coach-Daten-Löschung.

Grenzen: Local desktop data dir = Ownership-Boundary. Kein remote Coach-Service. Kein externer AI-Provider, kein Calendar-Feed, keine Voice-Transcription. Reminders brauchen laufende App/Tray + OS-Permission; Delivery bei geschlossener App nicht implementiert. Brain-Dump-Categorization ist transparente lokale Rule-Suggestion (kein GenAI). Capacity = verfügbare Minuten nach Terminen/Pausen. Estimates = einfache historische Evidenz. Memory nie inferierte Psychologie.

---

## 15. Decision-Engine (Detail)

Bounded Rust-Domain in `crates/aevio-core/src/decision/`. Besitzt typed Questions, Result-Validation, Routing, Policy-Evaluation. Nicht Coach-Scheduling/Presentation/Action-Execution.

- Contracts: `Choice`, `Noul`, `Score` mit bounded Context + Response-Validation.
- Deterministische Rules für unterstützte Coach-Decisions, sonst explizites Abstain.
- Laya (local HTTP) + Jev (REST) Adapter.
- Routing: `auto`, `local_only`, `cloud`, `rules_only`, `disabled`.
- `auto` darf bei transport-level local-provider failure auf deterministic rules fallbacken. Sendet nie an Jev ohne explizites cloud opt-in + backend key. Malformed/semantisch invalide Provider-Response wird rejected (nicht retried, nicht silent converted).
- Remote-Requests enthalten nur bounded `DecisionContext.summary` + typed questions. Keine raw activity streams, keine API-Keys, keine personal document contents.
- Coach bleibt Owner von Intervention-Priority, Quiet Hours, Snooze, Scheduling.
- Exposed Command: `coach_decision`. Bestehende Coach-Commands bleiben kompatibel.

---

## 16. Energy-Modul (in Arbeit)

- `crates/aevio-core/src/energy/mod.rs`, `apps/desktop/tauri/src/energy.rs`, `apps/web/src/features/settings/components/BatteryEnergyMonitor.tsx`.
- Energy-Summary, History-Query, Reset.
- Plan: `docs/superpowers/plans/2026-10-20-energy-core-extraction.md`.
- Coach-Energy-Check-in (freiwillig, kurz, keine automatische Diagnose mentaler Zustände).

---

## 17. Breaks (in Arbeit)

- `crates/aevio-core/src/breaks/mod.rs`, `apps/desktop/tauri/src/breaks/`, `apps/web` Break-UI.
- Commands: `break_state`, `start_break`, `end_break`, `snooze_break`, `skip_break`, `expand_break_reminder`, `extend_break`, `pause_break_reminders`, `list_breaks`, `resize_reminder_panel`, `open_break_settings`, `dev_sample_break_reminder`, `preview_break_chime`.
- Plan: `docs/superpowers/plans/2026-10-20-breaks-core-extraction.md`.

---

## 18. Automation (neu)

- `crates/aevio-core/src/automation.rs`, `apps/desktop/tauri/src/` (automation_snapshot, create_routine, run_routine).
- Web: `automation_snapshot`, `create_routine`, `run_routine`.
- Feature: benutzerdefinierte Regeln ("Beim Start von Projekt X die passende Fokusumgebung aktivieren").

---

## 19. Finance-Modul

- `crates/aevio-core/src/finance.rs`, `apps/desktop/tauri/src/` (finance_snapshot, rate cards, contracts, capacity plans, dashboard shares).
- Web: `features/finance`.
- Rate Cards, Contracts, Capacity Plans, Dashboard Sharing (revoke).

---

## 20. Governance

- `crates/aevio-core/src/governance.rs`, Tauri: `governance_snapshot`, `save_governance_team/department/member/defaults`, `authorize_governance`.
- Teams, Departments, Members, Defaults, Authorization.

---

## 21. Integrationen

- `crates/aevio-core/src/integrations/` (mod, models), Tauri: `list_integrations`, `save_integration`.
- Externe Integrationen (Modell + CRUD).

---

## 22. Pulse & Reminders

- `apps/desktop/tauri/src/pulse.rs`, Web: `features/pulse` (Pulse, Reminder).
- `resize_pulse_panel`, `hide_pulse_panel`.
- Native Notifications (cross-platform).

---

## 23. Widget & Summary

- `apps/desktop/tauri/src/widget.rs`, `summary.rs`, `usage.rs`, `usage/`.
- Capabilities: `widget.json`, `summary.json`.
- `resize_widget`, `open_productivity_summary`, `hide_productivity_summary`, `open_main_window`.

---

## 24. Watch-Bridge & Mobile

- `apps/desktop/tauri/src/watch_bridge.rs` – Desktop↔Watch.
- `apps/ios/`, `apps/watch/` – Swift-Apps (separat).
- Widget-Capability für iPhone-Widget (Quick Capture).

---

## 25. i18n

- `apps/desktop/tauri/src/i18n.rs`, Web: `hooks/useI18n.tsx`, `lib/i18n.ts`.
- Mehrsprachigkeit (in Arbeit).

---

## 26. Work-Management (Tasks, Issues, Cycles, Initiatives, Documents, Notes)

- Tauri-Commands: Documents, Notes, Comments, Attachments, Mentions, Cycles, Initiatives.
- Web-Features: `tasks`, `issues`, `cycles`, `initiatives`, `documents`, `notes`, `catalog`.
- Task-Relationships: `work_relationships_list`, `work_relationship_add`, `work_ask`.

---

## 27. Rize-Parity (Feature-Katalog)

Basiert auf `docs/plan/Rize_Feature_Katalog_2026.md` (Rize-Feature-Inventur). Aevio portiert Rize-Features in lokaler/privater Form. Navigation in Aevio (Sidebar) folgt Rize-Struktur: Track / Analyze / Work / Admin.

### Mapping Feature-Liste 4–18 → Codebase

| # | Feature | Prio | Status |
|---|---|---|---|
| 4 | AI Work Assistant | Sehr hoch | Teilweise: `features/agent`, `ai/agent.rs`, Swift `Agent.swift`, Decision-Engine. Q&A + Links auszubauen. |
| 5 | Smart Project Detection | Sehr hoch | Teilweise: Project-Discovery (Repo-Pfade, Forge-URLs, Editor-Titel), AI-Hints→Rules, Learning-Loop. |
| 6 | Deadline Intelligence | Hoch | Teilweise: Due-Dates/Budgets, Coach-Capacity/Estimates. Explizite "reicht die Zeit?"-Logik fehlt. |
| 7 | Quick Capture Everywhere | Hoch | Teilweise: Tray, Widget, Watch-Bridge, iOS. Global macOS-Shortcut zu prüfen. |
| 8 | Task Dependency Graph | Hoch | Teilweise: Tasks-Feature, Coach-Tasks/Carryover. Sichtbare Blocker-Graphen fehlen. |
| 9 | Adaptive Focus Environment | Mittel | Teilweise: Focus (Soundscapes, Silent Mode). Adaptive Vorschläge fehlen. |
| 10 | Meeting Recovery | Hoch | Teilweise: Pulse, Coach-Recovery, Focus. Meeting-Modul (Transkripte) fehlt. |
| 11 | Daily Shutdown Ritual | Hoch | Teilweise: Coach Daily-Review, Carryover, Brain-Dump, Routines. Strukturiertes "für morgen sichern" auszubauen. |
| 12 | Personal Command Center | Hoch | Vorhanden: kbar in Deps, TopBar/Command-Palette. |
| 13 | Flexible Energy Planning | Mittel | Implementiert (in Arbeit): Energy-Modul + BatteryEnergyMonitor + Coach-Energy-Check-in. |
| 14 | Goal-to-Outcome Tracking | Hoch | Teilweise: Coach-Goals + Initiatives/Cycles. Milestone-Tracking auszubauen. |
| 15 | Aevio Continuity (E2E) | Hoch | Teilweise: iOS/Watch/Widget. Sync-Server laut README "❌ Not started". |
| 16 | Workflow Automations | Mittel | Implementiert (neu): `automation.rs`. |
| 17 | Task Relationship Intelligence | Mittel | Teilweise: Coach Brain-Dump (lokale Kategorie-Suggestion). Duplikat-Erkennung fehlt. |
| 18 | Local Productivity Knowledge Graph | Sehr hoch | Grundlage: Coach `brain`-Schema, Activity-Store. KG noch zu bauen. |

---

## 28. Qualitätssicherung

### Scripts
- `bun run fix` – safe auto-fixes: oxfmt --write, oxlint --fix, cargo fmt, cargo clippy --fix
- `bun run verify` – canonical check: oxlint + oxfmt + tsc --noEmit + cargo fmt --check + cargo clippy -- -D warnings

### Tests
- Frontend: `node --test scripts/*.test.mjs` (z.B. `breaks.test.mjs`, `coach.test.mjs`, `native-driver.test.mjs`, `task-reminder-date.test.mjs`)
- Rust: `cargo test --manifest-path apps/desktop/tauri/Cargo.toml` (oder `cargo test -p aevio-core <modul>`)
- Specimen Invoices: `AEVIO_INVOICE_SAMPLES=<dir> cargo test --manifest-path apps/desktop/tauri/Cargo.toml --lib invoices::tests::renders`

### Visuelle Verifikation
- `bash scripts/dev-screenshot.sh` – isolated tauri dev, screenshot, teardown (macOS only)
- `bash scripts/app-drive.sh start` – interactive isolated instance (siehe `.agents/skills/drive-app/SKILL.md`)
- Kein Playwright/WebDriver für App-Window (kein Backend auf macOS).

### CI
- `.github/workflows/verify.yml`: `bun run verify` auf macos-26 + windows-latest (macOS braucht Xcode 26+ für Swift-Sidecar). Trigger: `run-ci`-Label auf PRs.
- `.github/workflows/release.yml`: signed macOS .dmg/.app auf `v*`-Tags → GitHub Releases + `latest.json` Updater-Manifest.

---

## 29. Aktuelle Entwicklungsphase (WIP)

Working-Tree ist ein großer in-flight-Sweep (87 geänderte Dateien + viele neue). Hauptthemen:

1. **Core-Domain-Extraktion** (`aevio-core` aus Tauri herausgezogen): `bootstrap`, `entries`, `ipc`, `activity` mit Store-Split. Docs: `core-domain-layout.md`, `backend-modularization.md`. Ziel: CLI + App teilen einen Core.
2. **Energy-Modul** (Core-Extraction in Arbeit).
3. **Breaks** (Core-Extraction in Arbeit).
4. **Invoices-Core-Extraction** (Plan vorhanden).
5. **Themes** (Plan/Spec, `styles/themes/*`).
6. **Coach** stark erweitert (ProductivityCoach 387 LOC Δ, neue API/Types/Hooks, Swift Coach/CoachPlanning, Decision-Engine).
7. **Decision-Engine** (Laya/Jev-Provider, typed Questions, Routing, Tauri-Command).
8. **Agent** (`ai/agent.rs`, Swift `Agent.swift`, `features/agent`).
9. **i18n** (`useI18n`, `lib/i18n.ts`, `i18n.rs`).
10. **Summary/Usage/Widget/Watch-Bridge** (iOS/Watch-Anbindung).
11. **Automation** (`automation.rs`, `feat: add automation and focus workflows`).
12. **MCP-Server im CLI** (`apps/cli/src/mcp.rs`, `docs/mcp.md`).

---

## 30. Technische Beobachtungen & Risiken

1. **Riesiger uncommitted Sweep** → hohe Integrations-Risiko. `verify` muss vor jedem Commit laufen. Keine Test-Frameworks/CI/Hooks ohne Notwendigkeit (AGENTS.md).
2. **README veraltet** gegenüber tatsächlichem Funktionsumfang (listet nur "early release"-Features). Doku-Drift.
3. **Sync-Server "❌ Not started"**, aber iOS/Watch/Widget/Continuity-Features im Code → inkonsistenter Reifegrad.
4. **Windows/Linux AI = rules-only**; neue Features (Coach, Agent, Decision) hängen an macOS Foundation Models → Plattform-Parität fragil. `cfg_attr`-Pattern + unsupported-Adapter vorhanden, aber nicht runtime-getestet.
5. **Core-Extraktion in flight** → CLI + App müssen konsistent bleiben; `COMMANDS.md` bei Änderungen aktualisieren.
6. **Keine automatisierten Web-App-Tests** (nur `scripts/*.test.mjs` + Rust-Tests). UI-Regressionen manuell/visuell.
7. **Versionierung:** jede user-facing Änderung → Patch-Bump + Conventional Commit. Aktuell 0.9.29; viele uncommitted Änderungen → Bump bei Merge.
8. **graphify-out** committed & dirty (erwartet nach Hooks) – nicht als Code-Änderung behandeln.
9. **Klock-Fork** (`../klock`) ist Enterprise-Fork; Änderungen manuell über git-ignored Port-Inboxes, nie durch Merging. Shared-Foundation-Fixes nach Merge hier für Klock dokumentieren (sync-klock skill).

---

## 31. Release-Prozess

- `RELEASING.md` für Release-Procedure.
- Jede user-facing Änderung: Patch-Bump in gleicher PR + Conventional Commit Subject.
- Release auf `v*`-Tags → signed macOS .dmg/.app + `latest.json` Updater-Manifest.

---

## 32. Dokumentations-Index

| Pfad | Inhalt |
|---|---|
| `README.md` | Produkt-Übersicht, Features (early release), Privacy, Under the hood |
| `AGENTS.md` | Agent-Instruktionen, Architektur, Verify, Platforms, CLI |
| `CLAUDE.md` | Kurze Agent-Hinweise |
| `RELEASING.md` | Release-Procedure |
| `docs/architecture-modernization.md` | Architektur-Modernisierung |
| `docs/backend-modularization.md` | Backend-Modularisierung |
| `docs/features/ai-coach/*` | Coach: overview, architecture, workflows, ai-orchestration, privacy, testing, data-model |
| `docs/features/decision-engine.md` | Decision-Engine-Scope |
| `docs/plan/Rize_Feature_Katalog_2026.md` | Rize-Feature-Inventur (Parity-Referenz) |
| `docs/mcp.md` | MCP-Dokumentation |
| `docs/superpowers/plans/*` | Implementierungspläne (core-domain, breaks, energy, invoices, themes) |
| `docs/superpowers/specs/*` | Design-Specs (backend-domain-structure, themes) |
| `apps/cli/COMMANDS.md` | CLI-Command-Referenz |

---

## 33. Empfohlene nächste Schritte

1. **Stabilisieren:** `bun run fix && bun run verify` + `cargo test --workspace` laufen lassen; WIP in logische Commits aufteilen (Core-Extraktion, Energy, Breaks, Invoices, Themes, Coach, Agent, i18n getrennt).
2. **README & Doku aktualisieren** auf erweiterten Funktionsumfang (oder bewusst als "in development" markieren).
3. **Feature-Liste 4–18 priorisieren:** Sehr-hoch-Items zuerst (#4 AI Work Assistant, #5 Smart Project Detection, #18 Knowledge Graph) – Infrastruktur (Agent, Coach Brain, Activity) bereits vorhanden.
4. **Plattform-Parität prüfen:** Windows/Linux-Adapter für neue Coach/Agent/Decision-Features als unsupported markieren + Fallbacks testen.
5. **Sync-Strategie klären:** Continuity (#15) zurückstellen oder E2E-Verschlüsselung definieren.
6. **Tests:** Rust-Unit-Tests für neue Core-Module (decision, energy, automation); Smoke-Tests für Web-Flows (Coach, Agent) via `app-drive.sh`.
7. **Version-Bump** bei Merge der WIP-Änderungen (0.9.29 → 0.9.30).

---

## 34. Kurzstatistik

- **Commits:** 2 auf master (frühe History) + großer WIP.
- **Rust-Crates:** aevio-core + Tauri (aevio_lib) + aevio-cli.
- **Web-Features:** 30+ Feature-Ordner.
- **Web-Routen:** 25+ Route-Varianten.
- **Tauri-Commands:** 185.
- **Core-Module (api):** 22.
- **Tauri-Module:** 20+.
- **Migrations:** versionierte SQLite-Migrationen.
- **Capabilities:** summary.json, widget.json.
- **Swift-Sidecar-Dateien:** Entry, Protocol, Agent, Coach, CoachPlanning.

---

*Ende der Analyse. Basis: Working-Tree-Stand zum Analysezeitpunkt.*
