import test from "node:test";
import assert from "node:assert/strict";
import { classifyPath, parseJsonOutput } from "./project-brain.mjs";

test("classifies Aevio ownership boundaries deterministically", () => {
  assert.equal(classifyPath("apps/api/src/finance.rs"), "backend-persistence");
  assert.equal(classifyPath("apps/web/src/features/agent/Agent.tsx"), "frontend-native");
  assert.equal(classifyPath("docs/decisions/001-modular-monolith.md"), "docs-retrieval");
  assert.equal(classifyPath(".github/workflows/verify.yml"), "quality-operations");
});

test("parses adapter output with non-JSON prefixes", () => {
  assert.deepEqual(parseJsonOutput("notice\n{\"ok\":true}"), { ok: true });
  assert.deepEqual(parseJsonOutput("log\n[{\"path\":\"README.md\"}]"), [{ path: "README.md" }]);
});
