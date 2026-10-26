---
id: design.app-website-rules
title: App and Website Rules
category: design
domain: activity-capture
status: proposed
source_paths:
  - crates/aevio-core/src/capture
  - crates/aevio-persistence/src/activity
  - crates/aevio-persistence/src/bootstrap/migrations.rs
  - crates/aevio-persistence/src/database
  - apps/desktop/tauri/src/activity.rs
  - apps/desktop/tauri/src/commands.rs
  - apps/web/src/features/settings
  - apps/web/src/features/apps
related_docs:
  - docs/features/activity-capture.md
  - docs/DATABASE_COMPATIBILITY.md
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: []
---

# App and Website Rules

## Status

Proposed in chat on 2026-10-26. Implementation must not start until this design is approved.

## Intent

Add a production App & Website Rules surface under Settings matching the supplied reference:

- searchable list of observed apps, observed websites, and custom rules;
- source filter and per-row actions;
- create/edit dialog for activity, custom URL, or exact app-name rules;
- optional window-title keyword matching;
- Do Not Track, Detect Idle Time, block during Focus Sessions, and automatic
  None/Focus/Meeting session behavior.

The controls must change capture behavior immediately and persist locally. They are not decorative
preferences. Rust owns the rules and their effects; React renders snapshots and submits commands.

## Goals

- Match observed activity, a literal full/partial URL, or an exact app name.
- Apply optional case-insensitive title-keyword constraints.
- Stop recording matched activity when Do Not Track is enabled.
- Keep tracking input-idle time when Detect Idle Time is disabled.
- Trigger the native distraction warning for matched rules during a Focus Session.
- Derive Focus and Meeting sessions from matching activity without destroying raw app, title, or URL
  evidence.
- Support create, edit, delete, search, and source filtering.
- Preserve existing app default-category/default-project behavior and AI category/project rules.
- Work on macOS and Windows through the shared Rust/Tauri boundary. Linux remains honestly
  unsupported with no simulated frontend behavior.

## Non-goals

- Do not terminate, hide, or manipulate third-party application processes.
- Do not install a browser extension or inspect page content.
- Do not add screenshots, keystrokes, or message/document content.
- Do not merge these capture rules with the existing AI `rules` table, whose responsibility is
  category/project assignment.
- Do not remove the existing Apps & Websites usage/default-mapping page.
- Do not add Rize options that were not present in the supplied references, such as Always Block,
  block during Breaks/Meetings, full-URL retention, or window-title retention switches.

## Approaches considered

### 1. Dedicated capture-rule domain — selected

Create a separate durable rule table, typed domain model, repository, matcher, commands, events, and
Settings UI. Observed apps/sites remain catalog entries; an explicit rule overrides their defaults.

Trade-off: this is a cross-boundary feature and needs a migration, but it gives deterministic matching,
custom patterns, correct ownership, and testable capture behavior.

### 2. Extend the existing `apps` table

Add behavior columns to every observed app/site row and create synthetic rows for custom patterns.

Rejected: synthetic observations corrupt the activity catalog, title-keyword variants do not fit its
unique identifier contract, and app defaults become coupled to capture policy.

### 3. Reuse the AI `rules` table

Rejected: those rules assign categories/projects after capture. Do Not Track and idle handling must run
before persistence, and mixing the concerns would weaken both data contracts.

## Durable model

Migration 40 adds `app_website_rules`:

```text
id                    TEXT PRIMARY KEY
match_kind            TEXT NOT NULL  # activity | url | app_name
activity_id           TEXT           # apps.id for activity rules
pattern               TEXT           # URL fragment or exact app name
platform              TEXT           # macos | windows; app_name only
title_keyword          TEXT           # optional literal substring
do_not_track          INTEGER NOT NULL DEFAULT 0
detect_idle            INTEGER NOT NULL DEFAULT 1
block_during_focus     INTEGER NOT NULL DEFAULT 0
session_kind           TEXT NOT NULL DEFAULT 'focus' # none | focus | meeting
created_at             INTEGER NOT NULL
updated_at             INTEGER NOT NULL
deleted_at             INTEGER
```

Constraints enforce valid kinds, valid platform/session values, the required target for each match kind,
and `activity_id` foreign-key compatibility without cascading deletion. A partial unique index prevents
duplicate live rules for the same normalized target and title keyword. Deletion is soft so undo/audit
compatibility can be added without losing history.

The core domain type is serialized in camelCase for IPC. SeaORM-sync owns production CRUD and matching;
historical migration SQL remains isolated in the migration module.

## Defaults and rule projection

Every observed app/site is shown in the list even if no explicit row exists. The backend projects an
implicit default rule:

- Do Not Track: off
- Detect Idle Time: on
- Block during Focus Sessions: off
- Session: Focus

Saving an implicit rule creates an explicit row. Deleting an explicit rule restores defaults. Custom URL
and app-name rules exist only as explicit rows.

The list response includes a stable target key, display name, source (`macos`, `windows`, or `website`),
whether the rule is explicit, and the editable settings. The frontend does not merge database rows.

## Matching and precedence

The matcher receives the existing `WindowSample` plus the current platform. It performs literal,
case-insensitive matching after Unicode-aware trimming/case folding:

1. rules with a title keyword before rules without one;
2. custom URL substring, longest pattern first;
3. observed website/domain rule;
4. observed app rule;
5. exact app-name rule for the current platform;
6. stable `id` ordering as the final tie-breaker.

An activity website rule matches the captured domain/identifier. An activity app rule matches its bundle
identifier first and display name only as the compatibility fallback already used by exclusions. A custom
URL rule matches the captured URL, not the window title. Title keywords are optional literal substrings;
regex syntax is deliberately unsupported.

One winning rule supplies the complete behavior. Rules are validated and normalized in Rust on every
write; invalid combinations never reach persistence.

## Capture behavior

### Do Not Track

Evaluation happens before a sample can open or extend a segment. A matching Do Not Track rule clears a
pending switch and closes current activity immediately, preserving the existing exclusion privacy
contract. Updating a rule against the current sample takes effect on the next one-second sampler tick.

Migration 40 backfills every live `apps.excluded = 1` row as an explicit activity rule with
`do_not_track = 1`, preserving the user's current privacy choice. After migration, an explicit capture
rule is authoritative; `apps.excluded` is consulted only when no explicit rule exists. Saving an activity
rule synchronizes `apps.excluded` in the same transaction, so turning Do Not Track off can never leave a
hidden legacy exclusion behind. The existing Apps page continues using `update_app`; exclusion changes
there create or update the linked activity rule transactionally. Custom rules never write `apps` rows.

### Detect Idle Time

When the winning rule has `detect_idle = false`, input-idle duration does not end that matched activity.
System away, screen lock, sleep, lid close, capture pause, and tracking-hours boundaries still close it;
an app rule cannot manufacture time while the computer is unavailable.

### Distraction blocker

While a live Focus Session exists, a matching `block_during_focus` rule creates a blocker effect. The
desktop adapter delivers it through the shared native notification plugin on macOS and Windows. Effects
are rate-limited per rule/foreground activation so the one-second sampler cannot spam notifications; a
rule can notify again only after the user leaves and returns, or after a bounded cooldown.

The blocker warns and returns focus guidance; it does not kill or control another process. Notification
failure is logged and does not stop capture. Linux returns an explicit unsupported capability.

## Automatic sessions

Automatic sessions are overlays on raw activity, not replacements for activity segments. Migration 40
also adds `activity_sessions` with `id`, `kind`, `origin = 'rule'`, `rule_id`, `started_at`, `ended_at`, and
timestamps. This prevents focus/meeting detection from deleting app, title, URL, and project evidence.
Manual Focus/Meeting sessions remain exclusive `segments` and retain the existing `start_session` /
`stop_session` behavior. This is a compatibility boundary for the current Focus UI, Break engine, Watch
bridge, and CLI. The new table stores automatic rule-derived overlays only.

- Meeting: 15 continuous minutes governed by the same winning rule with `session_kind = meeting`
  creates/extends an automatic Meeting session for that interval. A switch to another rule, even another
  Meeting rule, closes the candidate/overlay and starts a new candidate.
- Focus: a rolling window of at least 15 minutes creates a Focus session when at least 75% of tracked,
  non-meeting activity is governed by `session_kind = focus`. The session covers the qualifying window
  and extends while the threshold remains satisfied.
- Meeting wins over Focus for overlapping time.
- Manual sessions win over automatic sessions and retain their current user-controlled lifecycle. While
  a manual Focus segment is live, the sampler still evaluates the foreground sample for blocker effects
  but does not persist underlying activity, matching current manual-session capture behavior.
- Idle and Do Not Track time never count toward either threshold.

Session aggregation uses interval union so overlapping manual/automatic evidence is never double-counted.
`ActivitySnapshot` gains session overlays and derives `focusMs` from the union. Existing segment and time
entry APIs remain backward compatible.

Rule mutations affect samples and automatic-session detection from the next sampler tick forward. They do
not rewrite historical activity or already materialized automatic sessions. This keeps reviewed history
stable and avoids surprising retroactive metric changes.

## IPC and events

Add Tauri commands:

- `list_app_website_rules`
- `get_app_website_rule`
- `create_app_website_rule`
- `update_app_website_rule`
- `delete_app_website_rule`

Mutations return the authoritative saved model. They emit `app-website-rules-changed`; the webview
subscribes with `useTauriEvent`. Capture reloads matcher state atomically after each mutation so the next
tick uses the new rules. The CLI command surface and `apps/cli/COMMANDS.md` receive list/create/edit/delete
support because rules are durable core state.

## Settings UI

The existing Settings navigation entry becomes a real `appWebsiteRules` section instead of routing to the
Apps page. UI is composed only from installed shadcn components and semantic theme tokens.

The section contains:

- heading `App & Website Rules`;
- search input, source Select, and `New rule` Button;
- responsive Table of target and source with row DropdownMenu;
- empty, loading, and inline error states;
- create/edit Dialog using RadioGroup, Combobox/Select, Input, Switch, and footer Buttons;
- conditional fields for From Your Activity, Custom URL, and App Name;
- Do Not Track hides the remaining behavior controls;
- Meeting and Focus are mutually exclusive through a single `sessionKind` RadioGroup;
- Save is disabled until the Rust-validatable target is present.

The dialog is keyboard navigable, labelled, focus-trapped, responsive, and reduced-motion safe. German
and English strings are added through the existing i18n catalog. No raw colors, inline color styles, or
new custom UI primitives are introduced.

## Error handling and concurrency

- Validation errors identify the invalid field without echoing sensitive titles or URLs into logs.
- Repository writes are transactional and update timestamps atomically.
- A stale edit carries `updatedAt`; mismatches return a conflict instead of overwriting a newer rule.
- List/read failures keep the last successful snapshot and display a retry action.
- Before a mutation commits, Rust validates the input and compiles the complete prospective matcher from
  the rows visible inside the transaction. The transaction commits only after compilation succeeds; the
  precompiled immutable snapshot is then swapped atomically through an infallible in-process operation.
  Event-delivery failure is logged after commit but does not make a durable successful write appear to
  fail. Lock ordering is fixed as database transaction before matcher swap; sampler reads never acquire a
  database write lock.
- Deleting a rule referenced by historical automatic sessions retains the nullable historical reference.

## Compatibility and migration

- Migration opens every existing database in place, increments `user_version` from 39 to 40, and never
  performs schema sync or reset.
- Existing `apps.excluded` rows are backfilled to explicit rules and remain effective; linked writes keep
  both representations synchronized for backward compatibility.
- Existing AI category/project rules and project rules are unchanged.
- Existing manual session commands remain callable and serialized shapes are extended only with optional
  fields where needed.
- Database compatibility docs and command references are updated in the same change.
- The patch version is bumped once across all required manifests.

## Test strategy

Implementation follows red-green-refactor.

1. Migration tests: fresh DB, v39 upgrade, constraints/indexes, preserved existing data.
2. Repository tests: projection defaults, CRUD, normalization, optimistic conflict, soft delete.
3. Matcher tests: all match kinds, title keyword, case folding, precedence, legacy exclusion fallback.
4. Activity-store tests: immediate Do Not Track, idle exemption but not away/sleep exemption, live reload.
5. Session tests: 15-minute Meeting, 75% Focus window, precedence, closure, interval union/no double count.
6. Desktop tests: blocker effect cooldown and notification failure isolation.
7. Frontend contract tests: command names/payloads, settings route, validation and conditional controls.
8. Canonical `bun run fix`, focused Rust/Node tests, `bun run verify`, `just test`, GitNexus change
   analysis, Graphify update, and macOS app-drive visual verification.

## Rollback

The UI and commands can be removed without changing existing activity rows. Migration 40 is forward-only;
rollback leaves the new tables unused rather than lowering `user_version` or deleting user rules. Legacy
`apps.excluded` continues protecting existing exclusions throughout.

## Builder discretion

- Exact component filenames within `apps/web/src/features/settings/rules/`.
- Exact internal repository/matcher helper names.
- Whether rule snapshot caching uses an immutable `Arc` or a lock-protected owned value, provided reads are
  deterministic and the previous valid snapshot survives reload failure.
- Exact blocker cooldown between 30 and 60 seconds, with leave-and-return resetting it.
