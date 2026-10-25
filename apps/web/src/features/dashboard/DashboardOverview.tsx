import { Activity, Clock3, Eye, ListChecks, Monitor, RefreshCw } from "lucide-react";
import type { ReactNode } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { formatDuration } from "@/lib/format";
import type { useDashboardData } from "./useDashboardData";

type DashboardData = ReturnType<typeof useDashboardData>;

function greeting(): string {
  const hour = new Date().getHours();
  if (hour < 5) return "Working late";
  if (hour < 12) return "Good morning";
  if (hour < 18) return "Good afternoon";
  return "Good evening";
}

function dayLabel(date: Date): string {
  return date.toLocaleDateString(undefined, { weekday: "short" }).slice(0, 2);
}

function Stat({ icon, label, value }: { icon: ReactNode; label: string; value: string }) {
  return (
    <div className="rounded-xl border border-line/70 bg-surface/70 p-3">
      <div className="mb-2 flex items-center gap-2 text-fg-muted">
        {icon}
        <span className="text-[11px] font-medium uppercase tracking-wide">{label}</span>
      </div>
      <p className="text-xl font-semibold tabular-nums text-fg-strong">{value}</p>
    </div>
  );
}

export function DashboardOverview({
  data,
  onRefresh,
}: {
  data: DashboardData;
  onRefresh: () => void;
}) {
  const maxDay = Math.max(...data.days.map((day) => day.trackedMs), 1);
  const currentApp = data.recentEntries[0]?.dominantApp ?? "Not tracking";
  return (
    <Card className="mb-4 overflow-hidden border-line/80 bg-panel/80 shadow-sm">
      <CardHeader className="flex-row items-start justify-between gap-4 pb-4">
        <div>
          <p className="mb-1 text-[11px] font-semibold uppercase tracking-[0.18em] text-accent">
            {greeting()}
          </p>
          <CardTitle className="text-2xl tracking-tight text-fg-strong">Your workspace</CardTitle>
          <p className="mt-1 text-sm text-fg-muted">
            A local overview of your recent focus and activity.
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={onRefresh} disabled={data.loading}>
          <RefreshCw className={data.loading ? "size-4 animate-spin" : "size-4"} /> Refresh
        </Button>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <Stat
            icon={<Clock3 className="size-4" />}
            label="Tracked"
            value={formatDuration(data.metrics.weekMs)}
          />
          <Stat
            icon={<Activity className="size-4" />}
            label="Focus"
            value={formatDuration(data.metrics.todayMs)}
          />
          <Stat icon={<Eye className="size-4" />} label="Current" value={currentApp} />
          <Stat
            icon={<ListChecks className="size-4" />}
            label="Needs review"
            value={`${data.metrics.pendingEntries}`}
          />
        </div>
        <div className="grid gap-4 lg:grid-cols-[1.5fr_1fr]">
          <div className="rounded-xl border border-line/70 bg-surface/50 p-4">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <p className="text-sm font-semibold text-fg-strong">Activity history</p>
                <p className="text-xs text-fg-muted">Last 7 days</p>
              </div>
              <Badge variant="secondary">Local activity</Badge>
            </div>
            <div className="flex h-28 items-end gap-1.5">
              {data.days.map((day) => (
                <div
                  key={day.date.toISOString()}
                  className="flex min-w-0 flex-1 flex-col items-center gap-2"
                >
                  <div className="flex h-20 w-full items-end">
                    <div
                      className="w-full rounded-t-md bg-accent transition-[height]"
                      style={{ height: `${Math.max(4, (day.trackedMs / maxDay) * 100)}%` }}
                      title={formatDuration(day.trackedMs)}
                    />
                  </div>
                  <span className="text-[10px] text-fg-faint">{dayLabel(day.date)}</span>
                </div>
              ))}
            </div>
          </div>
          <div className="rounded-xl border border-line/70 bg-surface/50 p-4">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <p className="text-sm font-semibold text-fg-strong">Top projects</p>
                <p className="text-xs text-fg-muted">Where your time went</p>
              </div>
              <Monitor className="size-4 text-accent" />
            </div>
            <div className="space-y-3">
              {data.projectActivity.map(({ project, trackedMs }) => {
                const share = (trackedMs / Math.max(data.metrics.weekMs, 1)) * 100;
                return (
                  <div key={project?.id}>
                    <div className="mb-1 flex items-center justify-between gap-3 text-xs">
                      <span className="truncate text-fg">{project?.name}</span>
                      <span className="shrink-0 tabular-nums text-fg-muted">
                        {formatDuration(trackedMs)}
                      </span>
                    </div>
                    <div className="h-1.5 overflow-hidden rounded-full bg-line/70">
                      <div
                        className="h-full rounded-full bg-accent"
                        style={{ width: `${Math.min(100, share)}%` }}
                      />
                    </div>
                  </div>
                );
              })}
              {data.projectActivity.length === 0 && (
                <p className="text-sm text-fg-muted">No activity recorded yet.</p>
              )}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
