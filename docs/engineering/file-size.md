---
id: eng.file-size
title: File Size & Complexity Limits
category: engineering
domain: platform
status: implemented
source_paths: [AGENTS.md]
related_docs: [docs/engineering/rules.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# File Size & Complexity Limits

- Production files: max 500 physical lines; target 100–300; >400 is an architecture warning.
- Functions/methods: target max 40 lines; low cyclomatic/cognitive complexity.
- React presentation components: max 150 lines; no business logic in JSX.
- Exceptions only for generated code, lockfiles, snapshots, declarative data, migrations — verifiable, explicit, narrowly scoped.
- Never bypass via minification or artificial splitting.
