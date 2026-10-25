---
id: feat.activity-capture
title: Activity Capture
category: features
domain: activity
status: implemented
source_paths: [crates/aevio-persistence/src/activity/, crates/aevio-core/src/capture/, apps/desktop/tauri/src/capture.rs, apps/desktop/tauri/src/capture/]
related_docs: [docs/features/time-tracking.md, docs/architecture/rust-core.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Activity Capture

Background capture of foreground app, window title (Accessibility), browser URL (Safari/Chromium). Idle threshold configurable. Segments reviewed by user.

- `set_capture_enabled`, `set_idle_threshold`, `start_session`, `stop_session`, `mark_segment_reviewed`.
- Privacy: no keystrokes, no screenshots, no telemetry.
- Platform: macOS supported; Windows experimental; Linux unsupported (capture missing).
