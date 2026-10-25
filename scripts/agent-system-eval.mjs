#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const root = process.cwd();
const dataset = JSON.parse(readFileSync(join(root, "docs/agents/eval-dataset.json"), "utf8"));

function markdownFiles(dir) {
  const files = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...markdownFiles(path));
    else if (entry.name.endsWith(".md")) files.push(path);
  }
  return files;
}

const corpus = markdownFiles(join(root, "docs"));
const baselineChars = corpus.reduce((sum, path) => sum + statSync(path).size, 0);
const started = performance.now();
const results = dataset.questions.map((question) => {
  const raw = execFileSync("node", ["scripts/aevio-knowledge.mjs", "context", question.query, "--budget=4000"], { encoding: "utf8" });
  const bundle = JSON.parse(raw);
  const chars = bundle.context.length;
  return { id: question.id, selected: bundle.selected, chars, estimated_tokens: Math.ceil(chars / 4) };
});
const latencyMs = Math.round(performance.now() - started);
const selectedChars = results.reduce((sum, result) => sum + result.chars, 0);
const reduction = baselineChars === 0 ? 0 : 1 - selectedChars / (baselineChars * results.length);
console.log(JSON.stringify({
  dataset: results.length,
  baseline: { corpus_files: corpus.length, chars_per_question: baselineChars, estimated_tokens_per_question: Math.ceil(baselineChars / 4) },
  bounded_context: { chars_total: selectedChars, estimated_tokens_total: Math.ceil(selectedChars / 4), reduction_percent: Number((reduction * 100).toFixed(1)), latency_ms_total: latencyMs },
  results,
  note: "Retrieval proxy only; not an end-to-end model or single-agent quality benchmark."
}, null, 2));
