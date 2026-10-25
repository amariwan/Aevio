---
id: eng.ci-cd
title: CI/CD
category: engineering
domain: platform
status: implemented
source_paths: [.github/workflows/verify.yml, .github/workflows/release.yml, scripts/verify.sh]
related_docs: [docs/engineering/testing.md, docs/engineering/releases.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# CI/CD

- `verify.yml` runs `bun run verify` on macos-26 and windows-latest, triggered by `run-ci` label.
- macOS needs Xcode 26+ for Swift sidecar.
- `release.yml` builds signed macOS dmg/app on `v*` tags, publishes to GitHub Releases with latest.json updater manifest.
