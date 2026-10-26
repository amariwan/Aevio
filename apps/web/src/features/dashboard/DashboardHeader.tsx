import { ArrowUpRight, ChevronLeft, ChevronRight, Focus, RefreshCw, RotateCcw } from "lucide-react";
import { useId } from "react";
import { motion, useReducedMotion } from "motion/react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useI18n } from "@/hooks/useI18n";
import { localDateString, parseLocalDate } from "@/lib/dates";
import type { Route } from "@/lib/types";
import { periodBounds, shiftPeriod, type Period } from "./dashboardModel";
import type { DashboardView } from "./useDashboardView";

const periods: Period[] = ["day", "week", "month", "year"];

export function DashboardHeader({
  view,
  navigate,
}: {
  view: DashboardView;
  navigate: (route: Route) => void;
}) {
  const { t, locale } = useI18n();
  const indicator = useId();
  const reduced = useReducedMotion();
  const current = periodBounds(new Date(), view.period);
  const isCurrent = current.start === view.bounds.start;
  const label =
    view.period === "day"
      ? view.date.toLocaleDateString(locale, { weekday: "long", day: "numeric", month: "long" })
      : view.period === "year"
        ? view.date.getFullYear().toString()
        : view.period === "month"
          ? view.date.toLocaleDateString(locale, { month: "long", year: "numeric" })
          : `${new Date(view.bounds.start).toLocaleDateString(locale, { day: "numeric", month: "short" })} – ${new Date(view.bounds.end - 1).toLocaleDateString(locale, { day: "numeric", month: "short", year: "numeric" })}`;
  return (
    <header className="flex flex-col gap-6">
      <div className="flex flex-wrap items-start justify-between gap-5">
        <div className="flex flex-col gap-2">
          <Badge variant="outline" className="w-fit gap-2">
            <span className="size-1.5 rounded-full bg-success" aria-hidden="true" />
            {t("dashboard.localData")}
          </Badge>
          <h1 className="text-2xl font-semibold tracking-tight text-fg-strong sm:text-3xl xl:text-4xl">
            {t("dashboard.workspaceOverview")}
          </h1>
          <p className="text-sm text-fg-muted">{t("dashboard.subtitle")}</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="ghost"
            onClick={() => navigate({ name: "timesheet", date: localDateString(view.date) })}
          >
            {t("dashboard.viewTimesheet")}
            <ArrowUpRight data-icon="inline-end" />
          </Button>
          <Button size="lg" onClick={() => navigate({ name: "focus" })}>
            <Focus data-icon="inline-start" />
            {t("dashboard.focusNow")}
          </Button>
        </div>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-line pb-4">
        <Tabs
          value={view.period}
          onValueChange={(value) => {
            if (periods.includes(value as Period)) view.setPeriod(value as Period);
          }}
        >
          <TabsList aria-label={t("dashboard.selectedPeriod")}>
            {periods.map((period) => (
              <TabsTrigger key={period} value={period} className="relative min-w-14 px-3">
                {view.period === period && (
                  <motion.span
                    aria-hidden="true"
                    layoutId={indicator}
                    className="absolute inset-0 rounded-md bg-surface-strong ring-1 ring-line"
                    transition={{
                      type: "spring",
                      stiffness: 400,
                      damping: 32,
                      duration: reduced ? 0 : undefined,
                    }}
                  />
                )}
                <span className="relative">{t(`dashboard.${period}`)}</span>
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
        <div className="flex flex-wrap items-center gap-2">
          <NativeSelect
            aria-label={t("dashboard.projects")}
            value={view.projectId}
            onChange={(event) => view.setProjectId(event.target.value)}
            className="max-w-44"
          >
            <NativeSelectOption value="">{t("dashboard.allProjects")}</NativeSelectOption>
            {view.data.catalog.projects.map((project) => (
              <NativeSelectOption key={project.id} value={project.id}>
                {project.name}
              </NativeSelectOption>
            ))}
          </NativeSelect>
          <Button
            size="icon"
            variant="ghost"
            aria-label={t("dashboard.previousPeriod")}
            onClick={() => view.setDate(shiftPeriod(view.date, view.period, -1))}
          >
            <ChevronLeft />
          </Button>
          <div className="relative">
            <span
              className="pointer-events-none inline-flex h-8 items-center gap-2 text-sm font-medium"
              aria-hidden="true"
            >
              {label}
            </span>
            <Input
              type="date"
              aria-label={t("dashboard.chooseDate")}
              className="absolute inset-0 opacity-0 focus-visible:opacity-100"
              value={localDateString(view.date)}
              onChange={(event) => {
                if (event.target.value) view.setDate(parseLocalDate(event.target.value));
              }}
            />
          </div>
          <Button
            size="icon"
            variant="ghost"
            aria-label={t("dashboard.nextPeriod")}
            disabled={view.bounds.start >= current.start}
            onClick={() => view.setDate(shiftPeriod(view.date, view.period, 1))}
          >
            <ChevronRight />
          </Button>
          {!isCurrent && (
            <Button size="sm" variant="outline" onClick={() => view.setDate(new Date())}>
              {t("dashboard.today")}
            </Button>
          )}
          {view.projectId && (
            <Button
              size="icon"
              variant="ghost"
              aria-label={t("dashboard.resetFilters")}
              onClick={view.reset}
            >
              <RotateCcw />
            </Button>
          )}
          <Button
            size="icon"
            variant="ghost"
            aria-label={t("dashboard.refresh")}
            disabled={view.data.loading}
            onClick={() => void view.data.reload()}
          >
            <RefreshCw className={view.data.loading && !reduced ? "animate-spin" : undefined} />
          </Button>
        </div>
      </div>
    </header>
  );
}
