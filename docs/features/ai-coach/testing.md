# Verification

Verified locally on macOS on 2026-10-20. Version: 0.9.24. Existing unrelated working-tree changes were preserved; no branch, commit, or merge request was created.

## Automated checks — PASS

- `bun run fix`, followed by `bun run verify`: frontend lint/format/type checks and Rust format/Clippy checks passed. A Clippy rewrite required a second canonical fix/verify pass.
- `cargo test --workspace --no-fail-fast`: **322 passed, zero failed** (223 core, 76 desktop, 23 CLI). This includes **26 Coach tests**.
- `node --test scripts/*.test.mjs`: **19 passed, zero failed**, including three civil-date Coach tests.
- `git diff --check`: passed.

Coach coverage includes fresh and upgraded SQLite databases, persistence after reopening, 23/25-hour civil dates, reference integrity, ordering/carryover, transactional rollback, review revisions/idempotence, explicit goal outcomes, optional inference and memory, reminder consent/quiet hours/cooldowns, focus recovery, actual app transitions rather than title changes, brain-dump conversion, routine application, proposal validation, and mixed time-entry attribution without double counting. Existing tracking and CLI tests also pass.

The full suite initially exposed a legacy appearance-settings migration failure. The loader now translates legacy accent/theme values only when canonical palette/mode fields are missing. The original failing test and a new canonical-precedence regression test both pass.

## Native workflow evidence

The isolated real Tauri app was driven with `scripts/app-drive.sh`, using its own settings and SQLite directory. No production data was used.

Observed through native UI and database readback:

- Saved a daily goal and 180-minute capacity.
- Created an important personal task with a 30-minute estimate.
- Generated local steps without persisting them before acceptance.
- Requested actual on-device generation, observed its source label, and explicitly accepted five persisted steps. A direct Swift-sidecar request also returned structured German output with protocol version 2.
- Started, paused, resumed and ended focus with Body Doubling; recovery context and persisted interruptions were present.
- Explicit task completion appeared in Daily Review while the goal remained unassessed.
- Saved Coach/AI preferences and hid the existing widget through the native window boundary.

The native review outcome dropdown and free-text save interaction did not receive reliable end-to-end confirmation from the accessibility driver. Review persistence, repeated edits and revision conflicts are covered by Rust tests; the complete native review interaction remains **UNVERIFIED**. Brain Dump, routines, carryover, weekly insights and widget context have backend coverage but were not all driven through the native UI. Later focus-allocation/privacy/settings fixes passed the final automated checks; the earlier complete native workflow was not repeated for every subsequent change.

### Acceptance matrix

| Scenario | Classification | Evidence |
|---|---|---|
| Morning planning, goal persistence, personal task and edit | VERIFIED | Native session plus SQLite readback; repository tests cover reopen and edits |
| Focus start/pause/resume/end and recovery | VERIFIED | Native session plus persisted segments/interruptions; focus repository tests |
| On-device Coach generation and explicit step acceptance | VERIFIED | Native session and direct Swift-sidecar structured response |
| Daily Review create/save/edit/revision/conflict/reopen | BLOCKED | Rust tests pass; accessibility driver could not reliably focus/select/save the native controls |
| Brain Dump conversion | NOT TESTED | Repository tests only in this run |
| Routine repeated application | NOT TESTED | Repository idempotence tests only |
| Carryover | NOT TESTED | Repository carryover tests only |
| Weekly Insights and Memory | NOT TESTED | Repository tests only |
| Widget context after latest changes | NOT TESTED | Visibility was previously observed; latest context path was not driven |
| Privacy/settings deletion and opt-in behavior | NOT TESTED | Automated coverage only after the latest fixes |
| macOS notification permission and delivery | NOT TESTED | Scheduler policy tests only; no OS prompt/delivery evidence |

The accessibility failure is a tooling limitation observed at the WebKit accessibility boundary: the driver can enumerate controls and press buttons, but native `TextArea` focus and `PopUpButton` menu selection were not reliable. No confirmed application defect was isolated from that failure.

## Graph analysis

Graphify was refreshed: 6,636 nodes and 14,820 edges. SQL-parser support was unavailable, so SQL files are outside that parser's graph coverage; migrations and repository behavior have executable SQLite tests.

GitNexus was reindexed: 11,407 nodes and 25,652 edges. The final whole-working-tree change analysis reported **CRITICAL**, with 74 files, 541 symbols and 473 affected processes, including substantial pre-existing changes. Its output remains capped even with an increased limit. This is an **incomplete impact review**, not a clean graph verdict or an isolated Coach risk assessment. Pre-edit symbol impact analysis informed the implementation; no commit was made.

An isolated upstream query for `coach_day` resolved 47 callers with `CRITICAL` risk. A query for the stale/non-indexed `CoachStore` symbol returned `UNKNOWN`; that is unresolved index coverage, not evidence of no callers. No symbol was removed or renamed based on either result.

## Remaining verification boundaries

- Windows and Linux runtime behavior, notification delivery/permission prompts, and release bundles/signing were not tested here. Linux remains unsupported by the application; generation explicitly falls back outside supported macOS environments.
- Actual OS notification delivery, scheduled review offers and context-change hints were not manually triggered; scheduler policies have automated coverage.
- No external AI, calendar availability, voice or remote account integration is claimed. Rules remain usable without an AI provider.
- Native reminders require the running app/tray and OS permission. Closed-app scheduling is not implemented.

## Release recommendation

**CONDITIONAL GO** for automated and partially verified macOS functionality. **Native acceptance is not complete** until the blocked Daily Review interaction and the untested Brain Dump, routines, carryover, weekly, widget-context, privacy and notification scenarios are driven by a functioning native accessibility or manual macOS operator. No confirmed P0 regression is present in the automated evidence.

## Reproduction

Run the canonical fix/verify scripts, then the two test commands above. For remaining UI checks, launch an isolated instance with `scripts/app-drive.sh start`; select Coach, save/edit planning and review twice, reopen the selected date, and confirm SQLite persistence. Exercise Brain Dump conversion twice, apply one routine twice on the same date, carry an unfinished task forward, and inspect weekly insights and optional widget context. Finish with `scripts/app-drive.sh stop -s <session>` to remove the isolated data and processes.
