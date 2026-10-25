#!/usr/bin/env node
// Local-first facade over the repository's existing intelligence indexes.
// It deliberately owns orchestration and provenance, not a second graph.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const ROOT = resolve(process.cwd());
const CACHE_DIR = join(ROOT, ".project-brain", "cache");
const MAX_CACHE_AGE_MS = 5 * 60 * 1000;

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: ROOT,
    encoding: "utf8",
    maxBuffer: 2 * 1024 * 1024,
    timeout: options.timeout ?? 20_000,
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function optional(command, args, options = {}) {
  try {
    return { ok: true, value: run(command, args, options) };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : String(error) };
  }
}

export function parseJsonOutput(text) {
  const start = text.search(/[\[{]/);
  if (start < 0) throw new Error("adapter returned no JSON");
  const end = Math.max(text.lastIndexOf("}"), text.lastIndexOf("]"));
  if (end < start) throw new Error("adapter returned incomplete JSON");
  return JSON.parse(text.slice(start, end + 1));
}

export function classifyPath(path) {
  if (/^(apps\/api|apps\/desktop\/tauri|apps\/cli)\//.test(path)) return "backend-persistence";
  if (/^(apps\/web|packages\/(web|ui|types|api-client))\//.test(path)) return "frontend-native";
  if (/^(docs|AGENTS\.md|\.agents|\.codex)\//.test(path) || path === "AGENTS.md") return "docs-retrieval";
  if (/^(\.github|scripts|Cargo\.toml|package\.json|pnpm-workspace\.yaml)/.test(path)) return "quality-operations";
  return "architecture";
}

function gitState() {
  const branch = run("git", ["branch", "--show-current"]).trim() || "detached";
  const head = run("git", ["rev-parse", "HEAD"]).trim();
  const status = run("git", ["status", "--short", "--untracked-files=all"]);
  const fingerprint = createHash("sha256").update(`${branch}\n${head}\n${status}`).digest("hex");
  const indexPaths = [".aevio-knowledge.db", ".projectatlas/projectatlas.db", "graphify-out/manifest.json", ".gitnexus/meta.json"];
  const indexFingerprint = indexPaths.map(path => {
    try { return `${path}:${statSync(join(ROOT, path)).mtimeMs}`; }
    catch { return `${path}:missing`; }
  }).join("|");
  return { branch, head, dirty: status.length > 0, statusFingerprint: fingerprint, indexFingerprint, changedFiles: status.split("\n").filter(Boolean).map(line => line.slice(3)) };
}

function cacheKey(operation, input, state) {
  return createHash("sha256").update(JSON.stringify({ operation, input, branch: state.branch, head: state.head, dirty: state.statusFingerprint, indexes: state.indexFingerprint })).digest("hex");
}

function cached(operation, input, producer) {
  const state = gitState();
  const file = join(CACHE_DIR, `${cacheKey(operation, input, state)}.json`);
  if (existsSync(file)) {
    try {
      const record = JSON.parse(readFileSync(file, "utf8"));
      if (Date.now() - record.createdAt < MAX_CACHE_AGE_MS) return { ...record.result, cache: "hit" };
    } catch { /* stale or partial cache: recompute */ }
  }
  const result = producer(state);
  mkdirSync(CACHE_DIR, { recursive: true });
  writeFileSync(file, JSON.stringify({ createdAt: Date.now(), result }));
  return { ...result, cache: "miss" };
}

function source(path, adapter, confidence, detail) {
  return { path, adapter, confidence, detail };
}

function adapterJson(command, args) {
  const result = optional(command, args);
  if (!result.ok) return { available: false, error: result.error };
  try { return { available: true, data: parseJsonOutput(result.value) }; }
  catch (error) { return { available: false, error: error instanceof Error ? error.message : String(error) }; }
}

function docsSearch(query, limit) {
  const result = adapterJson("node", ["scripts/aevio-knowledge.mjs", "search", query]);
  if (!result.available) return result;
  return { available: true, data: result.data.slice(0, limit).map(row => ({ ...row, source: source(row.path, "aevio-knowledge", "high", "documentation FTS") })) };
}

function projectSearch(query, limit) {
  return adapterJson("projectatlas", ["--format", "json", "search", query, "--limit", String(limit)]);
}

function gitnexus(command, args) {
  return adapterJson("gitnexus", [command, ...args, "--repo", ROOT]);
}

function freshness(state) {
  const status = optional("gitnexus", ["status", "--repo", ROOT]);
  const indexedCommit = status.ok ? status.value.match(/Indexed commit:\s+([^\s]+)/)?.[1] ?? null : null;
  return {
    branch: state.branch,
    head: state.head,
    dirty: state.dirty,
    gitnexus: { available: status.ok, indexedCommit, freshAtHead: indexedCommit === state.head.slice(0, indexedCommit?.length ?? 0), output: status.ok ? undefined : status.error },
    graphify: existsSync(join(ROOT, "graphify-out", "graph.json")) ? "available" : "missing",
    projectAtlas: existsSync(join(ROOT, ".projectatlas", "projectatlas.db")) ? "available" : "missing",
  };
}

export function architectureRules() {
  return cached("architecture_rules", {}, state => ({ freshness: freshness(state), sources: [source("AGENTS.md", "repository", "verified", "project invariants"), source("docs/engineering/rules.md", "repository", "high", "engineering rules"), source("docs/agents/source-priority.md", "repository", "high", "source precedence")] }));
}

export function search(query, limit = 20) {
  return cached("search", { query, limit }, state => {
    const docs = docsSearch(query, limit);
    const project = projectSearch(query, limit);
    return { query, freshness: freshness(state), results: { docs: docs.available ? docs.data : [], projectAtlas: project.available ? project.data.results ?? project.data : [], unavailable: [docs, project].filter(result => !result.available).map(result => result.error) }, provenance: [source("scripts/aevio-knowledge.mjs", "aevio-knowledge", "high", "docs/ADR retrieval"), source("ProjectAtlas SQLite index", "projectatlas", "high", "repository text search")] };
  });
}

export function symbolLookup(name, file) {
  return cached("symbol_lookup", { name, file }, state => {
    const graph = gitnexus("context", [name, ...(file ? ["--file", file] : []), "--limit", "50"]);
    const atlas = adapterJson("projectatlas", ["--format", "json", "symbols", "list", "--query", name, "--limit", "50"]);
    return { symbol: name, file: file ?? null, freshness: freshness(state), graph: graph.available ? graph.data : null, projectAtlas: atlas.available ? atlas.data : null, provenance: [source(file ?? name, "GitNexus", graph.available ? "exact" : "unavailable", "symbol context"), source("ProjectAtlas symbol graph", "projectatlas", atlas.available ? "high" : "unavailable", "symbol lookup")] };
  });
}

export function dependencyTrace(name, direction = "outbound", file) {
  return cached("dependency_trace", { name, direction, file }, state => {
    const graph = gitnexus("context", [name, ...(file ? ["--file", file] : []), "--limit", "100"]);
    const atlas = adapterJson("projectatlas", ["--format", "json", "symbols", "relations", "--symbol", name, "--direction", direction, "--depth", "3", "--limit", "100"]);
    return { symbol: name, direction, freshness: freshness(state), relationships: graph.available ? graph.data : null, staticRelations: atlas.available ? atlas.data : null, confidence: graph.available && freshness(state).gitnexus.freshAtHead ? "high" : "medium", unresolved: graph.available ? [] : ["GitNexus unavailable or stale"], provenance: [source(name, "GitNexus", "exact", "callers/callees/processes"), source("ProjectAtlas symbol relations", "projectatlas", "high", "static relation traversal")] };
  });
}

function impactMetadata(files) {
  const roles = [...new Set(files.map(classifyPath))];
  return {
    owners: roles,
    databaseMigrationImpact: files.some(file => /(^|\/)(migrations?|schema|database|DATABASE_COMPATIBILITY)/i.test(file)),
    securitySensitive: files.some(file => /(auth|permission|secret|crypto|rpc|ipc|mcp|credential)/i.test(file)),
    boundaryReviewRequired: roles.length > 1,
    affectedTests: files.filter(file => /(test|spec|tests?\/)/i.test(file)),
  };
}

export function impact(target, options = {}) {
  const scope = options.scope ?? (target ? "symbol" : "all");
  return cached("impact", { target, scope, file: options.file, baseRef: options.baseRef }, state => {
    const graph = target ? gitnexus("impact", [target, "--direction", "upstream", "--include-tests", "--summary-only", ...(options.file ? ["--file", options.file] : [])]) : gitnexus("detect-changes", ["--scope", scope, ...(options.baseRef ? ["--base-ref", options.baseRef] : [])]);
    const files = target ? (options.file ? [options.file] : []) : state.changedFiles;
    const metadata = impactMetadata(files);
    const unresolved = [];
    const freshnessState = freshness(state);
    if (!freshnessState.gitnexus.freshAtHead) unresolved.push("GitNexus index is not at current HEAD; refresh before trusting zero-edge results");
    if (!target && files.length === 0) unresolved.push("No changed files were found in the selected diff scope");
    if (graph.available && graph.data.risk === "UNKNOWN") unresolved.push(graph.data.riskNote ?? "GitNexus reported UNKNOWN risk");
    return { target: target ?? null, scope, freshness: freshnessState, graph: graph.available ? graph.data : null, metadata, confidence: unresolved.length === 0 ? "high" : "medium", unresolved, provenance: [source("GitNexus impact/detect-changes", "GitNexus", graph.available ? "exact" : "unavailable", "callgraph/VCS analysis"), source("AGENTS.md", "repository", "verified", "boundary and safety rules")] };
  });
}

export function relevantContext(task, budget = 4000) {
  return cached("relevant_context", { task, budget }, state => {
    const result = adapterJson("node", ["scripts/aevio-knowledge.mjs", "context", task, `--budget=${budget}`]);
    const graph = adapterJson("gitnexus", ["query", task, "--repo", ROOT]);
    return { task, budget, freshness: freshness(state), context: result.available ? result.data : null, graph: graph.available ? graph.data : null, provenance: [source("scripts/aevio-knowledge.mjs", "aevio-knowledge", "high", "bounded task context"), source("graphify-out/graph.json", "Graphify", graph.available ? "medium" : "unavailable", "architecture relationships")], unresolved: [result, graph].filter(item => !item.available).map(item => item.error) };
  });
}

export function decisionHistory(query = "architecture decision ADR") {
  return cached("decision_history", { query }, state => {
    const docs = docsSearch(query, 30);
    const commits = optional("git", ["log", "--all", "--format=%h %ad %s", "--date=short", "--", "docs/decisions", "docs/superpowers"]);
    return { query, freshness: freshness(state), documents: docs.available ? docs.data.filter(row => /(^|\/)(decisions|superpowers)\//.test(row.path)) : [], commits: commits.ok ? commits.value.split("\n").filter(Boolean).slice(0, 30) : [], provenance: [source("docs/decisions/", "repository", "high", "ADR documents"), source("git history", "git", "exact", "historical decision commits")] };
  });
}

export function affectedTests(target) {
  const report = impact(target, { scope: "all" });
  const tests = report.metadata.affectedTests;
  return { target, tests, impact: report.graph, freshness: report.freshness, confidence: report.confidence, unresolved: report.unresolved, provenance: report.provenance };
}

export function update() {
  const results = {
    docs: optional("node", ["scripts/aevio-knowledge.mjs", "index"], { timeout: 60_000 }),
    graphify: optional("graphify", ["update", "."], { timeout: 120_000 }),
    // ProjectAtlas follows Graphify so its persisted text/symbol index ends
    // fresh even though the repository keeps derived graph artifacts visible.
    projectAtlas: optional("projectatlas", ["--format", "json", "scan"], { timeout: 120_000 }),
  };
  return { freshness: freshness(gitState()), results: Object.fromEntries(Object.entries(results).map(([key, value]) => [key, { ok: value.ok, output: value.ok ? value.value.slice(-2000) : value.error }])) };
}

const TOOLS = {
  project_search: args => search(args.query, args.limit),
  symbol_lookup: args => symbolLookup(args.name, args.file),
  dependency_trace: args => dependencyTrace(args.name, args.direction, args.file),
  impact_analysis: args => impact(args.target, args),
  relevant_context: args => relevantContext(args.task, args.budget),
  architecture_rules: () => architectureRules(),
  decision_history: args => decisionHistory(args.query),
  affected_tests: args => affectedTests(args.target),
  brain_status: () => ({ freshness: freshness(gitState()), adapters: ["aevio-knowledge", "ProjectAtlas", "GitNexus", "Graphify"] }),
};

function toolDefinitions() {
  return Object.keys(TOOLS).map(name => ({ name, description: `Read-only Aevio Project Brain capability: ${name}. Results include provenance and freshness.`, inputSchema: { type: "object", additionalProperties: true } }));
}

function handleMcp(request) {
  const id = request.id;
  const method = request.method;
  if (method === "notifications/initialized") return null;
  if (method === "initialize") return { jsonrpc: "2.0", id, result: { protocolVersion: "2025-11-25", capabilities: { tools: { listChanged: false } }, serverInfo: { name: "aevio-project-brain", version: "1.0.0" }, instructions: "Read-only local-first project knowledge and impact reports. Treat freshness and unresolved fields as authoritative warnings." } };
  if (method === "ping") return { jsonrpc: "2.0", id, result: {} };
  if (method === "tools/list") return { jsonrpc: "2.0", id, result: { tools: toolDefinitions() } };
  if (method === "tools/call") {
    const name = request.params?.name;
    const fn = TOOLS[name];
    if (!fn) return { jsonrpc: "2.0", id, error: { code: -32602, message: `unknown Project Brain tool: ${name}` } };
    try { return { jsonrpc: "2.0", id, result: { content: [{ type: "text", text: JSON.stringify(fn(request.params?.arguments ?? {}), null, 2) }] } }; }
    catch (error) { return { jsonrpc: "2.0", id, error: { code: -32000, message: error instanceof Error ? error.message : String(error) } }; }
  }
  return { jsonrpc: "2.0", id, error: { code: -32601, message: `method not found: ${method}` } };
}

function serveMcp() {
  process.stdin.setEncoding("utf8");
  let buffer = "";
  process.stdin.on("data", chunk => {
    buffer += chunk;
    const lines = buffer.split("\n");
    buffer = lines.pop() ?? "";
    for (const line of lines) {
      if (!line.trim()) continue;
      const response = handleMcp(JSON.parse(line));
      if (response) process.stdout.write(`${JSON.stringify(response)}\n`);
    }
  });
}

function main(argv) {
  const command = argv[0];
  if (command === "mcp") return serveMcp();
  if (command === "status") return console.log(JSON.stringify({ freshness: freshness(gitState()), adapters: Object.keys(TOOLS) }, null, 2));
  if (command === "update") return console.log(JSON.stringify(update(), null, 2));
  if (command === "search") return console.log(JSON.stringify(search(argv.slice(1).join(" ")), null, 2));
  if (command === "symbol") return console.log(JSON.stringify(symbolLookup(argv[1], argv[2]), null, 2));
  if (command === "dependencies") return console.log(JSON.stringify(dependencyTrace(argv[1], argv[2] ?? "outbound", argv[3]), null, 2));
  if (command === "impact") return console.log(JSON.stringify(impact(argv[1]), null, 2));
  if (command === "context") return console.log(JSON.stringify(relevantContext(argv.slice(1).join(" ")), null, 2));
  if (command === "rules") return console.log(JSON.stringify(architectureRules(), null, 2));
  if (command === "decisions") return console.log(JSON.stringify(decisionHistory(argv.slice(1).join(" ") || undefined), null, 2));
  if (command === "tests") return console.log(JSON.stringify(affectedTests(argv[1]), null, 2));
  console.error("Usage: project-brain.mjs {status|update|search|symbol|dependencies|impact|context|rules|decisions|tests|mcp}");
  process.exitCode = 2;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main(process.argv.slice(2));
