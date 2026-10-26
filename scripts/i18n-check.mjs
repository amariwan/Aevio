#!/usr/bin/env bun
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..");
const SRC = path.join(ROOT, "apps/web/src");
const I18N = path.join(SRC, "lib/i18n.ts");

const mod = await import(I18N);
const { messages } = mod;
const en = messages.en;
const de = messages.de;

const enKeys = Object.keys(en);
const deKeys = Object.keys(de);
const enSet = new Set(enKeys);
const deSet = new Set(deKeys);

let errors = [];
let warnings = [];

// 1. key parity
const missingInDe = enKeys.filter(k => !deSet.has(k));
const extraInDe = deKeys.filter(k => !enSet.has(k));
if (missingInDe.length) errors.push(`Missing in de (${missingInDe.length}): ${missingInDe.join(", ")}`);
if (extraInDe.length) errors.push(`Extra in de (${extraInDe.length}): ${extraInDe.join(", ")}`);

// 2. duplicate keys in source (static)
const src = await readFile(I18N, "utf8");
// naive: find repeated "key": within en/de blocks not trivial; skip runtime duplicate detection via Set size vs regex count
const keyRegex = /"([^"]+)":/g;
const allMatches = [...src.matchAll(keyRegex)].map(m => m[1]);
// duplicates overall (could be en+de both, so divide); detect keys appearing >2 times
const counts = new Map();
for (const k of allMatches) counts.set(k, (counts.get(k) || 0) + 1);
const dupes = [...counts.entries()].filter(([,c]) => c > 2).map(([k]) => k);
if (dupes.length) warnings.push(`Possible duplicate keys (>2 occurrences): ${dupes.join(", ")}`);

// 3. interpolation placeholders
function placeholders(val) {
  if (typeof val !== "string") return null; // function values skipped
  return [...val.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort();
}
for (const key of enKeys) {
  const ep = placeholders(en[key]);
  const dp = placeholders(de[key]);
  if (ep === null || dp === null) continue;
  if (JSON.stringify(ep) !== JSON.stringify(dp)) {
    warnings.push(`Placeholder mismatch for ${key}: en=${ep.join(",")} de=${dp.join(",")}`);
  }
}

// 4. used keys
async function walk(dir) {
  const out = [];
  for (const e of await readdir(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) out.push(...await walk(p));
    else if (/\.(ts|tsx)$/.test(e.name)) out.push(p);
  }
  return out;
}
const files = await walk(SRC);
const used = new Set();
const dynamic = [];
for (const f of files) {
  const s = await readFile(f, "utf8");
  for (const m of s.matchAll(/\bt\(\s*(['"])([^'"\n]+)\1/g)) used.add(m[2]);
  for (const m of s.matchAll(/\btranslate\(\s*[^,]+,\s*(['"])([^'"\n]+)\1/g)) used.add(m[2]);
  for (const m of s.matchAll(/\bt\(\s*`([^`]*)`/g)) {
    if (/\$\{/.test(m[1])) dynamic.push(m[1]);
    else used.add(m[1]);
  }
}
const usedButMissing = [...used].filter(k => !enSet.has(k));
if (usedButMissing.length) errors.push(`Used but undefined keys (${usedButMissing.length}): ${usedButMissing.join(", ")}`);

const unused = enKeys.filter(k => !used.has(k));
if (unused.length) warnings.push(`Potentially unused keys (${unused.length}): ${unused.slice(0,20).join(", ")}${unused.length>20?" ...":""}`);

// 5. dynamic key prefixes (best-effort): ensure dashboard.metric/period keys exist
const dynamicPrefixes = new Set(["dashboard."]);
// not enforced


// 6. hardcoded UI text heuristic (TSX)
let hardcodedCount = 0;
const hardcodedSamples = [];
for (const f of files) {
  if (!f.endsWith(".tsx")) continue;
  const s2 = await readFile(f, "utf8");
  for (const m of s2.matchAll(/>\s*([A-Za-zÄÖÜäöüß][^<>{}]{2,})\s*</g)) {
    const txt = m[1].trim();
    if (!txt) continue;
    // skip if contains t( already handled by jsx; these are raw text nodes
    hardcodedCount++;
    if (hardcodedSamples.length < 25) hardcodedSamples.push(`${path.relative(ROOT, f)}: ${txt.slice(0,60)}`);
  }
}
if (hardcodedCount) warnings.push(`Hardcoded JSX text nodes: ${hardcodedCount} (sampled ${hardcodedSamples.length})`);

console.log("=== i18n check ===");
console.log(`en keys: ${enKeys.length}, de keys: ${deKeys.length}`);
console.log(`used literal keys: ${used.size}`);
console.log(`hardcoded JSX text nodes: ${hardcodedCount}`);
console.log(`dynamic templates: ${dynamic.length}`);
if (errors.length) {
  console.log("\nERRORS:");
  errors.forEach(e => console.log("  - " + e));
}
if (warnings.length) {
  console.log("\nWARNINGS:");
  warnings.forEach(w => console.log("  - " + w));
}
if (errors.length) {
  console.error(`\nFAILED: ${errors.length} error(s)`);
  process.exit(1);
}
console.log("\nOK: no blocking errors");
