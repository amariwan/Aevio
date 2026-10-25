import { motion, useReducedMotion } from "motion/react";
import AppleActivityCard, { type ActivityData } from "@/components/kokonutui/apple-activity-card";
import { PALETTE } from "@/lib/palette";
import { cn } from "@/lib/utils";
import type { useDashboardData } from "../useDashboardData";

type DashboardData = ReturnType<typeof useDashboardData>;

const CHART_HEIGHT = 132;

function minutes(ms: number): number {
  return Math.round(ms / 60_000);
}

function activityRings(data: DashboardData): ActivityData[] {
  const activeProjects = data.metrics.activeProjects;
  return [
    {
      label: "FOCUS",
      color: PALETTE[0],
      colorEnd: PALETTE[8],
      size: 196,
      current: minutes(data.metrics.todayMs),
      target: 360,
      unit: "MIN",
    },
    {
      label: "WEEK",
      color: PALETTE[1],
      colorEnd: PALETTE[2],
      size: 156,
      current: minutes(data.metrics.weekMs),
      target: 1_200,
      unit: "MIN",
    },
    {
      label: "WORK",
      color: PALETTE[3],
      colorEnd: PALETTE[9],
      size: 116,
      current: activeProjects,
      target: Math.max(activeProjects, 6),
      unit: "PROJ",
    },
  ];
}

function TerrainChart({ data }: { data: DashboardData }) {
  const reduceMotion = useReducedMotion() ?? false;
  const max = Math.max(...data.days.map((day) => day.trackedMs), 1);

  return (
    <div className="relative h-full min-h-[240px] overflow-hidden rounded-2xl border border-line/70 bg-canvas/70 p-4 [perspective:900px]">
      <div className="pointer-events-none absolute inset-x-6 top-8 h-px bg-line/70" />
      <div className="pointer-events-none absolute inset-x-10 top-1/2 h-px bg-line/50" />
      <div className="pointer-events-none absolute inset-x-6 bottom-9 h-px bg-line/70" />
      <div className="absolute inset-x-6 top-8 bottom-9 flex items-end gap-2 [transform:rotateX(52deg)_rotateZ(-3deg)_scale(1.05)] [transform-style:preserve-3d]">
        {data.days.map((day, index) => {
          const height = Math.max(8, (day.trackedMs / max) * CHART_HEIGHT);
          return (
            <div className="flex h-full min-w-0 flex-1 items-end" key={day.date.toISOString()}>
              <motion.div
                className={cn(
                  "relative w-full origin-bottom rounded-t-md bg-accent shadow-[0_0_22px_hsl(var(--accent)/.28)]",
                  index === data.days.length - 1 &&
                    "bg-success shadow-[0_0_22px_hsl(var(--success)/.28)]",
                )}
                initial={reduceMotion ? { scaleY: 1 } : { scaleY: 0 }}
                animate={{ scaleY: 1 }}
                transition={{ duration: 0.7, delay: index * 0.06, ease: [0.22, 1, 0.36, 1] }}
                style={{ height, transformOrigin: "bottom" }}
                title={`${day.label}: ${minutes(day.trackedMs)} minutes`}
              >
                <span className="absolute inset-x-0 -top-2 h-2 rounded-full bg-current opacity-80" />
              </motion.div>
            </div>
          );
        })}
      </div>
      <div className="absolute inset-x-4 bottom-3 flex justify-between text-[10px] font-medium tracking-[0.16em] text-fg-faint uppercase">
        {data.days.map((day) => (
          <span key={day.date.toISOString()}>{day.label}</span>
        ))}
      </div>
      <div className="absolute top-4 left-4 flex items-center gap-2 text-[10px] font-semibold tracking-[0.16em] text-fg-muted uppercase">
        <span className="size-2 rounded-full bg-accent shadow-[0_0_12px_hsl(var(--accent)/.7)]" />
        Activity terrain
      </div>
    </div>
  );
}

export function ProductivityField({ data }: { data: DashboardData }) {
  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1.45fr)_minmax(320px,0.85fr)]">
      <TerrainChart data={data} />
      <div className="min-w-0 overflow-hidden rounded-2xl border border-line/70 bg-canvas/70">
        <AppleActivityCard
          activities={activityRings(data)}
          className="h-full max-w-none rounded-none bg-transparent p-5"
          title="Productivity field"
          titleAs="h3"
        />
      </div>
    </div>
  );
}
