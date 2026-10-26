import { ArrowDownRight, ArrowUpRight, Clock3, FolderOpen, ListChecks, Target } from "lucide-react";
import { useReducedMotion } from "motion/react";
import { AnimatedCounter } from "@/components/charts";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import { formatDuration } from "@/lib/format";
import type { Route } from "@/lib/types";
import { localDateString } from "@/lib/dates";
import type { DashboardView } from "./useDashboardView";

export function DashboardMetrics({
  view,
  navigate,
}: {
  view: DashboardView;
  navigate: (route: Route) => void;
}) {
  const { t, locale } = useI18n();
  const reduced = useReducedMotion();
  const { summary, comparison } = view;
  const delta =
    comparison.trackedMs > 0
      ? Math.round(((summary.trackedMs - comparison.trackedMs) / comparison.trackedMs) * 100)
      : null;
  const workShare =
    summary.trackedMs > 0 ? Math.round((summary.workMs / summary.trackedMs) * 100) : 0;
  const metrics = [
    {
      label: t("dashboard.trackedTime"),
      value: summary.trackedMs,
      format: formatDuration,
      icon: Clock3,
      detail:
        delta === null
          ? t("dashboard.noPrevious")
          : `${delta > 0 ? "+" : ""}${delta}% ${t("dashboard.vsPrevious")}`,
    },
    {
      label: t("dashboard.workShare"),
      value: workShare,
      format: (value: number) => `${Math.round(value)}%`,
      icon: Target,
      detail: `${formatDuration(summary.workMs)} / ${formatDuration(summary.trackedMs)}`,
    },
    {
      label: t("dashboard.projects"),
      value: summary.projects,
      format: (value: number) => Math.round(value).toLocaleString(locale),
      icon: FolderOpen,
      detail: t("dashboard.selectedPeriod"),
    },
    {
      label: t("dashboard.pendingReview"),
      value: summary.pending,
      format: (value: number) => Math.round(value).toLocaleString(locale),
      icon: ListChecks,
      detail: t("dashboard.timeEntriesWaiting"),
    },
  ];
  return (
    <section
      aria-label={t("dashboard.overview")}
      className="grid grid-cols-2 gap-y-6 border-b border-line pb-6 lg:grid-cols-4"
    >
      {metrics.map((metric, index) => (
        <div
          key={metric.label}
          className="flex min-w-0 flex-col gap-3 px-4 first:pl-0 lg:border-r lg:border-line lg:last:border-0"
        >
          <div className="flex items-center gap-2 text-fg-muted">
            <metric.icon className="size-4" aria-hidden="true" />
            <span className="text-xs font-medium sm:text-sm">{metric.label}</span>
          </div>
          <p className="text-3xl font-medium tracking-tight tabular-nums text-fg-strong sm:text-4xl">
            {reduced ? (
              metric.format(metric.value)
            ) : (
              <AnimatedCounter value={metric.value} format={metric.format} />
            )}
          </p>
          {index === 3 && summary.pending > 0 ? (
            <Button
              variant="link"
              size="sm"
              className="h-auto justify-start self-start p-0"
              onClick={() =>
                navigate({ name: "calendar", date: localDateString(view.date), review: true })
              }
            >
              {t("dashboard.reviewEntries")}
              <ArrowUpRight data-icon="inline-end" />
            </Button>
          ) : (
            <p className="flex items-center gap-1 text-xs text-fg-muted">
              {index === 0 &&
                delta !== null &&
                (delta >= 0 ? (
                  <ArrowUpRight className="size-3" aria-hidden="true" />
                ) : (
                  <ArrowDownRight className="size-3" aria-hidden="true" />
                ))}
              {metric.detail}
            </p>
          )}
        </div>
      ))}
    </section>
  );
}
