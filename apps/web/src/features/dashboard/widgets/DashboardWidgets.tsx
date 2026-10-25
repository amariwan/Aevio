import {
  AlertCircle,
  ArrowUpRight,
  Bell,
  CheckCircle2,
  Circle,
  Clock3,
  FolderKanban,
  ListTodo,
  Plus,
  TimerReset,
} from "lucide-react";
import type { ReactNode } from "react";
import MetricCard from "@/components/kokonutui/metric-card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { formatDuration, formatRelative } from "@/lib/format";
import type { Route } from "@/lib/types";
import { taskStatusLabel, type useDashboardData } from "../useDashboardData";
import { ProductivityField } from "./ProductivityField";

type DashboardData = ReturnType<typeof useDashboardData>;

export function WidgetShell({
  title,
  eyebrow,
  icon,
  action,
  children,
  className = "",
}: {
  title: string;
  eyebrow?: string;
  icon: ReactNode;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <Card className={`overflow-hidden border-line/80 bg-panel/80 shadow-sm ${className}`}>
      <CardHeader className="flex-row items-start justify-between gap-3 border-b border-line/60 pb-3">
        <div className="flex min-w-0 items-center gap-2.5">
          <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-accent-soft text-accent">
            {icon}
          </span>
          <div className="min-w-0">
            {eyebrow && (
              <p className="text-[10px] font-semibold tracking-[0.16em] text-fg-faint uppercase">
                {eyebrow}
              </p>
            )}
            <CardTitle className="truncate text-sm text-fg-strong">{title}</CardTitle>
          </div>
        </div>
        {action}
      </CardHeader>
      <CardContent className="pt-4">{children}</CardContent>
    </Card>
  );
}

export function MetricsWidget({ data }: { data: DashboardData }) {
  return (
    <MetricCard
      className="lg:col-span-2 max-w-none"
      data={data.days.map((day) => ({
        date: day.date.toISOString().slice(0, 10),
        parts: [Math.round(day.trackedMs / 60_000)],
      }))}
      defaultPeriod={3}
      format="number"
      label="Tracked minutes"
      periods={[3]}
      segments={["Tracked time"]}
    />
  );
}

export function ActivityWidget({ data }: { data: DashboardData }) {
  return (
    <WidgetShell
      title="Live activity"
      eyebrow="Workspace pulse"
      icon={<TimerReset className="size-4" />}
      className="lg:col-span-3"
    >
      <ProductivityField data={data} />
    </WidgetShell>
  );
}

export function QueueWidget({
  data,
  onNavigate,
}: {
  data: DashboardData;
  onNavigate: (route: Route) => void;
}) {
  return (
    <WidgetShell
      title="Work queue"
      eyebrow="Next up"
      icon={<ListTodo className="size-4" />}
      action={
        <Button variant="ghost" size="sm" onClick={() => onNavigate({ name: "tasks" })}>
          View all <ArrowUpRight />
        </Button>
      }
    >
      <div className="space-y-2">
        {data.openTasks.slice(0, 5).map((task) => (
          <button
            key={task.id}
            type="button"
            onClick={() => onNavigate({ name: "tasks" })}
            className="flex w-full items-center gap-3 rounded-lg px-2 py-2 text-left transition-colors hover:bg-surface"
          >
            <Circle
              className={`size-3.5 shrink-0 ${task.priority === "urgent" ? "text-danger" : task.priority === "high" ? "text-review" : "text-fg-faint"}`}
            />
            <span className="min-w-0 flex-1 truncate text-sm text-fg">{task.title}</span>
            <span className="shrink-0 text-[10px] text-fg-faint">{taskStatusLabel(task)}</span>
          </button>
        ))}
        {data.openTasks.length === 0 && <Empty label="Your queue is clear." />}
      </div>
    </WidgetShell>
  );
}

export function NotificationsWidget({
  data,
  onNavigate,
}: {
  data: DashboardData;
  onNavigate: (route: Route) => void;
}) {
  const items = [
    data.notifications.pendingEntries > 0 &&
      `${data.notifications.pendingEntries} time entries need review`,
    data.notifications.blockedTasks.length > 0 &&
      `${data.notifications.blockedTasks.length} blocked task${data.notifications.blockedTasks.length === 1 ? "" : "s"}`,
    data.notifications.overdueTasks.length > 0 &&
      `${data.notifications.overdueTasks.length} overdue task${data.notifications.overdueTasks.length === 1 ? "" : "s"}`,
  ].filter((item): item is string => Boolean(item));
  return (
    <WidgetShell title="Notifications" eyebrow="Needs attention" icon={<Bell className="size-4" />}>
      <div className="space-y-3">
        {items.map((item) => (
          <button
            key={item}
            type="button"
            onClick={() => onNavigate({ name: "tasks" })}
            className="flex w-full items-center gap-3 text-left text-sm text-fg hover:text-accent"
          >
            <AlertCircle className="size-4 shrink-0 text-review" />
            {item}
          </button>
        ))}
        {items.length === 0 && (
          <div className="flex items-center gap-2 text-sm text-success">
            <CheckCircle2 className="size-4" />
            All caught up.
          </div>
        )}
      </div>
    </WidgetShell>
  );
}

export function QuickAccessWidget({ onNavigate }: { onNavigate: (route: Route) => void }) {
  const links: Array<[string, Route, typeof FolderKanban]> = [
    ["Projects", { name: "projects" }, FolderKanban],
    ["Calendar", { name: "calendar" }, Clock3],
    ["Time entries", { name: "entries" }, TimerReset],
    ["New task", { name: "tasks" }, Plus],
  ];
  return (
    <WidgetShell
      title="Quick access"
      eyebrow="Workspace"
      icon={<ArrowUpRight className="size-4" />}
    >
      <div className="grid grid-cols-2 gap-2">
        {links.map(([label, route, Icon]) => (
          <Button
            key={label}
            variant="outline"
            className="justify-start gap-2"
            onClick={() => onNavigate(route)}
          >
            <Icon className="size-3.5 text-accent" />
            {label}
          </Button>
        ))}
      </div>
    </WidgetShell>
  );
}

export function KanbanWidget({
  data,
  onNavigate,
}: {
  data: DashboardData;
  onNavigate: (route: Route) => void;
}) {
  const columns = ["todo", "in_progress", "blocked"] as const;
  return (
    <WidgetShell
      title="Kanban board"
      eyebrow="Task flow"
      icon={<FolderKanban className="size-4" />}
      action={
        <Button variant="ghost" size="sm" onClick={() => onNavigate({ name: "tasks" })}>
          Open board <ArrowUpRight />
        </Button>
      }
      className="lg:col-span-2"
    >
      <div className="grid gap-3 md:grid-cols-3">
        {columns.map((status) => {
          const tasks = data.openTasks.filter((task) => task.status === status).slice(0, 4);
          return (
            <div key={status} className="rounded-lg bg-surface/60 p-2.5">
              <div className="mb-2 flex items-center justify-between">
                <span className="text-[11px] font-semibold text-fg-muted capitalize">
                  {status.replace("_", " ")}
                </span>
                <Badge variant="secondary">{tasks.length}</Badge>
              </div>
              <div className="space-y-2">
                {tasks.map((task) => (
                  <div
                    key={task.id}
                    className="rounded-md border border-line bg-panel px-2.5 py-2 text-xs text-fg"
                  >
                    {task.title}
                  </div>
                ))}
                {tasks.length === 0 && (
                  <p className="py-3 text-center text-[11px] text-fg-faint">Empty</p>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </WidgetShell>
  );
}

export function ProjectActivityWidget({
  data,
  onNavigate,
}: {
  data: DashboardData;
  onNavigate: (route: Route) => void;
}) {
  const max = Math.max(...data.projectActivity.map((item) => item.trackedMs), 1);
  return (
    <WidgetShell
      title="Project activity"
      eyebrow="Last 30 days"
      icon={<FolderKanban className="size-4" />}
    >
      <div className="space-y-3">
        {data.projectActivity.map(({ project, trackedMs }) => (
          <button
            key={project?.id}
            type="button"
            onClick={() => onNavigate({ name: "projects", projectId: project?.id })}
            className="w-full text-left"
          >
            <div className="mb-1 flex justify-between gap-3 text-xs">
              <span className="truncate text-fg">{project?.name}</span>
              <span className="shrink-0 tabular-nums text-fg-muted">
                {formatDuration(trackedMs)}
              </span>
            </div>
            <div className="h-1.5 overflow-hidden rounded-full bg-surface">
              <div
                className="h-full rounded-full"
                style={{
                  width: `${(trackedMs / max) * 100}%`,
                  backgroundColor: project?.color || "var(--accent)",
                }}
              />
            </div>
          </button>
        ))}
        {data.projectActivity.length === 0 && <Empty label="No project activity yet." />}
      </div>
    </WidgetShell>
  );
}

export function RecentActivityWidget({
  data,
  onNavigate,
}: {
  data: DashboardData;
  onNavigate: (route: Route) => void;
}) {
  return (
    <WidgetShell
      title="Recent activity"
      eyebrow="Latest entries"
      icon={<Clock3 className="size-4" />}
      action={
        <Button variant="ghost" size="sm" onClick={() => onNavigate({ name: "entries" })}>
          View entries <ArrowUpRight />
        </Button>
      }
    >
      <div className="space-y-2">
        {data.recentEntries.map((entry) => (
          <button
            key={entry.id}
            type="button"
            onClick={() => onNavigate({ name: "entries" })}
            className="flex w-full items-center gap-3 rounded-lg px-2 py-2 text-left hover:bg-surface"
          >
            <span className="size-2 shrink-0 rounded-full bg-accent" />
            <span className="min-w-0 flex-1 truncate text-sm text-fg">
              {entry.description || "Untitled entry"}
            </span>
            <span className="shrink-0 text-[10px] text-fg-faint">
              {formatRelative(entry.startedAt, Date.now())}
            </span>
          </button>
        ))}
        {data.recentEntries.length === 0 && <Empty label="No time entries yet." />}
      </div>
    </WidgetShell>
  );
}

function Empty({ label }: { label: string }) {
  return (
    <div className="flex items-center gap-2 rounded-lg border border-dashed border-line p-3 text-xs text-fg-muted">
      <CheckCircle2 className="size-4 text-success" />
      {label}
    </div>
  );
}
