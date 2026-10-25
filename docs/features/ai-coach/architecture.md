# AI Coach architecture and implementation plan

## Discovery and gaps (2026-10-20)

Aevio is a Bun/Next.js 16 React view over Tauri 2. Durable SQLite state and migrations live in `aevio-core` (`apps/api`), shared with the CLI. Desktop adapters live in `apps/desktop/tauri`. There is no HTTP backend or signed-in multi-tenant coach: local IPC and the operating-system data directory are the ownership boundary. Workspace members are collaboration metadata, not authenticated local principals. Do not introduce a second authentication system.

Existing reusable capabilities:

- `coach_tasks`: personal tasks, reminders and completion, already persisted by the desktop coach.
- `tasks`: project work items, referenced by `time_entries.task_id`; retain their identity and status.
- ActivityStore: capture, focus/meeting/break sessions, time entries, approval locks, tracking hours.
- Swift sidecar: private on-device classification; no existing general coaching operation or remote provider.
- Native notification plugin and 15-second coach scheduler; existing break policies and silent mode.
- Pulse and the timer widget: existing native windows; do not add another floating window.
- shadcn/Base UI, semantic Tailwind tokens, `useTauriEvent`, existing navigation route `coach`.
- Node test runner, Rust tests, canonical fix/verify scripts, isolated native app driver.

Gaps: no date-specific plans/goals, plan ordering/carryover, structured breakdown proposals, reviews/history, consent-based coach preferences/memory, or brain-dump workflow. Existing daily tracked time incorrectly treats local midnight as UTC. Classification availability must not be presented as working coaching AI.

## Decisions

1. Add a cohesive `aevio_core::coach` domain: typed contracts, validation/date boundaries, application services, policies and SQLite repositories. No SQL in IPC handlers or React.
2. Keep both existing task catalogs. Daily assignments reference exactly one personal or project task using foreign keys; never copy project titles/status into a separate task catalog. Personal tasks remain available without requiring a fictional project.
3. Apply migration v19 transactionally; preserve existing coach data and migrate legacy reminder metadata. Use `chrono-tz` for IANA civil-day boundaries including DST. A stored date is a calendar date, not a UTC timestamp. Recording durations never marks tasks or goals complete.
4. Commands return durable snapshots; changes emit invalidations and views refetch their selected day. Revision checks protect plan/review edits across windows.
5. Assistance is a read-only proposal. Rules work offline; optional on-device generation uses a bounded structured sidecar adapter. Only explicit acceptance writes steps. No activity titles/URLs leave the capture domain, no network providers, no autonomous tools.
6. Coach is opt-in. Native reminders obey explicit preferences, quiet hours, cooldowns and silent mode. Opening the page never requests OS permission.

## Ordered delivery and risks

1. Foundation: schema, plan/task/review/preferences contracts and repositories; migration, transaction, validation, carryover and DST tests.
2. Desktop/React: day selection, planning/skip, existing task selection, ordering/postponement, editable steps and Just Start, daily reviews and settings. Preserve existing navigation.
3. Focus integration: reuse ActivityStore, explicit time-entry assignment, recovery context, capacity comparison; native scheduler stays the only delivery boundary. Do not derive diagnoses or task completion from activity.
4. Adaptive: explicit/confirmed memory, aggregate weekly comparisons; computed evidence stays distinct from preferences and session context.
5. Advanced: brain dump and explicit conversion; voice/integrations only with an actual supported contract.
6. Validate fix → verify, real Rust/Node tests, native driver, graph update. Record exact unsupported or unverified boundaries in testing.md.

Central migration has CRITICAL graph impact (tracking/invoices/reports). Changes must be additive and covered by the existing full core regression suite. Cross-language IPC is not fully represented in GitNexus; source references and contract tests supplement graph results. The current checkout contains substantial unrelated work; preserve it. No branches, commits, PRs, hooks or CI are created.
