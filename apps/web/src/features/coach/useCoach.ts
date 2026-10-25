import { useCallback, useEffect, useRef, useState } from "react";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { COACH_DAY_CHANGED, ENTRIES_CHANGED, describeError } from "@/lib/api";
import * as api from "./api";
import type { CoachCommand, Day } from "./types";

export function useCoach(day: string, timeZone: string) {
  const [snapshot, setSnapshot] = useState<Day | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const version = useRef(0);
  const writing = useRef(false);
  const selection = useRef(day + timeZone);
  selection.current = day + timeZone;
  const refresh = useCallback(async () => {
    const current = ++version.current;
    try {
      const next = await api.fetchDay(day, timeZone);
      if (current === version.current && !writing.current) {
        setSnapshot(next);
        setError(null);
      }
    } catch (cause) {
      if (current === version.current) setError(describeError(cause));
    }
  }, [day, timeZone]);
  useEffect(() => {
    setSnapshot(null);
    void refresh();
    return () => {
      version.current++;
    };
  }, [refresh]);
  useTauriEvent(COACH_DAY_CHANGED, () => {
    if (!writing.current) void refresh();
  });
  useTauriEvent(ENTRIES_CHANGED, () => {
    if (!writing.current) void refresh();
  });
  const apply = useCallback(
    async (command: CoachCommand): Promise<Day | null> => {
      if (writing.current) return null;
      writing.current = true;
      setBusy(true);
      setError(null);
      version.current++;
      const selected = day + timeZone;
      try {
        const next = await api.apply(day, timeZone, command);
        if (selection.current === selected) setSnapshot(next);
        return next;
      } catch (cause) {
        setError(describeError(cause));
        return null;
      } finally {
        writing.current = false;
        setBusy(false);
      }
    },
    [day, timeZone],
  );
  return { snapshot, error, busy, apply, refresh };
}
