import type { Slice } from "@/components/data-viz/Charts";
import { calendarDayEdges } from "@/lib/dates";
import { countsAsWork, durationOf } from "@/lib/entries";
import type { CalendarScale, Category, RollupCell, TimeEntry } from "@/lib/types";

interface CalendarSummaryOptions {
  scale: CalendarScale;
  cells: RollupCell[];
  entries: TimeEntry[];
  reviewQueue: TimeEntry[];
  categoryById: Map<string, Category>;
  gridStart: Date;
  rangeStart: Date;
  daysInRange: number;
  now: number;
}

export interface CalendarSummary {
  categories: Slice[];
  workMs: number;
  reviewedMs: number;
  count: number;
  toReview: number;
}

export function calculateCalendarSummary(options: CalendarSummaryOptions): CalendarSummary {
  const byCategory = new Map<string | null, number>();
  let workMs = 0;
  let reviewedMs = 0;
  let count = 0;
  let toReview = 0;
  if (options.scale === "month") {
    const firstDay = calendarDayEdges(options.gridStart, options.rangeStart).length - 1;
    for (const cell of options.cells) {
      if (cell.bucket < firstDay || cell.bucket >= firstDay + options.daysInRange) continue;
      const key = cell.key ?? null;
      byCategory.set(key, (byCategory.get(key) ?? 0) + cell.ms);
      if (countsAsWork(key, options.categoryById)) workMs += cell.ms;
      reviewedMs += cell.approvedMs;
      count += cell.entries;
      toReview += cell.pending;
    }
  } else {
    for (const entry of options.entries) {
      const duration = durationOf(entry, options.now);
      const key = entry.categoryId ?? null;
      byCategory.set(key, (byCategory.get(key) ?? 0) + duration);
      if (countsAsWork(key, options.categoryById)) workMs += duration;
      if (entry.status === "approved") reviewedMs += duration;
    }
    count = options.entries.length;
    toReview = options.reviewQueue.length;
  }
  return {
    categories: categorySlices(byCategory, options.categoryById),
    workMs,
    reviewedMs,
    count,
    toReview,
  };
}

export function categorySlices(
  totals: Map<string | null, number>,
  categoryById: Map<string, Category>,
): Slice[] {
  return [...totals.entries()]
    .map(([id, ms]) => {
      const category = id ? categoryById.get(id) : undefined;
      return {
        key: id ?? "none",
        label: category?.name ?? "Uncategorized",
        color: category?.color ?? "var(--fg-ghost)",
        ms,
      };
    })
    .sort(
      (left, right) =>
        (categoryById.get(left.key)?.sort ?? Number.MAX_SAFE_INTEGER) -
        (categoryById.get(right.key)?.sort ?? Number.MAX_SAFE_INTEGER),
    );
}
