---
id: project-brain.benchmarks
title: Project Brain Benchmarks
category: agents
domain: project-brain
status: experimental
source_paths: [scripts/project-brain.test.mjs, scripts/agent-system-eval.mjs, docs/agents/eval-dataset.json]
related_docs: [docs/project-brain/OPERATIONS.md, docs/agents/EVALUATION.md, docs/agents/CONTEXT_ENGINEERING.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: []
---

# Benchmarks

Run the reproducible baseline and brain checks:

```sh
node scripts/aevio-knowledge.mjs index
node scripts/aevio-knowledge.mjs eval
node scripts/agent-system-eval.mjs
node --test scripts/project-brain.test.mjs
```

Record recall/precision@k, missed dependencies, false positives, estimated tokens, latency, freshness, and task success using identical questions and acceptance criteria. The existing retrieval benchmark is a baseline; the facade benchmark is a retrieval/adapter benchmark, not proof of model-level task success. Do not claim the 70–80% target until repeated baseline-vs-brain runs include quality scoring and delegation overhead.
