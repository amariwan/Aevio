import { useCallback, useEffect, useRef, useState } from "react";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { ACTIVITY_CHANGED, COACH_DAY_CHANGED, describeError } from "@/lib/api";
import * as api from "./api";
export interface CoachFocus {
  id: string;
  taskId: string;
  segmentId: number;
  title: string;
  goal: string;
  nextStep: string;
  plannedMinutes: number;
  bodyDoubling: boolean;
  state: "running" | "interrupted";
  startedAt: number;
  endedAt: number | null;
  elapsedMs: number;
}
export function useCoachFocus() {
  const [focus, setFocus] = useState<CoachFocus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [now, setNow] = useState(Date.now);
  const sequence = useRef(0);
  const pending = useRef(false);
  const refresh = useCallback(async () => {
    const request = ++sequence.current;
    try {
      const next = await api.fetchFocus<CoachFocus>();
      if (request === sequence.current) {
        setFocus(next);
        setError(null);
      }
    } catch (cause) {
      if (request === sequence.current) setError(describeError(cause));
    }
  }, []);
  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => {
      sequence.current++;
      window.clearInterval(timer);
    };
  }, [refresh]);
  useTauriEvent(ACTIVITY_CHANGED, () => void refresh());
  useTauriEvent(COACH_DAY_CHANGED, () => void refresh());
  async function action(
    command: "coach_start_focus" | "coach_end_focus",
    args: Record<string, unknown>,
  ) {
    if (pending.current) return;
    pending.current = true;
    sequence.current++;
    setBusy(true);
    setError(null);
    try {
      const request = sequence.current;
      const next = await api.runFocusAction<CoachFocus>(command, args);
      if (request === sequence.current) setFocus(next);
    } catch (cause) {
      setError(describeError(cause));
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  return { focus, error, busy, now, action };
}
