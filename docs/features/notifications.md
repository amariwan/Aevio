---
id: feat.notifications
title: Notifications
category: features
domain: notifications
status: implemented
source_paths: [apps/desktop/tauri/src/tray.rs, crates/aevio-core/src/settings/]
related_docs: [docs/platforms/macos.md]
platforms: [macos, windows, linux]
verification_commit: working-tree
---

# Notifications

User-visible reminders must use native OS notifications on every supported desktop platform via shared Rust/Tauri abstraction. Never in-app toast only. Delivery while app closed is not implemented.
