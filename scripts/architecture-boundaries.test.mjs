import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const ROOT = path.resolve(import.meta.dirname, "..");
const CORE = path.join(ROOT, "crates/aevio-core");

const FORBIDDEN_CORE_DEPENDENCIES = [
  "dirs",
  "futures-lite",
  "interprocess",
  "libc",
  "reqwest",
  "rusqlite",
  "sea-orm-sync",
  "tokio",
  "windows",
];

const FORBIDDEN_CORE_SOURCE = [
  /\b(?:reqwest|rusqlite|sea_orm|tokio|interprocess)::/,
  /\bstd::(?:fs|net|path)::/,
  /\b(?:include_str!|include_bytes!)\s*\(\s*"[^"]*\.sql"/,
];

async function rustFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const candidate = path.join(directory, entry.name);
      if (entry.isDirectory()) return rustFiles(candidate);
      return entry.isFile() && entry.name.endsWith(".rs") ? [candidate] : [];
    }),
  );
  return files.flat();
}

test("aevio-core has no runtime, persistence, IPC, network, or filesystem dependencies", async () => {
  const manifest = await readFile(path.join(CORE, "Cargo.toml"), "utf8");
  const dependencyLeaks = FORBIDDEN_CORE_DEPENDENCIES.filter((dependency) =>
    new RegExp(`^${dependency.replaceAll("-", "\\-")}\\s*=`, "m").test(manifest),
  );

  const sourceLeaks = [];
  for (const file of await rustFiles(path.join(CORE, "src"))) {
    const source = await readFile(file, "utf8");
    if (FORBIDDEN_CORE_SOURCE.some((pattern) => pattern.test(source))) {
      sourceLeaks.push(path.relative(ROOT, file));
    }
  }

  assert.deepEqual(
    { dependencyLeaks, sourceLeaks },
    { dependencyLeaks: [], sourceLeaks: [] },
    "move adapters and side effects to persistence/infrastructure crates",
  );
});
