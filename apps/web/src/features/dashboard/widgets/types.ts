import type { ReactNode } from "react";

export type DashboardWidgetId =
  | "metrics"
  | "activity"
  | "queue"
  | "notifications"
  | "quick-access"
  | "kanban"
  | "project-activity"
  | "recent-activity";

export interface DashboardWidgetDefinition {
  id: DashboardWidgetId;
  label: string;
  description: string;
  defaultVisible: boolean;
  content: ReactNode;
}

export interface DashboardWidgetLayout {
  order: DashboardWidgetId[];
  hidden: DashboardWidgetId[];
}
