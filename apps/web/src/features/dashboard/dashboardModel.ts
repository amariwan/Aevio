import { addDays, addMonths, startOfDay, startOfMonth, startOfWeek } from "../../lib/dates.ts";
import type { Category, TimeEntry } from "../../lib/types.ts";

export type Period = "day" | "week" | "month" | "year";
export type Bounds = { start: number; end: number };
export type Ranking = { id: string; name: string; ms: number };
export type ActivityBucket = Bounds & { label: string; tracked: number; billable: number };

export function periodBounds(date: Date, period: Period): Bounds {
  if (period === "year") {
    const year = date.getFullYear();
    return { start: +new Date(year, 0, 1), end: +new Date(year + 1, 0, 1) };
  }
  const start =
    period === "week"
      ? startOfWeek(date)
      : period === "month"
        ? startOfMonth(date)
        : startOfDay(date);
  const end = period === "month" ? addMonths(start, 1) : addDays(start, period === "week" ? 7 : 1);
  return { start: +start, end: +end };
}

export function shiftPeriod(date: Date, period: Period, amount: number): Date {
  if (period === "year") return new Date(date.getFullYear() + amount, 0, 1);
  if (period === "month") return addMonths(date, amount);
  return addDays(date, amount * (period === "week" ? 7 : 1));
}

export function previousBounds(date: Date, period: Period): Bounds {
  return periodBounds(shiftPeriod(new Date(periodBounds(date, period).start), period, -1), period);
}

export function clipEntries(entries: TimeEntry[], bounds: Bounds): TimeEntry[] {
  return entries
    .filter(
      (entry) =>
        entry.endedAt > bounds.start &&
        entry.startedAt < bounds.end &&
        entry.endedAt > entry.startedAt,
    )
    .map((entry) => ({
      ...entry,
      startedAt: Math.max(entry.startedAt, bounds.start),
      endedAt: Math.min(entry.endedAt, bounds.end),
    }));
}

export function summarize(
  entries: TimeEntry[],
  categories: ReadonlyMap<string, Pick<Category, "countsAsWork">>,
) {
  let trackedMs = 0;
  let workMs = 0;
  let billableMs = 0;
  let pending = 0;
  const projects = new Set<string>();
  for (const entry of entries) {
    const ms = Math.max(0, entry.endedAt - entry.startedAt);
    trackedMs += ms;
    if (categories.get(entry.categoryId ?? "")?.countsAsWork !== false) workMs += ms;
    if (entry.billable) billableMs += ms;
    if (entry.status === "pending") pending++;
    if (entry.projectId) projects.add(entry.projectId);
  }
  return {
    trackedMs,
    workMs,
    billableMs,
    pending,
    projects: projects.size,
    entries: entries.length,
  };
}

export function activityBuckets(
  entries: TimeEntry[],
  bounds: Bounds,
  period: Period,
  locale: string,
): ActivityBucket[] {
  const buckets: ActivityBucket[] = [];
  let start = bounds.start;
  while (start < bounds.end) {
    const date = new Date(start);
    const end =
      period === "day"
        ? start + 3_600_000
        : +(period === "year" ? addMonths(date, 1) : addDays(date, 1));
    const label = date.toLocaleDateString(
      locale,
      period === "year" ? { month: "short" } : { day: "numeric", month: "short" },
    );
    buckets.push({
      start,
      end: Math.min(end, bounds.end),
      label:
        period === "day"
          ? date.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" })
          : label,
      tracked: 0,
      billable: 0,
    });
    start = end;
  }
  for (const entry of entries) {
    for (const bucket of buckets) {
      const ms = Math.max(
        0,
        Math.min(entry.endedAt, bucket.end) - Math.max(entry.startedAt, bucket.start),
      );
      bucket.tracked += ms;
      if (entry.billable) bucket.billable += ms;
    }
  }
  return buckets;
}

export function rankEntries(
  entries: TimeEntry[],
  identify: (entry: TimeEntry) => { id: string; name: string },
): Ranking[] {
  const ranks = new Map<string, Ranking>();
  for (const entry of entries) {
    const { id, name } = identify(entry);
    const item = ranks.get(id) ?? { id, name, ms: 0 };
    item.ms += Math.max(0, entry.endedAt - entry.startedAt);
    ranks.set(id, item);
  }
  return [...ranks.values()].sort((a, b) => b.ms - a.ms);
}
