import { useEffect } from "react";

import * as api from "@/lib/api";
import { describeError } from "@/lib/api";
import { isLocked, isTyping } from "@/lib/entries";
import type { CalendarScale, TimeEntry } from "@/lib/types";

interface CalendarShortcutsOptions {
  scale: CalendarScale;
  visibleEntries: TimeEntry[];
  reviewQueue: TimeEntry[];
  reviewing: boolean;
  selectedId?: string;
  selectedEntry?: TimeEntry;
  select: (id?: string) => void;
  startReviewMode: () => void;
  advanceFrom: (id: string) => Promise<void>;
  setReviewing: (reviewing: boolean) => void;
  setError: (message: string) => void;
}

export function useCalendarShortcuts(options: CalendarShortcutsOptions): void {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (isTyping(event.target) || event.metaKey || event.ctrlKey || event.altKey) return;
      const key = event.key.toLowerCase();
      if (event.key === "Escape") {
        options.setReviewing(false);
        return;
      }
      if (key === "r" && options.scale !== "month") {
        event.preventDefault();
        options.startReviewMode();
        return;
      }
      if ((key === "j" || key === "k") && options.visibleEntries.length > 0) {
        event.preventDefault();
        selectAdjacentEntry(key === "j" ? 1 : -1, options);
        return;
      }
      if (
        (event.key === "Backspace" || event.key === "Delete") &&
        options.selectedId &&
        !document.querySelector("dialog[open]")
      ) {
        event.preventDefault();
        deleteSelectedEntry(options);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [options]);
}

function selectAdjacentEntry(step: 1 | -1, options: CalendarShortcutsOptions): void {
  const entries = options.reviewing ? options.reviewQueue : options.visibleEntries;
  if (entries.length === 0) return;
  const selectedIndex = entries.findIndex((entry) => entry.id === options.selectedId);
  const nextIndex =
    selectedIndex === -1
      ? step === 1
        ? 0
        : entries.length - 1
      : (selectedIndex + step + entries.length) % entries.length;
  options.select(entries[nextIndex].id);
}

function deleteSelectedEntry(options: CalendarShortcutsOptions): void {
  const selectedId = options.selectedId;
  if (!selectedId) return;
  if (options.selectedEntry?.id === selectedId && isLocked(options.selectedEntry)) {
    options.setError("Approved entries can't be deleted. Unapprove this one first.");
    return;
  }
  void (async () => {
    try {
      await api.deleteTimeEntry(selectedId);
      if (options.reviewing) await options.advanceFrom(selectedId);
      else options.select(undefined);
    } catch (cause) {
      options.setError(describeError(cause));
    }
  })();
}
