import { useEffect, useMemo, useState } from "react";
import { motion, useReducedMotion } from "motion/react";
import { ArrowUpRight, Settings2, Volume2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SettingsProvider, useSettings } from "@/hooks/useSettings";
import { I18nProvider, useI18n } from "@/hooks/useI18n";
import { startOfToday } from "@/lib/activity";
import { durationOf } from "@/lib/entries";
import { formatDuration } from "@/lib/format";
import { dailyTargetMs } from "@/lib/settings";
import * as api from "@/lib/api";
import type { ActivitySnapshot, Category, TimeEntry } from "@/lib/types";

const COLORS = ["bg-accent", "bg-cat-4", "bg-cat-8", "bg-fg-faint"];

export default function ProductivitySummary() {
  return (
    <SettingsProvider>
      <I18nProvider>
        <SummaryContent />
      </I18nProvider>
    </SettingsProvider>
  );
}

function SummaryContent() {
  const { settings } = useSettings();
  const { t } = useI18n();
  const [snapshot, setSnapshot] = useState<ActivitySnapshot | null>(null);
  const [entries, setEntries] = useState<TimeEntry[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [error, setError] = useState<string | null>(null);
  const target = dailyTargetMs(settings);

  useEffect(() => {
    const load = async (): Promise<void> => {
      try {
        const start = startOfToday();
        const [nextSnapshot, nextEntries, nextCategories] = await Promise.all([
          api.fetchActivitySnapshot(start),
          api.listTimeEntries(start, Date.now()),
          api.listCategories(),
        ]);
        setSnapshot(nextSnapshot);
        setEntries(nextEntries);
        setCategories(nextCategories);
      } catch (cause) {
        setError(api.describeError(cause));
      }
    };
    void load();
  }, []);

  const workMs = entries.reduce((total, entry) => total + durationOf(entry, Date.now()), 0);
  const percent = target > 0 ? Math.round((workMs / target) * 100) : 0;
  const meetingMs = Math.max(
    0,
    (snapshot?.trackedMs ?? workMs) - (snapshot?.focusMs ?? 0) - (snapshot?.breakMs ?? 0),
  );
  const categoriesById = useMemo(
    () => new Map(categories.map((category) => [category.id, category])),
    [categories],
  );
  const topCategories = useMemo(() => {
    const totals = new Map<string, number>();
    for (const entry of entries) {
      const key = entry.categoryId ?? "other";
      totals.set(key, (totals.get(key) ?? 0) + durationOf(entry, Date.now()));
    }
    return [...totals.entries()].sort((left, right) => right[1] - left[1]).slice(0, 3);
  }, [entries]);

  return (
    <main className="flex h-[555px] w-[420px] select-none flex-col overflow-hidden rounded-2xl border border-line bg-panel text-fg shadow-2xl">
      <header className="flex items-center justify-between border-b border-line px-6 py-3 text-[11px] tracking-[0.18em] text-fg-muted">
        <span className="font-semibold">
          AEVIO <span className="text-accent">✦ PRODUCTIVITY COACH</span>
        </span>
        <div className="flex items-center gap-1 text-fg-faint">
          <Button variant="ghost" size="icon-sm" aria-label={t("summary.soundSettings")}>
            <Volume2 />
          </Button>
          <Button variant="ghost" size="icon-sm" aria-label={t("summary.openCoachSettings")}>
            <Settings2 />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t("summary.dismiss")}
            onClick={() => void api.hideProductivitySummary()}
          >
            <X />
          </Button>
        </div>
      </header>

      <section className="space-y-2 px-6 pt-5">
        <h1 className="text-xl font-semibold tracking-tight text-fg-strong">
          You reached your work goal today.
        </h1>
        <p className="max-w-[330px] text-sm leading-5 text-fg-muted">
          Wrap up your day and see how your time was spent.
        </p>
      </section>

      <section className="grid grid-cols-2 gap-6 px-6 pt-6">
        <Metric label={t("summary.workHours")} value={formatDuration(workMs)} />
        <Metric
          label={t("summary.percentOfTarget")}
          value={`${percent}%`}
          note={`of ${formatDuration(target)}`}
        />
      </section>

      <section className="mt-6 border-t border-line px-6 pt-5">
        <div className="mb-3 text-[11px] text-fg-faint">{t("summary.breakdown")}</div>
        <div className="flex items-center gap-5">
          <Donut percent={Math.min(100, percent)} />
          <div className="min-w-0 flex-1 space-y-2 text-sm">
            <BreakdownRow
              color="bg-accent"
              label={t("summary.focus")}
              value={snapshot?.focusMs ?? 0}
            />
            <BreakdownRow color="bg-cat-4" label={t("summary.meetings")} value={meetingMs} />
            <BreakdownRow
              color="bg-cat-8"
              label={t("summary.breaks")}
              value={snapshot?.breakMs ?? 0}
            />
            <BreakdownRow
              color="bg-fg-faint"
              label={t("summary.other")}
              value={Math.max(0, workMs - (snapshot?.focusMs ?? 0) - meetingMs)}
            />
          </div>
        </div>
      </section>

      <section className="mt-5 border-t border-line px-6 pt-5">
        <div className="mb-3 text-[11px] text-fg-faint">{t("summary.topCategories")}</div>
        <div className="space-y-2.5">
          {topCategories.length === 0 && (
            <p className="text-sm text-fg-muted">No categorized time yet.</p>
          )}
          {topCategories.map(([id, value], index) => {
            const label =
              id === "other"
                ? t("summary.other")
                : (categoriesById.get(id)?.name ?? t("summary.uncategorized"));
            const share = workMs > 0 ? Math.round((value / workMs) * 100) : 0;
            return (
              <CategoryRow
                key={id}
                color={COLORS[index] ?? COLORS[3]}
                label={label}
                share={share}
                value={value}
              />
            );
          })}
        </div>
      </section>

      {error && <p className="mt-4 px-6 text-xs text-danger">{error}</p>}
      <footer className="mt-auto flex items-center justify-end gap-2 border-t border-line bg-surface px-6 py-4">
        <Button variant="outline" onClick={() => void api.hideProductivitySummary()}>
          Dismiss
        </Button>
        <Button
          onClick={() => {
            void api.hideProductivitySummary();
            void api.openMainWindow(false);
          }}
        >
          <ArrowUpRight data-icon="inline-start" /> View dashboard
        </Button>
      </footer>
    </main>
  );
}

function Metric({ label, value, note }: { label: string; value: string; note?: string }) {
  return (
    <div>
      <div className="text-xs text-fg-faint">{label}</div>
      <div className="mt-1 text-xl font-semibold tabular-nums text-fg-strong">
        {value} <span className="text-xs font-normal text-fg-faint">{note}</span>
      </div>
    </div>
  );
}

function Donut({ percent }: { percent: number }) {
  const reduced = useReducedMotion();
  const radius = 35;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference * (1 - Math.min(100, percent) / 100);
  return (
    <motion.svg
      viewBox="0 0 84 84"
      className="size-[84px] shrink-0"
      aria-label={`${percent}% of target`}
      initial={reduced ? false : { opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={reduced ? { duration: 0 } : { type: "spring", stiffness: 260, damping: 24 }}
    >
      <circle
        cx="42"
        cy="42"
        r={radius}
        fill="none"
        stroke="currentColor"
        strokeWidth="9"
        className="text-line"
      />
      <motion.circle
        cx="42"
        cy="42"
        r={radius}
        fill="none"
        stroke="currentColor"
        strokeWidth="9"
        strokeLinecap="round"
        className="text-accent"
        strokeDasharray={circumference}
        initial={reduced ? false : { strokeDashoffset: circumference }}
        animate={{ strokeDashoffset: offset }}
        transition={
          reduced ? { duration: 0 } : { type: "spring", stiffness: 180, damping: 22, mass: 1 }
        }
        transform="rotate(-90 42 42)"
      />
      <motion.text
        x="42"
        y="47"
        textAnchor="middle"
        className="fill-fg-strong text-[12px] font-semibold"
        initial={reduced ? false : { opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={reduced ? { duration: 0 } : { duration: 0.4, delay: 0.2 }}
      >
        {percent}%
      </motion.text>
    </motion.svg>
  );
}

function BreakdownRow({ color, label, value }: { color: string; label: string; value: number }) {
  return (
    <div className="flex items-center gap-2">
      <span className={`size-3 rounded-[3px] ${color}`} />
      <span className="flex-1 text-fg-muted">{label}</span>
      <span className="tabular-nums text-fg-muted">{formatDuration(value)}</span>
    </div>
  );
}

function CategoryRow({
  color,
  label,
  share,
  value,
}: {
  color: string;
  label: string;
  share: number;
  value: number;
}) {
  return (
    <div className="flex items-center gap-2 text-sm">
      <span className="w-8 tabular-nums text-fg-muted">{share}%</span>
      <span className="min-w-0 flex-1 truncate text-fg-muted">{label}</span>
      <span className="h-2 w-20 overflow-hidden rounded-full bg-surface-strong">
        <span
          className={`block h-full rounded-full ${color}`}
          style={{ width: `${Math.max(8, share)}%` }}
        />
      </span>
      <span className="w-16 text-right tabular-nums text-fg-muted">{formatDuration(value)}</span>
    </div>
  );
}
