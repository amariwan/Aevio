import assert from "node:assert/strict";
import { test } from "node:test";
import { parseLocalDatetime, toLocalDatetimeInput } from "../apps/web/src/features/coach/date.ts";

test("parses a valid local datetime", () => {
  const ms = parseLocalDatetime("2026-10-20T14:30");
  assert.ok(ms);
  const d = new Date(ms);
  assert.equal(d.getFullYear(), 2026);
  assert.equal(d.getMonth(), 9);
  assert.equal(d.getDate(), 20);
  assert.equal(d.getHours(), 14);
  assert.equal(d.getMinutes(), 30);
});

test("rejects malformed and impossible dates", () => {
  assert.equal(parseLocalDatetime(""), null);
  assert.equal(parseLocalDatetime("2026-10-20"), null);
  assert.equal(parseLocalDatetime("2026-13-20T14:30"), null);
  assert.equal(parseLocalDatetime("2026-10-32T14:30"), null);
  assert.equal(parseLocalDatetime("2026-10-20T24:30"), null);
  assert.equal(parseLocalDatetime("not-a-date"), null);
});

test("round-trips through toLocalDatetimeInput", () => {
  const ms = new Date(2026, 9, 20, 14, 30, 0).getTime();
  assert.equal(toLocalDatetimeInput(ms), "2026-10-20T14:30");
  assert.equal(parseLocalDatetime(toLocalDatetimeInput(ms)), ms);
});

test("preserves DST boundary dates", () => {
  // March 29 2026 02:30 local (DST transition day) should parse consistently.
  const ms = parseLocalDatetime("2026-03-29T03:30");
  assert.ok(ms);
  assert.equal(toLocalDatetimeInput(ms), "2026-03-29T03:30");
});