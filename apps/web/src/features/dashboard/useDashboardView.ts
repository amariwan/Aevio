import { useMemo, useState } from "react";
import { useI18n } from "@/hooks/useI18n";
import {
  activityBuckets,
  clipEntries,
  periodBounds,
  previousBounds,
  rankEntries,
  summarize,
  type Period,
} from "./dashboardModel";
import { useDashboardData } from "./useDashboardData";

export function useDashboardView() {
  const { t, locale } = useI18n();
  const [date, setDate] = useState(() => new Date());
  const [period, setPeriod] = useState<Period>("week");
  const [projectId, setProjectId] = useState("");
  const bounds = useMemo(() => periodBounds(date, period), [date, period]);
  const previous = useMemo(() => previousBounds(date, period), [date, period]);
  const data = useDashboardData();
  const analysis = useMemo(() => {
    const source = projectId
      ? data.entries.filter((entry) => entry.projectId === projectId)
      : data.entries;
    const entries = clipEntries(source, bounds);
    const before = clipEntries(source, previous);
    const priorBuckets = activityBuckets(before, previous, period, locale);
    const chart = activityBuckets(entries, bounds, period, locale).map((bucket, index) => ({
      ...bucket,
      previousTracked: priorBuckets[index]?.tracked ?? null,
      previousBillable: priorBuckets[index]?.billable ?? null,
    }));
    const categories = rankEntries(entries, (entry) => ({
      id: entry.categoryId ?? "",
      name:
        data.catalog.categoryById.get(entry.categoryId ?? "")?.name ?? t("dashboard.uncategorized"),
    }));
    const projects = rankEntries(entries, (entry) => ({
      id: entry.projectId ?? "",
      name: data.catalog.projectById.get(entry.projectId ?? "")?.name ?? t("dashboard.noProject"),
    }));
    const apps = rankEntries(entries, (entry) => ({
      id: entry.dominantApp ?? "",
      name: entry.dominantApp || t("dashboard.otherApp"),
    }));
    return {
      entries,
      chart,
      categories,
      projects,
      apps,
      summary: summarize(entries, data.catalog.categoryById),
      comparison: summarize(before, data.catalog.categoryById),
      recent: [...entries].sort((a, b) => b.startedAt - a.startedAt).slice(0, 5),
    };
  }, [
    data.entries,
    data.catalog.categoryById,
    data.catalog.projectById,
    bounds,
    previous,
    period,
    projectId,
    locale,
    t,
  ]);
  const tasks = data.openTasks.filter((task) => !projectId || task.projectId === projectId);
  const reset = () => {
    setDate(new Date());
    setProjectId("");
  };
  return {
    ...analysis,
    data,
    date,
    setDate,
    period,
    setPeriod,
    projectId,
    setProjectId,
    bounds,
    tasks,
    reset,
  };
}

export type DashboardView = ReturnType<typeof useDashboardView>;
