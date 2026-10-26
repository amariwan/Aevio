import type { LucideIcon } from "lucide-react";
import {
  ActivityIcon,
  CalendarDaysIcon,
  ClockIcon,
  FolderIcon,
  ListTodoIcon,
  PlayIcon,
  TagIcon,
  UsersIcon,
} from "lucide-react";
import type { MessageKey } from "@/lib/i18n";

export type CalendarColumn =
  | "activity"
  | "timeEntries"
  | "labels"
  | "tasks"
  | "projects"
  | "clients"
  | "sessions"
  | "calendar";

export type CalendarColumns = Record<CalendarColumn, boolean>;

export type CalendarColumnOrder = CalendarColumn[];

export interface CalendarLayout {
  order: CalendarColumnOrder;
  visible: CalendarColumns;
}

export const DEFAULT_COLUMN_ORDER: CalendarColumnOrder = [
  "activity",
  "timeEntries",
  "labels",
  "tasks",
  "sessions",
  "calendar",
  "projects",
  "clients",
];

export const DEFAULT_CALENDAR_COLUMNS: CalendarColumns = {
  activity: true,
  timeEntries: true,
  labels: true,
  tasks: true,
  projects: false,
  clients: false,
  sessions: true,
  calendar: true,
};

export const DEFAULT_LAYOUT: CalendarLayout = {
  order: [...DEFAULT_COLUMN_ORDER],
  visible: { ...DEFAULT_CALENDAR_COLUMNS },
};

function defaultLayout(): CalendarLayout {
  return {
    order: [...DEFAULT_COLUMN_ORDER],
    visible: { ...DEFAULT_CALENDAR_COLUMNS },
  };
}

/** Columns offered in the timeline settings, in display order. */
export const COLUMN_KEYS: readonly CalendarColumn[] = DEFAULT_COLUMN_ORDER;

/** i18n key for each column's display label. */
export const COLUMN_LABEL_KEY: Record<CalendarColumn, MessageKey> = {
  activity: "nav.apps",
  timeEntries: "nav.timeEntries",
  labels: "issues.labels",
  tasks: "tasks.tasks",
  sessions: "focus.sessions",
  calendar: "nav.calendar",
  projects: "nav.projects",
  clients: "nav.clients",
};

/** Icon for each column. */
export const COLUMN_ICON: Record<CalendarColumn, LucideIcon> = {
  activity: ActivityIcon,
  timeEntries: ClockIcon,
  labels: TagIcon,
  tasks: ListTodoIcon,
  projects: FolderIcon,
  clients: UsersIcon,
  sessions: PlayIcon,
  calendar: CalendarDaysIcon,
};

/** Resolves a column's localized label. */
export function columnLabel(column: CalendarColumn, t: (key: MessageKey) => string): string {
  return t(COLUMN_LABEL_KEY[column]);
}

const STORAGE_KEY = "aevio.calendar.columns";

function isValidColumn(value: unknown): value is CalendarColumn {
  return typeof value === "string" && (DEFAULT_COLUMN_ORDER as readonly string[]).includes(value);
}

export function readCalendarLayout(): CalendarLayout {
  if (typeof window === "undefined") return defaultLayout();
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return defaultLayout();
    const parsed: unknown = JSON.parse(raw);

    // Migration: old flat visibility record { activity: true, ... }.
    if (
      parsed &&
      typeof parsed === "object" &&
      !Array.isArray(parsed) &&
      !("order" in (parsed as Record<string, unknown>)) &&
      !("visible" in (parsed as Record<string, unknown>))
    ) {
      const visible = { ...DEFAULT_CALENDAR_COLUMNS, ...(parsed as Partial<CalendarColumns>) };
      return { order: [...DEFAULT_COLUMN_ORDER], visible };
    }

    const obj = parsed as { order?: unknown; visible?: unknown };
    const orderRaw = Array.isArray(obj?.order) ? obj.order : [];
    const order = [
      ...orderRaw.filter(isValidColumn),
      ...DEFAULT_COLUMN_ORDER.filter((c) => !orderRaw.includes(c)),
    ] as CalendarColumn[];
    const visible = {
      ...DEFAULT_CALENDAR_COLUMNS,
      ...((obj?.visible && typeof obj.visible === "object"
        ? obj.visible
        : {}) as Partial<CalendarColumns>),
    };
    return { order, visible };
  } catch {
    return defaultLayout();
  }
}

export function writeCalendarLayout(layout: CalendarLayout): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
  } catch {
    // Storage can be unavailable in an embedded/private webview.
  }
}
