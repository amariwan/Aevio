import { useCallback, useEffect, useMemo, useState } from "react";
import { useCatalog } from "@/hooks/useCatalog";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import * as api from "@/lib/api";
import { startOfDay } from "@/lib/dates";
import type { Task, TimeEntry } from "@/lib/types";

const DAY_MS = 86_400_000;
const RANGE_DAYS = 30;

export interface DashboardDay {
  date: Date;
  label: string;
  trackedMs: number;
}

function duration(entry: TimeEntry): number {
  return Math.max(0, entry.endedAt - entry.startedAt);
}

function priorityWeight(priority: string): number {
  return { urgent: 4, high: 3, normal: 2, low: 1 }[priority] ?? 0;
}

function dayLabel(date: Date): string {
  return date.toLocaleDateString(undefined, { weekday: "short" }).slice(0, 2);
}

export function useDashboardData() {
  const catalog = useCatalog();
  const [entries, setEntries] = useState<TimeEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const today = startOfDay(new Date());

  const reload = useCallback(async (): Promise<void> => {
    setLoading(true);
    try {
      const start = new Date(startOfDay(new Date()).getTime() - RANGE_DAYS * DAY_MS);
      const next = await api.listTimeEntries(start.getTime(), Date.now());
      setEntries(next);
      setError(null);
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);
  useTauriEvent(api.ENTRIES_CHANGED, () => void reload());

  const days = useMemo<DashboardDay[]>(() => {
    return Array.from({ length: 7 }, (_, index) => {
      const date = new Date(today.getTime() - (6 - index) * DAY_MS);
      const start = startOfDay(date).getTime();
      const end = start + DAY_MS;
      const trackedMs = entries.reduce((sum, entry) => {
        const overlap = Math.max(
          0,
          Math.min(entry.endedAt, end) - Math.max(entry.startedAt, start),
        );
        return sum + overlap;
      }, 0);
      return { date, label: dayLabel(date), trackedMs };
    });
  }, [entries, today]);

  const openTasks = useMemo(
    () =>
      catalog.tasks
        .filter((task) => task.status !== "done")
        .sort((left, right) => {
          const priority = priorityWeight(right.priority) - priorityWeight(left.priority);
          return priority || (left.dueDate ?? Infinity) - (right.dueDate ?? Infinity);
        }),
    [catalog.tasks],
  );

  const projectActivity = useMemo(() => {
    const totals = new Map<string, number>();
    for (const entry of entries) {
      if (entry.projectId)
        totals.set(entry.projectId, (totals.get(entry.projectId) ?? 0) + duration(entry));
    }
    return [...totals.entries()]
      .map(([projectId, trackedMs]) => ({ project: catalog.projectById.get(projectId), trackedMs }))
      .filter((item) => item.project)
      .sort((left, right) => right.trackedMs - left.trackedMs)
      .slice(0, 5);
  }, [catalog.projectById, entries]);

  const pendingEntries = entries.filter((entry) => entry.status === "pending").length;
  const todayMs = days[6]?.trackedMs ?? 0;
  const weekMs = days.reduce((sum, day) => sum + day.trackedMs, 0);
  const overdueTasks = openTasks.filter((task) => task.dueDate && task.dueDate < Date.now());
  const blockedTasks = openTasks.filter((task) => task.status === "blocked");

  return {
    catalog,
    entries,
    days,
    openTasks,
    projectActivity,
    recentEntries: [...entries].sort((left, right) => right.startedAt - left.startedAt).slice(0, 6),
    metrics: {
      todayMs,
      weekMs,
      activeProjects: catalog.projects.filter((project) => project.status === "active").length,
      openTasks: openTasks.length,
      pendingEntries,
    },
    notifications: { pendingEntries, blockedTasks, overdueTasks },
    loading,
    error,
    reload,
  };
}

export function taskStatusLabel(task: Task): string {
  return task.status === "in_progress" ? "In progress" : task.status.replace("_", " ");
}
