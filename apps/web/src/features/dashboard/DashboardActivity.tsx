import { useId, useState } from "react";
import { useReducedMotion } from "motion/react";
import {
  Area,
  CartesianGrid,
  Line,
  ComposedChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { ChartNoAxesCombined } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useI18n } from "@/hooks/useI18n";
import { formatDuration } from "@/lib/format";
import type { DashboardView } from "./useDashboardView";

export function DashboardActivity({ view }: { view: DashboardView }) {
  const { t } = useI18n();
  const [metric, setMetric] = useState<"tracked" | "billable">("tracked");
  const [compare, setCompare] = useState(true);
  const reduced = useReducedMotion();
  const fillId = useId().replaceAll(":", "");
  const previousKey = metric === "tracked" ? "previousTracked" : "previousBillable";
  const total = metric === "tracked" ? view.summary.trackedMs : view.summary.billableMs;
  return (
    <Card className="h-full min-w-0" whileHover={undefined} whileTap={undefined}>
      <CardHeader className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <CardTitle>{t("dashboard.activity")}</CardTitle>
          <CardDescription>{t("dashboard.selectedPeriod")}</CardDescription>
        </div>
        <Tabs
          value={metric}
          onValueChange={(value) => {
            if (value === "tracked" || value === "billable") setMetric(value);
          }}
        >
          <TabsList aria-label={t("dashboard.activity")}>
            <TabsTrigger value="tracked">{t("dashboard.tracked")}</TabsTrigger>
            <TabsTrigger value="billable">{t("dashboard.billable")}</TabsTrigger>
          </TabsList>
        </Tabs>
      </CardHeader>
      <CardContent className="flex flex-col gap-5">
        <div className="flex flex-wrap items-end justify-between gap-3">
          <p className="text-3xl font-medium tracking-tight tabular-nums text-fg-strong">
            {formatDuration(total)}
          </p>
          <Button
            size="sm"
            variant={compare ? "secondary" : "ghost"}
            aria-pressed={compare}
            onClick={() => setCompare((value) => !value)}
          >
            <ChartNoAxesCombined data-icon="inline-start" />
            {t("dashboard.compare")}
          </Button>
        </div>
        <div
          className="h-60 min-w-0 sm:h-72"
          role="img"
          aria-label={`${t("dashboard.activity")}: ${formatDuration(total)}`}
        >
          <ResponsiveContainer width="100%" height="100%">
            <ComposedChart
              data={view.chart}
              margin={{ top: 8, right: 4, bottom: 0, left: -12 }}
              accessibilityLayer
            >
              <defs>
                <linearGradient id={fillId} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="var(--accent)" stopOpacity={0.28} />
                  <stop offset="100%" stopColor="var(--accent)" stopOpacity={0.01} />
                </linearGradient>
              </defs>
              <CartesianGrid stroke="var(--line)" vertical={false} strokeDasharray="3 5" />
              <XAxis
                dataKey="label"
                axisLine={false}
                tickLine={false}
                minTickGap={28}
                tick={{ fill: "var(--fg-muted)", fontSize: 11 }}
                dy={8}
              />
              <YAxis
                axisLine={false}
                tickLine={false}
                width={48}
                tick={{ fill: "var(--fg-muted)", fontSize: 11 }}
                tickFormatter={(value: number) => `${Math.round(value / 3_600_000)}h`}
              />
              <Tooltip
                formatter={(value) => formatDuration(Number(value))}
                contentStyle={{
                  background: "var(--panel)",
                  border: "1px solid var(--line)",
                  borderRadius: 12,
                  color: "var(--fg)",
                  fontSize: 12,
                }}
              />
              {compare && (
                <Line
                  name={t("dashboard.previous")}
                  type="monotone"
                  dataKey={previousKey}
                  stroke="var(--fg-faint)"
                  strokeDasharray="4 5"
                  strokeWidth={1.5}
                  dot={false}
                  isAnimationActive={false}
                />
              )}
              <Area
                key={metric}
                name={t(`dashboard.${metric}`)}
                type="monotone"
                dataKey={metric}
                stroke="var(--accent)"
                strokeWidth={2.5}
                fill={`url(#${fillId})`}
                isAnimationActive={!reduced}
                animationDuration={450}
              />
            </ComposedChart>
          </ResponsiveContainer>
        </div>
        <div className="flex flex-wrap items-center gap-4 text-xs text-fg-muted">
          <span className="flex items-center gap-2">
            <span className="size-2 rounded-full bg-accent" aria-hidden="true" />
            {t(`dashboard.${metric}`)} · {formatDuration(total)}
          </span>
          {compare && (
            <span className="flex items-center gap-2">
              <span className="w-4 border-t border-dashed border-fg-faint" aria-hidden="true" />
              {t("dashboard.previous")} ·{" "}
              {formatDuration(
                metric === "tracked" ? view.comparison.trackedMs : view.comparison.billableMs,
              )}
            </span>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
