---
id: arch.ai-sidecar
title: On-Device AI Sidecar
category: architecture
domain: ai
status: implemented
source_paths: [apps/desktop/tauri/src/ai/, apps/desktop/tauri/swift/, apps/desktop/tauri/build.rs]
related_docs: [docs/features/coach.md, docs/features/decision-engine.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# On-Device AI Sidecar

Swift sidecar compiled by `build.rs` with `swift build`, shipped as a Tauri `externalBin`.

## Wire protocol

`protocolVersion` in `Entry.swift` and `PROTOCOL_VERSION` in `ai/sidecar.rs` must be bumped together when the wire format changes.

## ML tiers

- macOS Apple Silicon: T0 + T1 + T2 (Foundation Models, NaturalLanguage, Create ML).
- macOS Intel: T0 + T1 (no Foundation Model).
- Windows: T0 (rules only).

## Fallback

Without Xcode 26+ command line tools, `build.rs` stages an empty placeholder; app runs with rules-only fallback. No build failure.

## Rules

Weak-linked; never require AI availability. Rules fallback must always work.
