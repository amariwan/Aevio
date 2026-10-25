import { CalendarDays, ChevronLeft, ChevronRight, LayoutDashboard, RefreshCw } from "lucide-react";
import { useState, type ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { formatShortDate } from "@/lib/format";
import type { Route } from "@/lib/types";
import { useDashboardData } from "./useDashboardData";
import {
  ActivityWidget,
  KanbanWidget,
  MetricsWidget,
  NotificationsWidget,
  ProjectActivityWidget,
  QueueWidget,
  QuickAccessWidget,
  RecentActivityWidget,
  WidgetShell,
} from "./widgets/DashboardWidgets";
import { WidgetControls } from "./widgets/WidgetControls";
import { useWidgetLayout } from "./widgets/useWidgetLayout";
import type { DashboardWidgetId } from "./widgets/types";

type Period = "day" | "week" | "month" | "year";

const PERIODS: Period[] = ["day", "week", "month", "year"];

function dateLabel(date: Date): string {
  return date.toLocaleDateString(undefined, {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

function widgetClass(id: DashboardWidgetId): string {
  if (id === "metrics") return "lg:col-span-2";
  if (id === "activity") return "lg:col-span-3";
  if (id === "kanban") return "lg:col-span-2";
  if (id === "recent-activity") return "lg:col-span-2";
  return "";
}

export function Dashboard({ navigate }: { navigate: (route: Route) => void }) {
  const data = useDashboardData();
  const layout = useWidgetLayout();
  const [period, setPeriod] = useState<Period>("day");
  const [selectedDate, setSelectedDate] = useState(() => new Date());

  const stepDate = (amount: number) => {
    setSelectedDate((current) => {
      const next = new Date(current);
      next.setDate(next.getDate() + amount);
      return next;
    });
  };

  const widgets: Record<DashboardWidgetId, ReactNode> = {
    metrics: <MetricsWidget data={data} />,
    activity: <ActivityWidget data={data} />,
    queue: <QueueWidget data={data} onNavigate={navigate} />,
    notifications: <NotificationsWidget data={data} onNavigate={navigate} />,
    "quick-access": <QuickAccessWidget onNavigate={navigate} />,
    kanban: <KanbanWidget data={data} onNavigate={navigate} />,
    "project-activity": <ProjectActivityWidget data={data} onNavigate={navigate} />,
    "recent-activity": <RecentActivityWidget data={data} onNavigate={navigate} />,
  };

  return (
    <main className="relative isolate min-h-0 flex-1 overflow-auto bg-canvas px-4 py-5 text-fg sm:px-6 lg:px-8">
      <div className="pointer-events-none absolute -top-32 left-1/3 size-96 rounded-full bg-accent/5 blur-3xl" />
      <div className="pointer-events-none absolute top-1/2 -right-40 size-96 rounded-full bg-accent-dim/5 blur-3xl" />
      <div className="relative z-10 mx-auto flex max-w-[1500px] flex-col gap-5">
        <header className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div>
            <p className="mb-3 inline-flex items-center gap-2 rounded-full border border-accent/20 bg-accent-soft/60 px-3 py-1 text-xs font-medium text-accent">
              <span className="size-1.5 rounded-full bg-accent shadow-[0_0_10px_var(--accent)]" />
              Today. Nice day ☀️
            </p>
            <h1 className="bg-gradient-to-r from-fg-strong via-fg-strong to-accent bg-clip-text text-3xl font-semibold tracking-tight text-transparent sm:text-4xl">
              {dateLabel(selectedDate)}
            </h1>
            <p className="mt-2 text-sm text-fg-muted">
              Your local activity, focus, and work queue in one view.
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <div className="flex rounded-full bg-panel/70 p-1 shadow-lg ring-1 ring-line/80 backdrop-blur-xl">
              {PERIODS.map((item) => (
                <Button
                  key={item}
                  className="rounded-full px-4 capitalize"
                  onClick={() => setPeriod(item)}
                  size="sm"
                  variant={period === item ? "secondary" : "ghost"}
                >
                  {item}
                </Button>
              ))}
            </div>
            <div className="flex items-center gap-1 rounded-full bg-panel/70 p-1 shadow-lg ring-1 ring-line/80 backdrop-blur-xl">
              <Button
                aria-label="Previous day"
                onClick={() => stepDate(-1)}
                size="icon"
                variant="ghost"
              >
                <ChevronLeft aria-hidden="true" />
              </Button>
              <Button
                aria-label="Open calendar"
                onClick={() => navigate({ name: "calendar" })}
                size="icon"
                variant="ghost"
              >
                <CalendarDays aria-hidden="true" />
              </Button>
              <Button aria-label="Next day" onClick={() => stepDate(1)} size="icon" variant="ghost">
                <ChevronRight aria-hidden="true" />
              </Button>
            </div>
            <Button
              aria-label="Refresh dashboard"
              disabled={data.loading}
              onClick={() => void data.reload()}
              size="icon"
              variant="outline"
            >
              <RefreshCw aria-hidden="true" className={data.loading ? "animate-spin" : undefined} />
            </Button>
            <WidgetControls
              layout={layout.layout}
              onMove={layout.move}
              onReset={layout.reset}
              onToggle={layout.toggle}
            />
          </div>
        </header>

        {data.error && (
          <p className="rounded-xl border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger">
            {data.error}
          </p>
        )}

        <div className="grid gap-4 lg:grid-cols-3">
          {layout.visibleIds.map((id) => (
            <div key={`${id}-${period}`} className={widgetClass(id)}>
              {widgets[id]}
            </div>
          ))}
        </div>

        <WidgetShell
          title="Dashboard snapshot"
          eyebrow="Aevio workspace"
          icon={<LayoutDashboard className="size-4" />}
        >
          <div className="flex flex-wrap items-center justify-between gap-3 text-sm text-fg-muted">
            <span>
              {formatShortDate(selectedDate.getTime(), Date.now())} · {period} view
            </span>
            <span>
              {data.metrics.activeProjects} active projects · {data.metrics.openTasks} open tasks
            </span>
          </div>
        </WidgetShell>
      </div>
    </main>
  );
}
