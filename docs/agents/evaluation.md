---
id: agents.system-evaluation
title: Aevio Agent System Evaluation
category: agents
domain: agents
status: implemented
source_paths: [docs/agents/eval-dataset.json, scripts/aevio-knowledge.mjs, docs/agents/EVALUATION.md]
related_docs: [docs/agents/CONTEXT_ENGINEERING.md, docs/agents/ROUTING.md, docs/agents/MAINTENANCE.md]
platforms: [macos, windows]
verification_commit: working-tree
owner: Aevio engineering
symbols: [cmdEval]
---

# Evaluation

Run:

```sh
node scripts/aevio-knowledge.mjs index
node scripts/aevio-knowledge.mjs eval
node scripts/agent-system-eval.mjs
```

The existing ten-question dataset measures retrieval recall. The additional benchmark compares the full documentation corpus with the bounded context builder. It reports corpus/retrieved characters, estimated tokens, reduction, latency, and failures. This is a retrieval proxy, not proof of end-to-end model quality.

## Retrieval methodology

`docs/agents/eval-dataset.json` contains real Aevio architecture questions with expected canonical documents. Track precision@k, recall, source correctness, freshness (`verification_commit`), latency, and estimated context tokens. Compare direct file browsing, documentation FTS, and the hybrid FTS + Graphify + GitNexus path. The 70–80% token reduction objective remains experimental: correctness and essential context take precedence.

Representative acceptance checks:

- architecture investigation returns source-backed boundaries;
- feature planning identifies owning frontend/backend slices;
- bug investigation begins with impact and tests;
- review/security returns severity, confidence, and exact locations;
- documentation lookup returns the expected canonical document.

Quality is accepted only when evidence is exact, routing is deterministic, no secret is emitted, and verification output is real. Multi-agent runs are compared with a single-agent pass using the same task, context budget, acceptance criteria, and model where possible. Delegation overhead is recorded separately; parallelism is not counted as a win if it increases tokens without improving correctness or latency.
