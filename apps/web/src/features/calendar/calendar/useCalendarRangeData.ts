import { useCallback, useEffect, useRef, useState } from "react";

import { useTauriEvent } from "@/hooks/useTauriEvent";
import * as api from "@/lib/api";
import { describeError } from "@/lib/api";
import type { BreakEntry } from "@/lib/breaks";
import { calendarDayEdges } from "@/lib/dates";
import type { ActivitySegment, CalendarScale, RollupCell, TimeEntry } from "@/lib/types";

interface CalendarGrid {
  start: Date;
  end: Date;
}

interface CalendarRangeDataOptions {
  scale: CalendarScale;
  startMs: number;
  endMs: number;
  grid: CalendarGrid;
}

function segmentsInRange(
  segments: ActivitySegment[],
  startMs: number,
  endMs: number,
): ActivitySegment[] {
  return segments.filter((segment) => segment.startedAt >= startMs && segment.startedAt < endMs);
}

export function useCalendarRangeData({ scale, startMs, endMs, grid }: CalendarRangeDataOptions) {
  const [entries, setEntries] = useState<TimeEntry[]>([]);
  const [segments, setSegments] = useState<ActivitySegment[]>([]);
  const [cells, setCells] = useState<RollupCell[]>([]);
  const [breakEntries, setBreakEntries] = useState<BreakEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(Date.now);

  const rangeKey = `${scale}:${startMs}`;
  const shownKey = useRef(rangeKey);
  shownKey.current = rangeKey;

  const refresh = useCallback(async (): Promise<void> => {
    setNow(Date.now());
    const requestedKey = `${scale}:${startMs}`;
    try {
      if (scale === "month") {
        const [nextCells, breaks] = await Promise.all([
          api.entryRollup(
            { startMs: grid.start.getTime(), endMs: grid.end.getTime() },
            calendarDayEdges(grid.start, grid.end),
            "category",
          ),
          api.listBreaks(startMs, endMs),
        ]);
        if (shownKey.current === requestedKey) {
          setCells(nextCells);
          setBreakEntries(breaks);
        }
      } else {
        const [nextEntries, snapshot, breaks] = await Promise.all([
          api.listTimeEntries(startMs, endMs - 1),
          scale === "day" ? api.fetchActivitySnapshot(startMs) : null,
          api.listBreaks(startMs, endMs),
        ]);
        if (shownKey.current === requestedKey) {
          setEntries(nextEntries);
          setBreakEntries(breaks);
          if (snapshot) setSegments(segmentsInRange(snapshot.segments, startMs, endMs));
        }
      }
      setError(null);
    } catch (cause) {
      setError(describeError(cause));
    }
  }, [scale, startMs, endMs, grid]);

  const load = useCallback(async (): Promise<void> => {
    const requestedKey = `${scale}:${startMs}`;
    setLoading(true);
    try {
      const [nextEntries, snapshot, breaks] = await Promise.all([
        api.rebuildTimeEntries(startMs, endMs - 1),
        scale === "day" ? api.fetchActivitySnapshot(startMs) : null,
        api.listBreaks(startMs, endMs),
      ]);
      if (shownKey.current !== requestedKey) return;
      setBreakEntries(breaks);
      if (scale === "month") {
        await refresh();
      } else {
        setEntries(nextEntries);
        setError(null);
      }
      setSegments(snapshot ? segmentsInRange(snapshot.segments, startMs, endMs) : []);
    } catch (cause) {
      setError(describeError(cause));
    } finally {
      setLoading(false);
    }
  }, [scale, startMs, endMs, refresh]);

  useEffect(() => {
    void load();
  }, [load]);

  useTauriEvent(api.ENTRIES_CHANGED, () => void refresh());
  useTauriEvent(api.ACTIVITY_CHANGED, () => void refresh());
  useTauriEvent(api.BREAK_STATE_CHANGED, () => void refresh());
  useTauriEvent(api.SUGGESTION_READY, () => void refresh());

  useEffect(() => {
    let timer: number | undefined;
    const update = (): void => {
      if (document.visibilityState !== "visible" || !document.hasFocus()) return;
      setNow(Date.now());
      void refresh();
    };
    const sync = (): void => {
      window.clearInterval(timer);
      timer = undefined;
      if (document.visibilityState === "visible" && document.hasFocus()) {
        update();
        timer = window.setInterval(update, 15_000);
      }
    };
    document.addEventListener("visibilitychange", sync);
    window.addEventListener("focus", sync);
    window.addEventListener("blur", sync);
    sync();
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", sync);
      window.removeEventListener("focus", sync);
      window.removeEventListener("blur", sync);
    };
  }, [refresh]);

  return {
    entries,
    setEntries,
    segments,
    cells,
    breakEntries,
    loading,
    error,
    setError,
    now,
    refresh,
    load,
    shownKey,
  };
}
