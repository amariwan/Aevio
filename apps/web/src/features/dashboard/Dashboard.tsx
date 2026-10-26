import {
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  MoreHorizontal,
  Clock,
  Target,
  Zap,
  ListChecks,
  Sun,
} from "lucide-react";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip as RechartsTooltip,
  XAxis,
  YAxis,
} from "recharts";
import { MotionConfig, motion, useReducedMotion } from "motion/react";
import { AnimatedCounter } from "@/components/charts";
import { BarRow, Donut, type Slice } from "@/components/data-viz/Charts";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { formatDuration } from "@/lib/format";
import { useI18n } from "@/hooks/useI18n";
import type { MessageKey } from "@/lib/i18n";
import type { Route, TimeEntry } from "@/lib/types";
import { useDashboardData } from "./useDashboardData";

type Period = "day" | "week" | "month" | "year";
const PERIODS: Period[] = ["day", "week", "month", "year"];

function duration(entry: TimeEntry): number {
  return Math.max(0, entry.endedAt - entry.startedAt);
}
function keyFor(date: Date): string {
  return date.toISOString().slice(0, 10);
}
function periodBounds(date: Date, period: Period): { start: Date; end: Date } {
  const start = new Date(date);
  start.setHours(0, 0, 0, 0);
  if (period === "week")
    start.setDate(start.getDate() + (start.getDay() === 0 ? -6 : 1 - start.getDay()));
  if (period === "month") start.setDate(1);
  if (period === "year") start.setMonth(0, 1);
  const end = new Date(start);
  if (period === "day") end.setDate(end.getDate() + 1);
  if (period === "week") end.setDate(end.getDate() + 7);
  if (period === "month") end.setMonth(end.getMonth() + 1);
  if (period === "year") end.setFullYear(end.getFullYear() + 1);
  return { start, end };
}
function periodTarget(period: Period, date: Date): number {
  const day = 7.5 * 60 * 60 * 1000;
  if (period === "day") return day;
  if (period === "week") return day * 5;
  if (period === "month")
    return day * new Date(date.getFullYear(), date.getMonth() + 1, 0).getDate();
  return day * 260;
}
function changeLabel(current: number, previous: number): string {
  if (previous === 0) return current > 0 ? "+100% vs previous" : "No change";
  const change = Math.round(((current - previous) / previous) * 100);
  return `${change > 0 ? "+" : ""}${change}% vs previous`;
}
function labelFor(date: Date): string {
  const weekday = date.toLocaleDateString(undefined, { weekday: "long" });
  const rest = date.toLocaleDateString(undefined, {
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  return `${weekday}, ${rest}`;
}
function nameFor(value: string): string {
  return value.replace(/[-_]/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}
function More() {
  const { t } = useI18n();
  return (
    <Button aria-label={t("dashboard.moreOptions")} size="icon-sm" variant="ghost">
      <MoreHorizontal aria-hidden="true" />
    </Button>
  );
}

function Reveal({
  children,
  delay = 0,
  className = "",
}: {
  children: ReactNode;
  delay?: number;
  className?: string;
}) {
  const reduceMotion = useReducedMotion() ?? false;
  return (
    <motion.div
      animate={{ opacity: 1, y: 0, scale: 1 }}
      className={className}
      initial={reduceMotion ? undefined : { opacity: 0, y: 16, scale: 0.98 }}
      transition={{ delay, duration: 0.5, ease: [0.22, 1, 0.36, 1] }}
    >
      {children}
    </motion.div>
  );
}

function Stat({
  label,
  value,
  detail,
  icon,
  children,
}: {
  label: string;
  value: ReactNode;
  detail?: ReactNode;
  icon?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <motion.div whileHover={{ y: -2 }} transition={{ duration: 0.2 }}>
      <Card className="border-line/70 bg-panel/75 shadow-none transition-shadow hover:shadow-md">
        <CardContent className="flex min-h-32 items-end justify-between gap-3 p-5">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              {icon && <span className="text-fg-muted">{icon}</span>}
              <p className="text-sm text-fg-muted">{label}</p>
            </div>
            <p className="mt-4 text-[2.6rem] font-medium leading-none tracking-[-0.06em] tabular-nums text-fg-strong">
              {value}
            </p>
            {detail && <p className="mt-2 text-xs text-fg-faint">{detail}</p>}
          </div>
          {children}
        </CardContent>
      </Card>
    </motion.div>
  );
}

function aggregate(entries: TimeEntry[], getName: (entry: TimeEntry) => string) {
  const totals = new Map<string, number>();
  for (const entry of entries)
    totals.set(getName(entry), (totals.get(getName(entry)) ?? 0) + duration(entry));
  return [...totals]
    .map(([name, ms]) => ({ name, ms }))
    .sort((a, b) => b.ms - a.ms)
    .slice(0, 5);
}

function Timeline({ entries }: { entries: TimeEntry[] }) {
  const { t } = useI18n();
  const reduceMotion = useReducedMotion() ?? false;
  const start = new Date();
  start.setHours(6, 0, 0, 0);
  const end = start.getTime() + 12 * 60 * 60 * 1000;
  const marks = Array.from(
    { length: 13 },
    (_, index) => `${index + 6}${index + 6 >= 12 ? "pm" : "am"}`,
  );
  return (
    <Card className="border-line/70 bg-panel/75 shadow-none">
      <CardHeader className="flex flex-row items-center justify-between pb-4">
        <CardTitle className="text-xl text-fg-strong">{t("dashboard.timeline")}</CardTitle>
        <More />
      </CardHeader>
      <CardContent>
        <div className="relative h-36 overflow-hidden rounded-sm border border-line bg-canvas/60">
          <div className="absolute inset-0 grid grid-cols-12">
            {Array.from({ length: 12 }, (_, index) => (
              <span className="border-r border-line/70" key={index} />
            ))}
          </div>
          {entries.map((entry) => {
            const left = Math.max(
              0,
              ((entry.startedAt - start.getTime()) / (end - start.getTime())) * 100,
            );
            const width = Math.max(
              1.4,
              ((Math.min(entry.endedAt, end) - Math.max(entry.startedAt, start.getTime())) /
                (end - start.getTime())) *
                100,
            );
            return (
              <motion.span
                className="absolute top-7 h-20 rounded-[2px] bg-accent shadow-sm"
                initial={reduceMotion ? { scaleY: 1 } : { scaleY: 0 }}
                key={entry.id}
                animate={{ scaleY: 1 }}
                style={{ left: `${left}%`, width: `${width}%` }}
                title={entry.description || entry.dominantApp || t("dashboard.trackedTime")}
                transition={{ delay: 0.2 + left / 500, duration: 0.65, ease: [0.22, 1, 0.36, 1] }}
                whileHover={{ scaleY: 1.05, filter: "brightness(1.15)" }}
              />
            );
          })}
        </div>
        <div className="mt-2 flex justify-between text-[10px] text-fg-faint">
          {marks.map((mark) => (
            <span key={mark}>{mark}</span>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function Ranking({
  title,
  items,
  total,
  colors,
}: {
  title: string;
  items: Array<{ name: string; ms: number }>;
  total: number;
  colors: string[];
}) {
  const { t } = useI18n();
  const maxMs = items[0]?.ms ?? 0;
  return (
    <Card className="border-line/70 bg-panel/75 shadow-none transition-shadow hover:shadow-md">
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-xl text-fg-strong">{title}</CardTitle>
        <More />
      </CardHeader>
      <CardContent className="flex flex-col gap-4 pt-3">
        {items.map((item, index) => (
          <motion.div
            key={item.name}
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: index * 0.06, duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
          >
            <BarRow
              label={
                <span className="flex min-w-0 items-center gap-2">
                  <span
                    className="size-2 shrink-0 rounded-full"
                    style={{ backgroundColor: colors[index % colors.length] }}
                  />
                  <span className="truncate">{nameFor(item.name)}</span>
                </span>
              }
              large
              maxMs={maxMs}
              ms={item.ms}
              color={colors[index % colors.length]}
              trailing={
                <span className="tabular-nums">
                  {Math.round((item.ms / Math.max(total, 1)) * 100)}% · {formatDuration(item.ms)}
                </span>
              }
            />
          </motion.div>
        ))}
        {items.length === 0 && (
          <p className="py-8 text-sm text-fg-muted">{t("dashboard.noTrackedActivity")}</p>
        )}
      </CardContent>
    </Card>
  );
}

function ActivityChart({ data }: { data: Array<{ day: string; tracked: number }> }) {
  const { t } = useI18n();
  const reduceMotion = useReducedMotion() ?? false;
  return (
    <Card className="border-line/70 bg-panel/75 shadow-none">
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <div>
          <CardTitle className="text-xl text-fg-strong">{t("dashboard.activity")}</CardTitle>
          <p className="text-xs text-fg-faint">{t("dashboard.trackedMinutesSelectedPeriod")}</p>
        </div>
        <More />
      </CardHeader>
      <CardContent>
        <div className="h-64 w-full">
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={data} margin={{ top: 8, right: 8, left: -16, bottom: 0 }}>
              <defs>
                <linearGradient id="activityFill" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="var(--accent)" stopOpacity={0.35} />
                  <stop offset="100%" stopColor="var(--accent)" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid stroke="var(--line)" strokeDasharray="3 3" vertical={false} />
              <XAxis
                dataKey="day"
                tick={{ fill: "var(--fg-faint)", fontSize: 11 }}
                tickLine={false}
                axisLine={false}
              />
              <YAxis
                tick={{ fill: "var(--fg-faint)", fontSize: 11 }}
                tickLine={false}
                axisLine={false}
                width={32}
              />
              <RechartsTooltip
                contentStyle={{
                  background: "var(--panel)",
                  border: "1px solid var(--line)",
                  borderRadius: 8,
                  fontSize: 12,
                }}
                cursor={{ stroke: "var(--line-strong)" }}
              />
              <Area
                type="monotone"
                dataKey="tracked"
                stroke="var(--accent)"
                strokeWidth={2}
                fill="url(#activityFill)"
                isAnimationActive={!reduceMotion}
                animationDuration={900}
                animationEasing="ease-out"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </CardContent>
    </Card>
  );
}

export function Dashboard({ navigate }: { navigate: (route: Route) => void }) {
  const { t } = useI18n();
  const data = useDashboardData();
  const [period, setPeriod] = useState<Period>("day");
  const [date, setDate] = useState(() => new Date());
  const dateKey = keyFor(date);
  const dayEntries = useMemo(
    () => data.entries.filter((entry) => keyFor(new Date(entry.startedAt)) === dateKey),
    [data.entries, dateKey],
  );
  const bounds = useMemo(() => periodBounds(date, period), [date, period]);
  const previousBounds = useMemo(() => {
    const span = bounds.end.getTime() - bounds.start.getTime();
    return { start: new Date(bounds.start.getTime() - span), end: bounds.start };
  }, [bounds]);
  const entries = useMemo(
    () =>
      data.entries.filter(
        (entry) =>
          entry.startedAt >= bounds.start.getTime() && entry.startedAt < bounds.end.getTime(),
      ),
    [bounds.end, bounds.start, data.entries],
  );
  const previousEntries = useMemo(
    () =>
      data.entries.filter(
        (entry) =>
          entry.startedAt >= previousBounds.start.getTime() &&
          entry.startedAt < previousBounds.end.getTime(),
      ),
    [data.entries, previousBounds.end, previousBounds.start],
  );
  const total = entries.reduce((sum, entry) => sum + duration(entry), 0);
  const previousTotal = previousEntries.reduce((sum, entry) => sum + duration(entry), 0);
  const target = periodTarget(period, date);
  const categories = aggregate(
    entries,
    (entry) => data.catalog.categoryById.get(entry.categoryId ?? "")?.name ?? "Uncategorized",
  );
  const apps = aggregate(entries, (entry) => entry.dominantApp ?? "Other");
  const projects = aggregate(
    entries,
    (entry) => data.catalog.projectById.get(entry.projectId ?? "")?.name ?? "No project",
  );
  const summarySlices: Slice[] = categories.map((item, index) => ({
    key: item.name,
    label: item.name,
    color: `var(--chart-${(index % 5) + 1})`,
    ms: item.ms,
  }));

  const activityData = useMemo(() => {
    const buckets = new Map<string, number>();
    const span = bounds.end.getTime() - bounds.start.getTime();
    const days = Math.max(1, Math.round(span / 86_400_000));
    for (let i = 0; i < days; i++) {
      const d = new Date(bounds.start.getTime() + i * 86_400_000);
      buckets.set(d.toLocaleDateString(undefined, { month: "short", day: "numeric" }), 0);
    }
    for (const entry of entries) {
      const d = new Date(entry.startedAt);
      const key = d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
      if (buckets.has(key)) buckets.set(key, (buckets.get(key) ?? 0) + duration(entry));
    }
    return [...buckets.entries()].map(([day, ms]) => ({
      day,
      tracked: Math.round(ms / 60_000),
    }));
  }, [bounds.end, bounds.start, entries]);

  const shift = (amount: number) =>
    setDate((current) => {
      const next = new Date(current);
      if (period === "year") next.setFullYear(next.getFullYear() + amount);
      else if (period === "month") next.setMonth(next.getMonth() + amount);
      else next.setDate(next.getDate() + amount * (period === "week" ? 7 : 1));
      return next;
    });
  useEffect(() => {
    const interval = window.setInterval(() => void data.reload(), 30_000);
    return () => window.clearInterval(interval);
  }, [data.reload]);
  return (
    <MotionConfig reducedMotion="user">
      <main className="min-h-0 flex-1 overflow-auto bg-canvas px-4 py-5 font-sans text-fg sm:px-6 lg:px-8">
        <div className="mx-auto flex max-w-[1500px] flex-col gap-5">
          <header className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
            <div>
              <p className="mb-2 flex items-center gap-1.5 text-sm text-fg-muted">
                <Sun aria-hidden="true" className="size-4" /> {t("dashboard.todayNiceDay")}
              </p>
              <h1 className="text-3xl font-semibold tracking-tight text-fg-strong sm:text-4xl">
                {labelFor(date)}
              </h1>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <div className="flex rounded-full bg-panel p-1 ring-1 ring-line">
                {PERIODS.map((item) => (
                  <Button
                    className="rounded-full px-4 capitalize"
                    key={item}
                    onClick={() => setPeriod(item)}
                    size="sm"
                    variant={period === item ? "secondary" : "ghost"}
                  >
                    {t(`dashboard.${item}` as MessageKey)}
                  </Button>
                ))}
              </div>
              <div className="flex items-center gap-1 rounded-full bg-panel p-1 ring-1 ring-line">
                <Button
                  aria-label={t("dashboard.previousDay")}
                  onClick={() => shift(-1)}
                  size="icon"
                  variant="ghost"
                >
                  <ChevronLeft aria-hidden="true" />
                </Button>
                <Button
                  aria-label={t("dashboard.openCalendar")}
                  onClick={() => navigate({ name: "calendar" })}
                  size="icon"
                  variant="ghost"
                >
                  <CalendarDays aria-hidden="true" />
                </Button>
                <Button
                  aria-label={t("dashboard.nextDay")}
                  onClick={() => shift(1)}
                  size="icon"
                  variant="ghost"
                >
                  <ChevronRight aria-hidden="true" />
                </Button>
              </div>
            </div>
          </header>
          {data.error && (
            <p className="rounded-lg border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger">
              {data.error}
            </p>
          )}
          <section className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <Reveal>
              <Stat
                label={t("dashboard.workHours")}
                icon={<Clock className="size-4" />}
                value={
                  <AnimatedCounter
                    value={Math.round(total / 60_000)}
                    format={(value) => `${Math.round(value)}m`}
                  />
                }
                detail={
                  <>
                    {period === "day"
                      ? t("dashboard.trackedToday")
                      : t("dashboard.trackedThisPeriod", { period })}{" "}
                    · {changeLabel(total, previousTotal)}
                  </>
                }
              />
            </Reveal>
            <Reveal delay={0.06}>
              <Stat
                label={t("dashboard.percentOfTarget")}
                icon={<Target className="size-4" />}
                value={
                  <AnimatedCounter
                    value={Math.round((total / target) * 100)}
                    format={(value) => `${Math.round(value)}%`}
                  />
                }
                detail={
                  <>
                    of {formatDuration(target)} · {changeLabel(total, previousTotal)}
                  </>
                }
              />
            </Reveal>
            <Reveal delay={0.12}>
              <Stat
                label={t("dashboard.focusPercent")}
                icon={<Zap className="size-4" />}
                value={
                  <AnimatedCounter
                    value={Math.round(((categories[0]?.ms ?? 0) / Math.max(total, 1)) * 100)}
                    format={(value) => `${Math.round(value)}%`}
                  />
                }
                detail={period === "day" ? "Primary category share" : `Primary ${period} share`}
              />
            </Reveal>
            <Reveal delay={0.18}>
              <Stat
                label={t("dashboard.entries")}
                icon={<ListChecks className="size-4" />}
                value={
                  <AnimatedCounter
                    value={entries.length}
                    format={(value) => Math.round(value).toString()}
                  />
                }
                detail={`${data.metrics.pendingEntries} pending review`}
              />
            </Reveal>
          </section>
          <section className="grid gap-5 xl:grid-cols-[minmax(0,1.7fr)_minmax(320px,0.85fr)]">
            <Reveal delay={0.22}>
              <ActivityChart data={activityData} />
            </Reveal>
            <Reveal delay={0.26}>
              <Card className="border-line/70 bg-panel/75 shadow-none">
                <CardHeader className="flex flex-row items-center justify-between pb-0">
                  <CardTitle className="text-xl text-fg-strong">
                    {period === "day" ? "Daily summary" : `${nameFor(period)} summary`}
                  </CardTitle>
                  <More />
                </CardHeader>
                <CardContent className="pt-0">
                  <div className="flex justify-center py-3">
                    <Donut
                      slices={summarySlices}
                      size={220}
                      title={t("dashboard.periodSummary", { period: nameFor(period) })}
                    />
                  </div>
                </CardContent>
              </Card>
            </Reveal>
          </section>
          <Reveal delay={0.3}>
            <Timeline entries={dayEntries} />
          </Reveal>
          <section className="grid gap-5 lg:grid-cols-3">
            <Reveal delay={0.34}>
              <Ranking
                title={t("dashboard.categories")}
                items={categories}
                total={total}
                colors={["var(--chart-1)", "var(--chart-2)", "var(--chart-3)", "var(--chart-4)"]}
              />
            </Reveal>
            <Reveal delay={0.4}>
              <Ranking
                title={t("dashboard.apps")}
                items={apps}
                total={total}
                colors={["var(--chart-2)", "var(--chart-4)", "var(--chart-5)", "var(--chart-1)"]}
              />
            </Reveal>
            <Reveal delay={0.46}>
              <Ranking
                title={t("dashboard.projects")}
                items={projects}
                total={total}
                colors={["var(--chart-1)", "var(--chart-3)", "var(--chart-4)", "var(--chart-5)"]}
              />
            </Reveal>
          </section>
          <section className="grid gap-5 lg:grid-cols-2">
            <Reveal delay={0.5}>
              <Card className="border-line/70 bg-panel/75 shadow-none">
                <CardHeader className="flex flex-row items-center justify-between pb-2">
                  <CardTitle className="text-xl text-fg-strong">
                    {t("dashboard.recentActivity")}
                  </CardTitle>
                  <More />
                </CardHeader>
                <CardContent className="flex flex-col gap-2 pt-2">
                  {data.recentEntries.slice(0, 6).map((entry, index) => (
                    <motion.div
                      key={entry.id}
                      initial={{ opacity: 0, x: -6 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: index * 0.04, duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
                      className="flex items-center justify-between gap-3 rounded-md px-2 py-1.5 hover:bg-canvas/60"
                    >
                      <div className="min-w-0">
                        <p className="truncate text-sm text-fg-strong">
                          {entry.description || entry.dominantApp || t("dashboard.trackedTime")}
                        </p>
                        <p className="text-xs text-fg-faint">
                          {new Date(entry.startedAt).toLocaleTimeString(undefined, {
                            hour: "2-digit",
                            minute: "2-digit",
                          })}
                        </p>
                      </div>
                      <span className="shrink-0 tabular-nums text-xs text-fg-muted">
                        {formatDuration(duration(entry))}
                      </span>
                    </motion.div>
                  ))}
                  {data.recentEntries.length === 0 && (
                    <p className="py-6 text-sm text-fg-muted">{t("dashboard.noRecentActivity")}</p>
                  )}
                </CardContent>
              </Card>
            </Reveal>
            <Reveal delay={0.56}>
              <Card className="border-line/70 bg-panel/75 shadow-none">
                <CardHeader className="flex flex-row items-center justify-between pb-2">
                  <CardTitle className="text-xl text-fg-strong">
                    {t("dashboard.openTasks")}
                  </CardTitle>
                  <Button size="sm" variant="ghost" onClick={() => navigate({ name: "tasks" })}>
                    {t("dashboard.viewAll")}
                  </Button>
                </CardHeader>
                <CardContent className="flex flex-col gap-2 pt-2">
                  {data.openTasks.slice(0, 6).map((task, index) => (
                    <motion.div
                      key={task.id}
                      initial={{ opacity: 0, x: -6 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: index * 0.04, duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
                      className="flex items-center justify-between gap-3 rounded-md px-2 py-1.5 hover:bg-canvas/60"
                    >
                      <div className="min-w-0">
                        <p className="truncate text-sm text-fg-strong">{task.title}</p>
                        <p className="text-xs capitalize text-fg-faint">
                          {task.status.replace("_", " ")}
                        </p>
                      </div>
                      <Badge
                        variant={task.priority === "urgent" ? "destructive" : "secondary"}
                        className="shrink-0"
                      >
                        {task.priority}
                      </Badge>
                    </motion.div>
                  ))}
                  {data.openTasks.length === 0 && (
                    <p className="py-6 text-sm text-fg-muted">{t("dashboard.noOpenTasks")}</p>
                  )}
                </CardContent>
              </Card>
            </Reveal>
          </section>
        </div>
      </main>
    </MotionConfig>
  );
}
