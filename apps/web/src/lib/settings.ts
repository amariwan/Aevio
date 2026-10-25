/**
 * User preferences, mirroring `Settings` in src-tauri/src/settings.rs.
 *
 * The palette is the ONE place appearance choices are defined. `applyAppearance`
 * writes the selected theme onto `:root` as CSS variables, and every Tailwind utility
 * (`bg-accent`, `text-accent`, `border-accent/30`, ...) reads those variables -
 * so changing a hex here repaints the entire app, no component edits.
 */

export type Palette =
  | "zinc"
  | "forest"
  | "ocean"
  | "violet"
  | "sunset"
  | "mono"
  | "claude"
  | "neobrutualism"
  | "supabase"
  | "vercel"
  | "notebook"
  | "light-green"
  | "zen"
  | "astro-vista"
  | "whatsapp";
export type Mode = "system" | "light" | "dark";
export type Language = "system" | "de" | "en";
export type Shape = "rounded" | "sharper";
export type SizeMode = "compact" | "normal" | "relaxed" | "veryRelaxed";
export type CloseBehavior = "quit" | "hide";
/** What the AI suggests for each entry (Aevio's "Suggestion level"). */
export type AiSuggest = "category" | "categoryProject";

export interface DaySchedule {
  enabled: boolean;
  start: string;
  end: string;
}

export interface TrackingHours {
  enabled: boolean;
  perDay: boolean;
  defaultStart: string;
  defaultEnd: string;
  monday: DaySchedule;
  tuesday: DaySchedule;
  wednesday: DaySchedule;
  thursday: DaySchedule;
  friday: DaySchedule;
  saturday: DaySchedule;
  sunday: DaySchedule;
}

export const DEFAULT_DAY_SCHEDULE: DaySchedule = {
  enabled: true,
  start: "07:00",
  end: "19:00",
};

export const DEFAULT_TRACKING_HOURS: TrackingHours = {
  enabled: true,
  perDay: false,
  defaultStart: "07:00",
  defaultEnd: "19:00",
  monday: { ...DEFAULT_DAY_SCHEDULE },
  tuesday: { ...DEFAULT_DAY_SCHEDULE },
  wednesday: { ...DEFAULT_DAY_SCHEDULE },
  thursday: { ...DEFAULT_DAY_SCHEDULE },
  friday: { ...DEFAULT_DAY_SCHEDULE },
  saturday: { ...DEFAULT_DAY_SCHEDULE },
  sunday: { ...DEFAULT_DAY_SCHEDULE },
};

const DAY_KEYS = [
  "sunday",
  "monday",
  "tuesday",
  "wednesday",
  "thursday",
  "friday",
  "saturday",
] as const;

/** `"07:30"` -> 450, or undefined when malformed. */
function minutesOf(clock: string): number | undefined {
  const [hours, minutes] = clock.split(":").map(Number);
  if (hours === undefined || minutes === undefined) return undefined;
  if (Number.isNaN(hours) || Number.isNaN(minutes)) return undefined;
  return hours * 60 + minutes;
}

/**
 * A day's tracking window in minutes after midnight, or null when that day
 * does not track. `end` below `start` means the window runs past midnight.
 */
function dayWindow(th: TrackingHours, date: Date): { start: number; end: number } | null {
  const dayKey = DAY_KEYS[date.getDay()];
  if (!dayKey) return null;
  const [start, end, enabled] = th.perDay
    ? [th[dayKey].start, th[dayKey].end, th[dayKey].enabled]
    : [th.defaultStart, th.defaultEnd, true];
  if (!enabled || start === end) return null;
  const startMins = minutesOf(start);
  const endMins = minutesOf(end);
  if (startMins === undefined || endMins === undefined) return null;
  return { start: startMins, end: endMins };
}

/** Whether the given date is inside the configured tracking hours window. */
export function isInsideTrackingHours(th: TrackingHours, date: Date = new Date()): boolean {
  if (!th.enabled) return true;
  const window = dayWindow(th, date);
  if (!window) return false;
  const curMins = date.getHours() * 60 + date.getMinutes();
  if (window.start < window.end) {
    return curMins >= window.start && curMins < window.end;
  }
  return curMins >= window.start || curMins < window.end;
}

/** When the schedule next starts tracking, within a week; null if never. */
export function nextTrackingStart(th: TrackingHours, now: Date = new Date()): Date | null {
  if (!th.enabled) return null;
  for (let offset = 0; offset <= 7; offset++) {
    const day = new Date(now);
    day.setDate(day.getDate() + offset);
    const window = dayWindow(th, day);
    if (!window) continue;
    day.setHours(0, window.start, 0, 0);
    if (day.getTime() > now.getTime()) return day;
  }
  return null;
}

export interface Weekdays {
  mon: boolean;
  tue: boolean;
  wed: boolean;
  thu: boolean;
  fri: boolean;
  sat: boolean;
  sun: boolean;
}

/** A fixed-time recurring break such as "Lunch 12:30, 45 min, weekdays". */
export interface ScheduledBreak {
  id: string;
  label: string;
  /** `HH:MM`, local time. */
  at: string;
  minutes: number;
  days: Weekdays;
  enabled: boolean;
}

/** Interval reminders have no days: tracking hours decide when they fire. */
export interface BreakSettings {
  enabled: boolean;
  workMinutes: number;
  breakMinutes: number;
  /** What the reminder's snooze button snoozes for; 5, 10 or 15. */
  snoozeMinutes: number;
  /** An extra line on the reminder. Empty shows nothing. */
  message: string;
  /** Pause running stopwatches for a break and resume them after. */
  pauseStopwatches: boolean;
  chime: boolean;
  schedules: ScheduledBreak[];
}

export const WEEKDAY_KEYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] as const;

export const WEEKDAYS_MON_FRI: Weekdays = {
  mon: true,
  tue: true,
  wed: true,
  thu: true,
  fri: true,
  sat: false,
  sun: false,
};

export const MAX_BREAK_MESSAGE_CHARS = 120;
export const MAX_SCHEDULED_BREAKS = 12;

export const DEFAULT_BREAK_SETTINGS: BreakSettings = {
  enabled: true,
  workMinutes: 50,
  breakMinutes: 5,
  snoozeMinutes: 5,
  message: "",
  pauseStopwatches: false,
  chime: false,
  schedules: [],
};

export type NotificationPlacement =
  | "topLeft"
  | "topRight"
  | "topCenter"
  | "bottomLeft"
  | "bottomRight";

export interface StopwatchReminder {
  enabled: boolean;
  /** Current run length; accumulated time never counts. */
  afterMinutes: number;
}

export interface Settings {
  palette: Palette;
  mode: Mode;
  language: Language;
  shape: Shape;
  sizeMode: SizeMode;
  closeBehavior: CloseBehavior;
  trayEnabled: boolean;
  /** Activity history older than this many days. 0 = keep forever. */
  retentionDays: number;
  aiSuggest: AiSuggest;
  /** Approve entries whose every suggested field clears the threshold. */
  autoAccept: boolean;
  autoAcceptPercent: number;
  /** Appended to the on-device model's instructions. */
  aiCustomPrompt: string;
  /** AI provider: "none", "ollama", or "openai". */
  aiProvider: string;
  /** Base URL for the AI provider (e.g. http://localhost:11434). */
  aiBaseUrl: string;
  /** Model name for the AI provider. */
  aiModel: string;
  /** Optional API key for the AI provider. */
  aiApiKey: string;
  /** Expected work hours per week; a day's target is a fifth of it. */
  weeklyTargetHours: number;
  trackingHours: TrackingHours;
  breaks: BreakSettings;
  stopwatchReminder: StopwatchReminder;
  notificationPlacement: NotificationPlacement;
}

export const DEFAULT_SETTINGS: Settings = {
  palette: "zinc",
  mode: "system",
  language: "system",
  shape: "rounded",
  sizeMode: "normal",
  closeBehavior: "hide",
  trayEnabled: true,
  retentionDays: 0,
  aiSuggest: "categoryProject",
  autoAccept: true,
  autoAcceptPercent: 95,
  aiCustomPrompt: "",
  aiProvider: "",
  aiBaseUrl: "",
  aiModel: "",
  aiApiKey: "",
  weeklyTargetHours: 40,
  trackingHours: DEFAULT_TRACKING_HOURS,
  breaks: DEFAULT_BREAK_SETTINGS,
  stopwatchReminder: { enabled: true, afterMinutes: 180 },
  notificationPlacement: "bottomRight",
};

/** A working day's share of the weekly target, in milliseconds. */
export function dailyTargetMs(settings: Settings): number {
  return (settings.weeklyTargetHours / 5) * 3_600_000;
}

export function weeklyTargetMs(settings: Settings): number {
  return settings.weeklyTargetHours * 3_600_000;
}

/**
 * Expected work time for a Calendar-style range: a day's or a week's share of
 * the weekly target, or for a month the weekly target prorated by its days.
 */
export function targetMsFor(
  settings: Settings,
  scale: "day" | "week" | "month",
  days: number,
): number {
  if (scale === "day") return dailyTargetMs(settings);
  if (scale === "week") return weeklyTargetMs(settings);
  return weeklyTargetMs(settings) * (days / 7);
}

/** A target in hours: "7.5h" for a short one, whole hours ("171h") above 10. */
export function formatTargetHours(ms: number): string {
  const hours = ms / 3_600_000;
  return `${hours >= 10 ? Math.round(hours) : Math.round(hours * 10) / 10}h`;
}

export interface PaletteMeta {
  label: string;
  /** Key colors shown as dots on the palette tile. */
  colors: string[];
}

export const PALETTES: Record<Palette, PaletteMeta> = {
  zinc: {
    label: "Zinc",
    colors: ["#dde2df", "#252827", "#5bcf8f", "#191c1b"],
  },
  forest: {
    label: "Forest",
    colors: ["#d4e0d4", "#1c261c", "#4ade80", "#10160f"],
  },
  ocean: {
    label: "Ocean",
    colors: ["#c8d4e0", "#182030", "#60a5fa", "#0c1119"],
  },
  violet: {
    label: "Violet",
    colors: ["#d8cce4", "#1e1628", "#a78bfa", "#120c19"],
  },
  sunset: {
    label: "Sunset",
    colors: ["#e4d4c4", "#2a1e14", "#fb923c", "#191209"],
  },
  mono: {
    label: "Mono",
    colors: ["#e0e0e0", "#1a1a1a", "#cccccc", "#0d0d0d"],
  },
  claude: {
    label: "Claude",
    colors: ["#f5f0e8", "#2b2926", "#c46b3c", "#181716"],
  },
  neobrutualism: {
    label: "Neobrutalism",
    colors: ["#ffffff", "#000000", "#f04438", "#f5d90a"],
  },
  supabase: {
    label: "Supabase",
    colors: ["#f7faf9", "#17201d", "#3ecf8e", "#101413"],
  },
  vercel: {
    label: "Vercel",
    colors: ["#ffffff", "#171717", "#171717", "#000000"],
  },
  notebook: {
    label: "Notebook",
    colors: ["#faf9f6", "#464646", "#f0d98b", "#292929"],
  },
  "light-green": {
    label: "Light Green",
    colors: ["#f7faf3", "#182231", "#b7e33c", "#101924"],
  },
  zen: {
    label: "Zen",
    colors: ["#e9e4d8", "#3f3c37", "#d58d55", "#252525"],
  },
  "astro-vista": {
    label: "Astro Vista",
    colors: ["#eef1f4", "#30343a", "#d77f45", "#232629"],
  },
  whatsapp: {
    label: "WhatsApp",
    colors: ["#f4f6f8", "#24323a", "#25d366", "#17252c"],
  },
};

export const PALETTE_ORDER = Object.keys(PALETTES) as Palette[];

export function isPalette(value: unknown): value is Palette {
  return typeof value === "string" && PALETTE_ORDER.includes(value as Palette);
}

export const MODE_ORDER: Mode[] = ["system", "light", "dark"];

/** Mirror of `LoginItemState` in src-tauri/src/login_item.rs. */
export type LoginItemState = "enabled" | "disabled" | "requiresApproval" | "unsupported";

export interface StoragePaths {
  configFile: string;
  dataDir: string;
  /** Trackers and activity share this one SQLite file (decision A8). */
  databaseFile: string;
}

/**
 * Where the saved palette, mode, and shape are mirrored for index.html's
 * pre-paint script, so a Forest-Light or Sharper user does not see the defaults
 * flash before settings load. Rust seeds a missing copy before the page runs
 * (`appearance_init_script` in api/src/settings/mod.rs), so keep the key
 * names in sync with it.
 */
const PALETTE_HINT_KEY = "aevio.theme";
const MODE_HINT_KEY = "aevio.mode";
const SHAPE_HINT_KEY = "aevio.shape";

/** Which concrete variant a mode preference means right now. */
export function resolveMode(mode: Mode): "light" | "dark" {
  if (mode !== "system") return mode;
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

/** Map a legacy `accent` value to its palette. Unknown values fall back to zinc. */
function paletteFromAccent(accent: string): Palette {
  switch (accent) {
    case "green":
      return "forest";
    case "blue":
      return "ocean";
    case "purple":
      return "violet";
    case "orange":
      return "sunset";
    default:
      return "zinc";
  }
}

/** Map a legacy `theme` value to a mode. Unknown values fall back to system. */
function modeFromTheme(theme: string): Mode {
  if (theme === "light" || theme === "dark") return theme;
  return "system";
}

/**
 * Migrate a legacy settings object (with `theme` + `accent`) to the new
 * `palette` + `mode` shape. Accepts a parsed JSON object and returns a
 * complete Settings with defaults filled in.
 */
export function migrateSettings(
  raw: Partial<Settings> & { theme?: string; accent?: string },
): Settings {
  const palette = isPalette(raw.palette)
    ? raw.palette
    : raw.accent
      ? paletteFromAccent(raw.accent)
      : "zinc";
  const mode = raw.mode ? (raw.mode as Mode) : raw.theme ? modeFromTheme(raw.theme) : "system";
  const language = raw.language === "de" || raw.language === "en" ? raw.language : "system";
  return { ...DEFAULT_SETTINGS, ...raw, palette, mode, language };
}

/**
 * Paints the current preference onto the document. Palette is an attribute
 * (`[data-theme]`) so the CSS variable overrides in themes.css apply; mode
 * picks the light/dark variant within the palette.
 */
export function applyAppearance(settings: Settings): void {
  const root = document.documentElement;
  const palette = isPalette(settings.palette) ? settings.palette : "zinc";
  const mode = resolveMode(settings.mode);
  try {
    localStorage.setItem(PALETTE_HINT_KEY, palette);
    localStorage.setItem(MODE_HINT_KEY, settings.mode);
    localStorage.setItem(SHAPE_HINT_KEY, settings.shape);
  } catch {
    // Storage can be unavailable; the hint only avoids a startup flash.
  }
  root.dataset.theme = palette;
  // Tells the browser to render native widgets (scrollbars, form controls)
  // for the active scheme.
  root.style.colorScheme = mode;
  // Always expose the resolved mode. Theme files use data-mode selectors and
  // need a concrete value even when the preference follows the OS.
  root.dataset.mode = mode;
  root.dataset.shape = settings.shape;
  root.dataset.sizeMode = settings.sizeMode;
}
