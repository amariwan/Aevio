---
id: feat.current-inventory
title: Current Feature Inventory
category: features
domain: product
status: implemented
source_paths: [apps/web/src/features/, apps/web/src/lib/, apps/desktop/tauri/src/, crates/aevio-core/src/, crates/aevio-services/src/, crates/aevio-persistence/src/]
related_docs: [docs/features/time-tracking.md, docs/features/activity-capture.md, docs/features/work-management.md, docs/features/finance.md, docs/features/coach.md, docs/features/state-sync.md]
platforms: [macos, windows, ios, watchos]
verification_commit: working-tree
---

# Current Feature Inventory

This document records the product capabilities that are present in the current
working tree. It is an implementation inventory, not a roadmap and not a claim
that every native or cross-platform path has been manually verified.

## Status vocabulary

- **Implemented** — the feature has a product path in Rust and/or React and is
  part of the current application surface.
- **Partial** — the main capability exists, but important workflows, platform
  parity, or production integration are incomplete.
- **Experimental** — the capability exists behind a limited platform, sidecar,
  server, or development-only path.
- **Unverified** — source and contracts exist, but the current audit did not run
  the native app, target hardware, or an external service.

## 1. Activity capture and time tracking

**Status: Implemented on macOS; Windows experimental; Linux unsupported.**

- Background capture of the foreground application.
- Window-title capture through the platform accessibility boundary.
- Browser URL/domain capture for Safari and Chromium-based browsers.
- Configurable idle detection and sleep/idle segment handling.
- Tracking sessions that can be started and stopped explicitly.
- Activity segments with app, title, URL/domain, start/end, kind, review state,
  and optional project/category labels.
- Reviewable time entries generated from captured activity.
- Manual time-entry creation for past work.
- Entry editing, deletion, splitting, merging, approval, rejection, and bulk
  updates.
- Entry provenance (`auto`, `manual`, or `import`) and description origin.
- Billable flag and invoice linkage.
- AI classification state, confidence, rationale, alternatives, and review
  outcomes.
- Rules and learned project/category suggestions based on app, domain, labels,
  and personal history.

Primary sources: `crates/aevio-core/src/entries/`,
`crates/aevio-core/src/capture/`, `apps/web/src/features/entries/`,
`apps/desktop/tauri/src/capture.rs`.

## 2. Timers, sessions, and breaks

**Status: Implemented; native behavior remains platform-dependent.**

- Multiple named stopwatches.
- Start, pause, reset, rename, delete, and segment-level editing.
- Timer segments with accumulated duration.
- Tray-based timer control without opening the main window.
- Floating timer widget.
- Focus, meeting, activity, and break session kinds.
- Native break reminders with start, end, snooze, skip, extend, and pause-
  reminders actions.
- Fixed-time scheduled breaks and interval-based break reminders.
- Break history and break/work duration separation.
- Forgotten-stopwatch reminders with configurable thresholds.
- Optional break chime and notification placement.
- Focus sessions with optional visual timer, recovery context, end check-in,
  and body-doubling flow.

Primary sources: `crates/aevio-core/src/timers/`,
`crates/aevio-core/src/breaks/`, `apps/web/src/features/timers/`,
`apps/web/src/features/pulse/`, `apps/web/src/features/focus/`.

## 3. Calendar, timesheets, reports, and analytics

**Status: Implemented for local desktop data.**

- Calendar as a permanent core feature with day, week, and month views.
- Calendar blocks for captured and manually-created entries.
- Inline time-range editing and entry review from the calendar.
- Personal timesheet view.
- Team/project timesheet rollups.
- Entry list with filters, date ranges, grouping, sorting, and bulk selection.
- Rollups by project, category, tag, and time bucket.
- Total, approved, pending, and billable duration metrics.
- Productivity summary and dashboard widgets.
- Project activity statistics, monthly totals, billable totals, and unbilled
  entry counts.
- Finance dashboards with tracked time, planned time, revenue, cost, margin,
  warnings, and capacity-oriented views.
- Chart-based dashboard surfaces where the corresponding data source exists.

Primary sources: `apps/web/src/features/calendar/`,
`apps/web/src/features/timesheets/`, `apps/web/src/features/entries/`,
`apps/web/src/features/dashboard/`, `crates/aevio-core/src/reports/`,
`crates/aevio-services/src/finance.rs`.

## 4. Projects, clients, tasks, and work management

**Status: Implemented for personal and workspace-local work management.**

- Projects with active, completed, and archived states.
- Clients with contact details, default rates, notes, and archive/restore.
- Project descriptions, AI hints, due dates, budgets, budget periods, and
  billable defaults.
- Project-specific hourly rates and project statistics.
- Project discovery from captured activity, repository paths, editor titles,
  and Forge URLs.
- Project rules and hint previews for automatic classification.
- CSV project import with optional client creation.
- Tasks with descriptions, priorities, due dates, statuses, keywords, AI
  instructions, completion, and project links.
- Labels/tags with multiple labels per task.
- Issues, initiatives, and cycles.
- Task relationships and work-management queries.
- Documents, notes, comments, attachments, and mentions in workspace content.

Primary sources: `apps/web/src/features/projects/`,
`apps/web/src/features/tasks/`, `apps/web/src/features/issues/`,
`apps/web/src/features/initiatives/`, `apps/web/src/features/cycles/`,
`crates/aevio-core/src/projects/`, `crates/aevio-persistence/src/projects.rs`.

## 5. Finance, rates, and invoices

**Status: Implemented; external accounting integrations are not included here.**

- Integer-cent money representation in Rust.
- Client and project billing context.
- Project hourly rates and finance rate cards with bill and cost rates.
- Contracts and capacity plans.
- Revenue, cost, margin, billable-time, and unbilled-time calculations.
- Finance warnings and project profitability rows.
- Invoice drafts, open/finalized invoices, paid invoices, and void invoices.
- Time, retainer, and manual invoice lines.
- Selection of approved, billable, uninvoiced entries.
- Invoice numbering, issue dates, due dates, payment terms, notes, and payment
  instructions.
- Invoice profile with company information, defaults, and logo support.
- Rust-composed PDF invoice previews and finalized invoice PDFs.
- Invoice export to a user-selected file path.
- Dashboard sharing and revocation for supported finance views.

Primary sources: `crates/aevio-core/src/invoices/`,
`crates/aevio-services/src/finance.rs`,
`apps/web/src/features/invoices/`, `apps/web/src/features/finance/`.

## 6. Export, import, and data portability

**Status: Partial.**

- Time-entry export as CSV and JSON.
- CLI export of time entries as CSV or JSON.
- Project CSV import.
- Invoice PDF export.
- Local settings persistence in JSON.
- Server database backup and restore procedures are documented and have a
  restore rehearsal in the operations documentation.

Current limits:

- No user-facing full desktop database backup/restore workflow is documented
  as an in-app feature.
- No XLSX/XLS export implementation is present.
- Export columns and regional spreadsheet settings are not yet configurable at
  the Timesheet level.

Primary sources: `crates/aevio-core/src/reports/query.rs`,
`apps/cli/src/main.rs`, `crates/aevio-core/src/projects/service.rs`,
`docs/operations/BACKUP_RECOVERY.md`.

## 7. Coach, planning, energy, and productivity support

**Status: Implemented locally; AI-assisted paths are platform-dependent.**

- Date-specific daily goals, capacity, notes, energy, and planning state.
- Personal tasks and references to existing project tasks.
- Estimates, priority, ordering, completion, postponement, carryover, and
  explicit skip behavior.
- Editable task breakdown and Just Start guidance.
- Rules-based daily priority suggestions.
- Optional on-device AI proposals with explicit acceptance.
- Daily progress with task completion, goal assessment, work duration, and
  unassigned time.
- Five-state daily reviews with obstacles, reflection, and tomorrow planning.
- Focus-session integration with existing ActivityStore data.
- Coach reminders, routines, memory notes, brain dump, and weekly aggregates.
- Energy check-ins, energy history, and battery/energy monitoring.
- Focus music catalog with seeded tracks, silence mode, and local audio-file
  import.

Primary sources: `apps/web/src/features/coach/`,
`crates/aevio-core/src/coach/`, `crates/aevio-core/src/energy/`,
`docs/features/ai-coach/`.

## 8. Settings, localization, and updates

**Status: Implemented for the desktop application.**

- Light, dark, and system appearance modes with registered Aevio palettes.
- Date, time, duration, currency, distance-unit, and first-day-of-week
  preferences.
- German and English product translations with the shared i18n surface.
- Tracking hours, idle threshold, retention, expected-hours, break, reminder,
  tray, widget, and close-behavior settings.
- Category defaults and AI classification settings.
- Native notification placement and notification preferences.
- Start-at-login/login-item configuration.
- Update status and update-check workflows.
- Settings migration and validation for persisted JSON settings.

Primary sources: `apps/web/src/features/settings/`,
`apps/web/src/lib/settings.ts`, `apps/web/src/lib/i18n.ts`,
`crates/aevio-core/src/settings/`, `apps/desktop/tauri/src/updater.rs`.

## 9. AI, decision support, and automation

**Status: Partial/experimental depending on provider and platform.**

- Rules-only classification fallback.
- Optional Apple/on-device AI sidecar for classification and agent workflows.
- AI confidence calibration, learning metrics, retraining, and reset controls.
- Local agent chat with context from pending entries, projects, categories, and
  current app state.
- Typed decision engine with bounded use cases and provider status.
- Routines with cadence, scope, timezone, and delivery channels.
- Scheduled report definitions and saved dashboards.
- Automation run history and delivery metadata.
- Plugin workflows, jobs, triggers, and actions.

Current limits:

- Foundation-model capabilities depend on macOS/device eligibility.
- Windows and non-Apple paths use the documented fallback behavior.
- Timesheet-style Geofence, WLAN, and NFC timer automation is not present.

Primary sources: `apps/web/src/features/agent/`,
`crates/aevio-core/src/ai/`, `crates/aevio-services/src/decision/`,
`crates/aevio-services/src/automation.rs`.

## 10. Teams, governance, staffing, and permissions

**Status: Implemented locally; connected multi-user behavior is partially
verified.**

- Workspace, team, department, and member records.
- Team owner, manager, member, viewer, and workspace-admin concepts.
- Member invitations and team assignment data.
- Permission and finance-visibility fields.
- Workspace defaults and governance audit log.
- Role-based access checks for protected operations.
- Staffing directory, freelancer records, skills, availability, requests,
  assignments, contracts, pipeline entries, and matching.
- Capacity and resourcing metrics.
- Server workspace membership, authentication, authorization, and sync
  contracts.

Current limits:

- Full project-level sharing and salary visibility parity with Timesheet still
  need end-to-end product verification.
- Real PostgreSQL integration checks require the configured test database.

Primary sources: `apps/web/src/features/teams/`,
`apps/web/src/features/staffing/`, `crates/aevio-services/src/staffing.rs`,
`apps/api/src/app/views/`, `crates/aevio-contracts/src/sync.rs`.

## 11. Integrations and synchronization

**Status: Partial.**

- Integration connection records with provider, direction, workspace reference,
  enabled state, and last-sync metadata.
- Current integration providers represented in the product: ClickUp, Linear,
  and Asana.
- Timeline event publication and synchronization with device/workspace
  identity, cursor, outbox, and conflict counters.
- Optional server mode with local-first operation preserved.
- Local network watch pairing for iOS/watchOS timer control.
- Calendar event creation from timer/entry workflows.

Current limits:

- Full multi-device synchronization of all personal Aevio data is not yet a
  completed product contract.
- QuickBooks, Zapier, DATEV, BMD NTCS, and full Google Calendar two-way sync
  are not currently implemented as complete integrations.

Primary sources: `apps/web/src/features/integrations/`,
`apps/web/src/features/timeline/`, `crates/aevio-core/src/integrations/`,
`apps/api/src/app/views/sync.rs`, `apps/ios/`, `apps/watch/`.

## 12. Native desktop surfaces

**Status: Implemented on supported desktop targets; native visual verification
is separate from static/unit verification.**

- macOS menu-bar/tray surface.
- Windows system-tray path.
- Native OS notification plugin integration.
- Floating timer/coach widget window.
- Break reminder and pulse windows.
- Login-item/start-at-login support.
- Window open/close behavior and background tracking while minimized/hidden.
- Native permissions and platform capability checks.
- Updater status and update checks.
- Secure local persistence, WAL/locking, migrations, and compatibility checks.

Primary sources: `apps/desktop/tauri/src/lib.rs`,
`apps/desktop/tauri/src/tray.rs`, `apps/desktop/tauri/src/widget.rs`,
`apps/desktop/tauri/src/permissions/`, `crates/aevio-infrastructure/src/`.

## 13. iOS and watchOS companions

**Status: Experimental companion clients.**

- Local pairing with the desktop app.
- Timer list and current timer state.
- Start/pause timer actions.
- iOS focus-session start/stop actions.
- WatchOS timer control through the local network bridge.

Current limits:

- These clients are not yet full mobile equivalents of the desktop timesheet,
  project, invoice, export, or reporting workflows.
- Target-device and App Store-style verification is not part of the desktop
  verification flow.

Primary sources: `apps/ios/`, `apps/watch/`,
`apps/desktop/tauri/src/watch_bridge.rs`.

## 14. Plugin platform and extensibility

**Status: Implemented platform foundation; individual plugins vary.**

- Local plugin catalog and install state.
- Plugin package staging, manifest validation, compatibility checks, and
  artifact verification.
- Plugin lifecycle states including install, activation, disable, update,
  quarantine, and recovery.
- Capability-based permissions and explicit permission requests.
- Plugin navigation, pages, widgets, settings, commands, search, workflows,
  triggers, and scheduled jobs.
- Plugin storage, events, notifications, network access, and job execution
  through host-owned bridges.
- Registry catalog, update policies, release channels, and workspace desired
  state.
- Settings Plugin Store with Discover and Manage surfaces.
- Calendar remains a permanent core feature and is not installation-gated.

Primary sources: `packages/plugin-sdk/`, `crates/aevio-services/src/plugins*.rs`,
`apps/desktop/tauri/src/plugin_*.rs`, `apps/web/src/features/plugin-store/`.

## 15. CLI, MCP, privacy, and security boundaries

**Status: Implemented foundation; connected/server security requires deployment
verification.**

- `aevio` CLI for local/offline commands and RPC to a running desktop app.
- JSON output envelopes for scripting and automation.
- CLI time-entry, timer, project, settings, export, and plugin tooling.
- MCP server/tooling surface for supported local Aevio operations.
- Local SQLite ownership of durable desktop state.
- No keystroke logging, screenshots, telemetry, or mandatory cloud account for
  the standalone desktop workflow.
- Rust-owned persistence with typed React/Tauri contracts.
- Native capability boundaries for capture, notifications, widgets, and
  permissions.
- Explicit server authentication, workspace membership, roles, scopes, and
  audit records for connected operation.
- Secure plugin capability evaluation and transient registry credentials.
- Data export and account/data deletion boundaries in the server model.

## Verification boundary

This inventory was created from current source, route/type/API definitions,
feature documentation, and existing test/documentation references. It does not
claim that the following were run in this audit:

- Native Tauri/WebView visual checks.
- iOS/watchOS target-device checks.
- Full external integration checks.
- PostgreSQL integration tests without the configured test database.
- A clean-tree release build.

For the canonical local checks use `bun run fix`, `bun run verify`,
`cargo test --workspace`, and `node --test scripts/*.test.mjs` according to the
repository instructions.
