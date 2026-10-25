import assert from "node:assert/strict";
import { test } from "node:test";
import { localDay, nextDay } from "../apps/web/src/features/coach/date.ts";

test("planning navigation uses civil dates across month and year boundaries", () => {
  assert.equal(nextDay("2026-12-31"), "2027-01-01");
  assert.equal(nextDay("2028-02-28"), "2028-02-29");
  assert.equal(nextDay("2026-02-28"), "2026-03-01");
});
test("day labels stay local and do not truncate UTC dates", () => {
  const date = new Date(2026, 9, 20, 0, 5);
  assert.equal(localDay(date), "2026-10-20");
});
test("calendar navigation remains one day over both DST transitions", () => {
  assert.equal(nextDay("2026-03-29"), "2026-03-30");
  assert.equal(nextDay("2026-10-25"), "2026-10-26");
});
