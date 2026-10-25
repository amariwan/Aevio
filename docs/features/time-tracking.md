---
id: feat.time-tracking
title: Time Tracking & Entries
category: features
domain: entries
status: implemented
source_paths: [crates/aevio-core/src/entries/, crates/aevio-core/src/timers/, apps/web/src/features/entries/, apps/web/src/features/timers/]
related_docs: [docs/features/activity-capture.md, docs/features/finance.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Time Tracking & Entries

Activity segments fold into reviewable Time Entries. Timers support segments. Timesheets derive from entries.

- Commands: `list_timers`, `create_timer`, `start_timer`, `pause_timer`, `reset_timer`, `add_timer_segment`, ...
- Entries are the billing source for invoices.
