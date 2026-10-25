# Aevio Productivity Coach

The Coach extends existing personal tasks, project tasks and ActivityStore tracking. It is a private desktop feature with local SQLite persistence, an optional on-device model, and explicit user actions for every plan/task change.

Available workflows:

- Editable date-specific goals, capacity, notes, optional energy and planning/skip state.
- Existing personal/project task references, new personal tasks, estimates, priority, order, completion, postponement and explicit carryover.
- Editable/reviewable task breakdown and Just Start for six starting difficulties. Rules work offline; on-device proposals are optional and labeled.
- Rules or on-device daily priority suggestions constrained to existing task identities and available time. Acceptance changes ordering only; postponement stays explicit.
- Daily progress with separate task completion, goal assessment, work duration and unassigned time. Editable five-state reviews, with a configurable native review offer.
- Focus sessions through the existing ActivityStore, optional visual timer, recovery context, gentle end check-in and quiet Body Doubling. The existing floating widget can show goal/task/next-step/progress and pause/resume; its visibility and context are configurable.
- Opt-in hints after frequent recent context changes. They describe an observation, not motivation or a diagnosis.
- Weekly local aggregates with the previous week, explicit/confirmed editable memory notes, optional duration estimates from repeated exact task identities, and user-confirmed insights.
- Brain Dump, reviewable local category suggestions, explicit idempotent conversion to a personal task, and editable routine templates applied at most once per day on explicit request.
- Coach enablement, style, optional on-device AI, native reminders, quiet hours, cooldowns, timer/widget options and full coach-data deletion.

## Supported boundaries

The local desktop data directory is the ownership boundary. There is no authenticated remote Coach service. No external AI provider, calendar availability feed, voice transcription or new third-party integration has been fabricated. Existing work-management views stay available. User-visible reminders require the running app/tray and OS permission; delivery while the app is closed is not implemented.

On-device generation uses macOS Foundation Models when available. Windows and Linux adapters explicitly return an unsupported result with an editable rules fallback. Native notification delivery uses the existing cross-platform plugin. Cross-platform source design does not constitute a Windows/Linux runtime test.

Brain-dump categorization is currently a transparent local rule suggestion, not generative AI. Capacity uses the user's available working minutes after appointments and breaks. Estimates are simple historical evidence, not a predictive model; memory is never inferred psychology. Separate review and goal statuses are deliberate.

See [architecture](architecture.md), [data model](data-model.md), [workflows](workflows.md), [AI orchestration](ai-orchestration.md), [privacy](privacy.md), and [verification](testing.md).
