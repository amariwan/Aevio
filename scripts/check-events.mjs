// Checks that event-name string literals stay in sync between the Rust core
// (apps/desktop/tauri/src/**.rs) and the TypeScript IPC layer (src/lib/api.ts).
// Exit code 1 on mismatch.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

function walk(dir, ext) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) out.push(...walk(p, ext));
    else if (name.endsWith(ext)) out.push(p);
  }
  return out;
}

function rustStrings() {
  const files = walk("apps/desktop/tauri/src", ".rs");
  const set = new Set();
  for (const f of files) {
    const src = readFileSync(f, "utf8");
    for (const m of src.matchAll(/pub const EVENT_\w+:\s*&str\s*=\s*"([^"]+)"/g)) {
      set.add(m[1]);
    }
  }
  return set;
}

function tsStrings() {
  const files = ["apps/web/src/lib/api.ts", "apps/web/src/lib/ipc/events.ts"];
  const set = new Set();
  for (const f of files) {
    const src = readFileSync(f, "utf8");
    for (const m of src.matchAll(/export const [A-Z_]+ = "([^"]+)"/g)) {
      set.add(m[1]);
    }
  }
  return set;
}

const r = rustStrings();
const t = tsStrings();
const onlyRust = [...r].filter((x) => !t.has(x));
const onlyTs = [...t].filter((x) => !r.has(x));

if (onlyRust.length || onlyTs.length) {
  console.error("Event-name parity check FAILED");
  if (onlyRust.length) console.error("  only in Rust:", onlyRust.join(", "));
  if (onlyTs.length) console.error("  only in TS:  ", onlyTs.join(", "));
  process.exit(1);
}
console.log(`Event-name parity OK (${r.size} shared names)`);