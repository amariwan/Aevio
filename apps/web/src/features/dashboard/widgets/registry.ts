import type { DashboardWidgetId } from "./types";

export const DASHBOARD_WIDGET_REGISTRY: Record<
  DashboardWidgetId,
  { label: string; description: string; defaultVisible: boolean }
> = {
  metrics: {
    label: "Summary metrics",
    description: "Tracked time, projects, tasks and outstanding invoices.",
    defaultVisible: true,
  },
  activity: {
    label: "Activity overview",
    description: "Seven-day tracked-time chart.",
    defaultVisible: true,
  },
  queue: {
    label: "Work queue",
    description: "Your highest-priority open tasks.",
    defaultVisible: true,
  },
  notifications: {
    label: "Notifications",
    description: "Reviews, blockers and overdue work.",
    defaultVisible: true,
  },
  "quick-access": {
    label: "Quick access",
    description: "Shortcuts to the rest of the workspace.",
    defaultVisible: true,
  },
  kanban: {
    label: "Kanban board",
    description: "Drag tasks between status columns.",
    defaultVisible: true,
  },
  "project-activity": {
    label: "Project activity",
    description: "Time distribution across active projects.",
    defaultVisible: true,
  },
  "recent-activity": {
    label: "Recent activity",
    description: "Latest time entries from Aevio.",
    defaultVisible: true,
  },
};

export const DASHBOARD_WIDGET_IDS = Object.keys(DASHBOARD_WIDGET_REGISTRY) as DashboardWidgetId[];
