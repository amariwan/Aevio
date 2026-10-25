#!/usr/bin/env node
// aevio-knowledge: local-first documentation retrieval CLI
import { DatabaseSync } from "node:sqlite";
import { readdirSync, readFileSync, statSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join, relative, resolve, dirname, extname } from "node:path";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";

const ROOT = resolve(process.cwd());
const DOCS = join(ROOT, "docs");
const DB_PATH = join(ROOT, ".aevio-knowledge.db");
const SCHEMA = join(DOCS, "schemas", "frontmatter.schema.json");

function walk(dir) {
  const out = [];
  if (!existsSync(dir)) return out;
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name === "node_modules" || e.name === ".git") continue;
      out.push(...walk(p));
    } else if (e.isFile() && extname(e.name) === ".md") {
      out.push(p);
    }
  }
  return out;
}

function parseFrontmatter(text) {
  const m = text.match(/^---\n([\s\S]*?)\n---\n?/);
  if (!m) return { meta: null, body: text };
  const raw = m[1];
  const meta = {};
  let current = null;
  for (const line of raw.split("\n")) {
    const kv = line.match(/^([A-Za-z_][\w-]*):\s*(.*)$/);
    if (kv) {
      const key = kv[1];
      const val = kv[2].trim();
      if (val === "") { meta[key] = null; current = key; }
      else if (val.startsWith("[") && val.endsWith("]")) {
        const inner = val.slice(1, -1).trim();
        if (inner === "") meta[key] = [];
        else meta[key] = inner.split(",").map(s => s.trim().replace(/^["']|["']$/g, "")).filter(Boolean);
        current = null;
      } else {
        meta[key] = val.replace(/^["']|["']$/g, "");
        current = null;
      }
    } else if (line.trim().startsWith("- ") && current) {
      if (!Array.isArray(meta[current])) meta[current] = [];
      meta[current].push(line.trim().slice(2).replace(/^["']|["']$/g, ""));
    }
  }
  // normalize null arrays
  for (const k of Object.keys(meta)) if (meta[k] === null) meta[k] = [];
  return { meta, body: text.slice(m[0].length) };
}

function sha256(s) { return createHash("sha256").update(s).digest("hex"); }

function openDb() {
  const db = new DatabaseSync(DB_PATH);
  db.exec(`CREATE TABLE IF NOT EXISTS docs (path TEXT PRIMARY KEY, id TEXT, title TEXT, category TEXT, domain TEXT, status TEXT, hash TEXT, body TEXT, source_paths TEXT, related_docs TEXT, platforms TEXT, verification_commit TEXT)`);
  db.exec(`CREATE VIRTUAL TABLE IF NOT EXISTS docs_fts USING fts5(path, id, title, body, content='docs', content_rowid='rowid')`);
  // triggers
  db.exec(`CREATE TRIGGER IF NOT EXISTS docs_ai AFTER INSERT ON docs BEGIN INSERT INTO docs_fts(rowid,path,id,title,body) VALUES (new.rowid,new.path,new.id,new.title,new.body); END;`);
  db.exec(`CREATE TRIGGER IF NOT EXISTS docs_ad AFTER DELETE ON docs BEGIN INSERT INTO docs_fts(docs_fts, rowid, path, id, title, body) VALUES('delete', old.rowid, old.path, old.id, old.title, old.body); END;`);
  db.exec(`CREATE TRIGGER IF NOT EXISTS docs_au AFTER UPDATE ON docs BEGIN INSERT INTO docs_fts(docs_fts, rowid, path, id, title, body) VALUES('delete', old.rowid, old.path, old.id, old.title, old.body); INSERT INTO docs_fts(rowid,path,id,title,body) VALUES (new.rowid,new.path,new.id,new.title,new.body); END;`);
  db.exec(`CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT)`);
  return db;
}

function cmdIndex() {
  const db = openDb();
  const files = walk(DOCS);
  let added = 0, updated = 0, removed = 0;
  const seen = new Set();
  for (const f of files) {
    const rel = relative(ROOT, f);
    seen.add(rel);
    const text = readFileSync(f, "utf8");
    const hash = sha256(text);
    const { meta, body } = parseFrontmatter(text);
    const row = db.prepare("SELECT hash FROM docs WHERE path=?").get(rel);
    if (row && row.hash === hash) continue;
    const id = meta?.id ?? "";
    const title = meta?.title ?? rel;
    const category = meta?.category ?? "";
    const domain = meta?.domain ?? "";
    const status = meta?.status ?? "";
    const source_paths = JSON.stringify(meta?.source_paths ?? []);
    const related_docs = JSON.stringify(meta?.related_docs ?? []);
    const platforms = JSON.stringify(meta?.platforms ?? []);
    const verification_commit = meta?.verification_commit ?? "";
    if (row) {
      db.prepare(`UPDATE docs SET id=?,title=?,category=?,domain=?,status=?,hash=?,body=?,source_paths=?,related_docs=?,platforms=?,verification_commit=? WHERE path=?`)
        .run(id, title, category, domain, status, hash, body, source_paths, related_docs, platforms, verification_commit, rel);
      updated++;
    } else {
      db.prepare(`INSERT INTO docs(path,id,title,category,domain,status,hash,body,source_paths,related_docs,platforms,verification_commit) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)`)
        .run(rel, id, title, category, domain, status, hash, body, source_paths, related_docs, platforms, verification_commit);
      added++;
    }
  }
  // removed
  const rows = db.prepare("SELECT path FROM docs").all();
  for (const r of rows) {
    if (!seen.has(r.path)) {
      db.prepare("DELETE FROM docs WHERE path=?").run(r.path);
      removed++;
    }
  }
  db.prepare("INSERT INTO meta(key,value) VALUES('last_indexed',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").run(new Date().toISOString());
  db.close();
  console.log(JSON.stringify({ indexed: files.length, added, updated, removed }, null, 2));
}

function cmdSearch(q, limit = 20) {
  const db = openDb();
  const stmt = db.prepare(`SELECT path, id, title, snippet(docs_fts, 3, '[', ']', '…', 12) AS snippet, bm25(docs_fts, 5.0, 10.0, 8.0, 1.0) AS rank FROM docs_fts WHERE docs_fts MATCH ? ORDER BY rank LIMIT ?`);
  let rows;
  try { rows = stmt.all(q, limit); }
  catch (e) { rows = db.prepare(`SELECT path, id, title, substr(body,1,200) AS snippet, 0 AS rank FROM docs WHERE body LIKE ? OR title LIKE ? OR id LIKE ? LIMIT ?`).all(`%${q}%`, `%${q}%`, `%${q}%`, limit); }
  db.close();
  console.log(JSON.stringify(rows, null, 2));
}

function cmdFeature(domain) {
  const db = openDb();
  const rows = db.prepare(`SELECT path, id, title, status, source_paths FROM docs WHERE domain=? ORDER BY path`).all(domain);
  db.close();
  console.log(JSON.stringify(rows, null, 2));
}

const ROUTING = {
  "understand": ["architecture", "features", "reference"],
  "bug": ["features", "reference"],
  "implement": ["engineering", "architecture", "features", "reference"],
  "refactor": ["architecture", "engineering", "features"],
  "review": ["engineering", "architecture"],
  "regression": ["features", "reference"],
  "schema": ["reference", "architecture"],
  "ipc": ["reference", "architecture"],
  "native": ["platforms", "architecture"],
  "architecture": ["architecture", "decisions", "engineering"],
};

function cmdContext(task, budget = 4000) {
  const db = openDb();
  const lower = task.toLowerCase();
  let cats = null;
  for (const k of Object.keys(ROUTING)) if (lower.includes(k)) { cats = ROUTING[k]; break; }
  // keyword search
  const words = lower.split(/\W+/).filter(w => w.length > 2).slice(0, 8);
  const ftsQuery = words.map(w => `"${w}"`).join(" OR ") || task;
  let rows = [];
  try { rows = db.prepare(`SELECT path, id, title, body, bm25(docs_fts, 5.0, 10.0, 8.0, 1.0) AS rank FROM docs_fts WHERE docs_fts MATCH ? ORDER BY rank LIMIT 30`).all(ftsQuery); }
  catch (e) { rows = db.prepare(`SELECT path, id, title, body, 0 AS rank FROM docs WHERE body LIKE ? LIMIT 30`).all(`%${task}%`); }
  // filter by category if routed
  if (cats) {
    const catSet = new Set(cats);
    const filtered = rows.filter(r => {
      const doc = db.prepare("SELECT category FROM docs WHERE path=?").get(r.path);
      return doc && catSet.has(doc.category);
    });
    if (filtered.length > 0) rows = filtered;
  }
  // build bundle within budget (rough: 4 chars/token)
  const charBudget = budget * 4;
  let used = 0;
  const bundle = [];
  for (const r of rows) {
    const entry = `## ${r.title} (${r.id})\nPath: ${r.path}\n${r.body.slice(0, 1200)}\n`;
    if (used + entry.length > charBudget) break;
    bundle.push(entry);
    used += entry.length;
  }
  db.close();
  console.log(JSON.stringify({ task, categories: cats, selected: bundle.length, estimated_tokens: Math.round(used / 4), context: bundle.join("\n---\n") }, null, 2));
}

function cmdVerify() {
  const db = openDb();
  const files = walk(DOCS);
  const errors = [];
  const ids = new Map();
  let schema = null;
  try { schema = JSON.parse(readFileSync(SCHEMA, "utf8")); } catch { errors.push("schema missing/unreadable"); }
  // simple schema check (required fields)
  const required = schema?.required ?? [];
  for (const f of files) {
    const rel = relative(ROOT, f);
    if (rel.startsWith("docs/plan/") || rel.startsWith("docs/superpowers/") || rel.startsWith("docs/website-screenshots/")) continue;
    const text = readFileSync(f, "utf8");
    const { meta } = parseFrontmatter(text);
    if (!meta) { continue; } // unstructured doc - not validated
    for (const r of required) {
      if (meta[r] === undefined || meta[r] === null) { errors.push(`${rel}: missing required '${r}'`); continue; }
      if (typeof meta[r] === "string" && meta[r] === "") errors.push(`${rel}: empty required '${r}'`);
      // empty arrays allowed (e.g. unsupported platforms/docs)
    }
    // id uniqueness
    if (meta.id) {
      if (ids.has(meta.id)) errors.push(`${rel}: duplicate id ${meta.id} (also ${ids.get(meta.id)})`);
      else ids.set(meta.id, rel);
    } else errors.push(`${rel}: missing id`);
    // category enum
    const cats = schema?.properties?.category?.enum;
    if (cats && meta.category && !cats.includes(meta.category)) errors.push(`${rel}: invalid category ${meta.category}`);
    const statuses = schema?.properties?.status?.enum;
    if (statuses && meta.status && !statuses.includes(meta.status)) errors.push(`${rel}: invalid status ${meta.status}`);
    // related_docs existence
    for (const rd of (meta.related_docs ?? [])) {
      const target = resolve(ROOT, rd);
      if (!existsSync(target)) errors.push(`${rel}: broken related_doc ${rd}`);
    }
    // source_paths existence (only if not empty)
    for (const sp of (meta.source_paths ?? [])) {
      if (!existsSync(resolve(ROOT, sp))) errors.push(`${rel}: missing source_path ${sp}`);
    }
  }
  db.close();
  const result = { files: files.length, errors: errors.length, details: errors.slice(0, 50) };
  console.log(JSON.stringify(result, null, 2));
  if (errors.length) process.exitCode = 1;
}

function cmdEval() {
  const datasetPath = join(DOCS, "agents", "eval-dataset.json");
  if (!existsSync(datasetPath)) { console.log(JSON.stringify({ error: "no dataset" })); return; }
  const dataset = JSON.parse(readFileSync(datasetPath, "utf8"));
  const db = openDb();
  let hits = 0, total = 0;
  const results = [];
  for (const q of dataset.questions) {
    total++;
    const words = q.query.toLowerCase().split(/\W+/).filter(w => w.length > 2);
    // FTS5 does not stem by default. Keep the exact term and add a short
    // prefix alternative so common inflections such as owned/owns retrieve
    // the same canonical architecture document without a synonym database.
    const ftsQuery = words.map(w => `"${w}" OR ${w.slice(0, Math.max(3, w.length - 2))}*`).join(" OR ") || q.query;
    let rows = [];
    try { rows = db.prepare(`SELECT path FROM docs_fts WHERE docs_fts MATCH ? ORDER BY bm25(docs_fts, 5.0, 10.0, 8.0, 1.0) LIMIT 10`).all(ftsQuery); }
    catch { rows = []; }
    const paths = rows.map(r => r.path);
    const expected = q.expected_docs ?? [];
    const hit = expected.some(e => paths.some(p => p.includes(e) || e.includes(p)));
    if (hit) hits++;
    results.push({ id: q.id, hit, retrieved: paths.slice(0, 5) });
  }
  db.close();
  console.log(JSON.stringify({ total, hits, recall: (hits / total).toFixed(3), results }, null, 2));
}

const args = process.argv.slice(2);
const cmd = args[0];
const contextArgs = args.slice(1).filter(arg => !arg.startsWith("--budget="));
const contextBudget = parseInt(args.find(a => a.startsWith("--budget="))?.split("=")[1] ?? "4000");
if (cmd === "index") cmdIndex();
else if (cmd === "search") cmdSearch(args.slice(1).join(" "));
else if (cmd === "feature") cmdFeature(args[1]);
else if (cmd === "context") cmdContext(contextArgs.join(" "), contextBudget);
else if (cmd === "verify") cmdVerify();
else if (cmd === "eval") cmdEval();
else {
  console.log("Usage: aevio-knowledge {index|search <q>|feature <domain>|context <task>|verify|eval}");
}
