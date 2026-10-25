import { type ReactNode, useId, useState } from "react";
import { motion, useReducedMotion } from "motion/react";
import { formatDuration, formatHours } from "@/lib/format";
import { Tooltip } from "../primitives/Tooltip";
import { CHART_TOKENS, CHART_MOTION } from "@/components/charts";

/**
 * Small SVG charts for totals by category, project, or app. Identity is
 * never colour alone: category colours are user data, so every chart ships a
 * named legend, a 2px surface gap between touching marks, and a hover
 * tooltip, and each view that uses them also offers the numbers as a table.
 *
 * All charts are animated with spring physics and respect prefers-reduced-motion.
 */

export interface Slice {
  key: string;
  label: string;
  color: string;
  ms: number;
}

const GAP_PX = CHART_TOKENS.gap;
const OTHER_KEY = "__other";

/** Round the four corners of an annular slice, keeping its radial edges flat. */
function roundedDonutPath(
  center: number,
  radius: number,
  thickness: number,
  start: number,
  end: number,
): string {
  const outer = radius + thickness / 2;
  const inner = radius - thickness / 2;
  const point = (r: number, angle: number) =>
    `${center + r * Math.cos(angle)},${center + r * Math.sin(angle)}`;

  if (end - start >= 2 * Math.PI - 1e-6) {
    return [
      `M ${point(outer, 0)}`,
      `A ${outer},${outer} 0 1 1 ${point(outer, Math.PI)}`,
      `A ${outer},${outer} 0 1 1 ${point(outer, 0)} Z`,
      `M ${point(inner, 0)}`,
      `A ${inner},${inner} 0 1 0 ${point(inner, Math.PI)}`,
      `A ${inner},${inner} 0 1 0 ${point(inner, 0)} Z`,
    ].join(" ");
  }

  const corner = Math.min(2, thickness / 2, ((end - start) * inner) / 2);
  const outerTrim = corner / outer;
  const innerTrim = corner / inner;
  const outerLarge = end - start - 2 * outerTrim > Math.PI ? 1 : 0;
  const innerLarge = end - start - 2 * innerTrim > Math.PI ? 1 : 0;

  return [
    `M ${point(outer, start + outerTrim)}`,
    `A ${outer},${outer} 0 ${outerLarge} 1 ${point(outer, end - outerTrim)}`,
    `Q ${point(outer, end)} ${point(outer - corner, end)}`,
    `L ${point(inner + corner, end)}`,
    `Q ${point(inner, end)} ${point(inner, end - innerTrim)}`,
    `A ${inner},${inner} 0 ${innerLarge} 0 ${point(inner, start + innerTrim)}`,
    `Q ${point(inner, start)} ${point(inner + corner, start)}`,
    `L ${point(outer - corner, start)}`,
    `Q ${point(outer, start)} ${point(outer, start + outerTrim)} Z`,
  ].join(" ");
}

/** A donut with a legend beside it: share of time by group. */
export function Donut({
  slices,
  size = 132,
  title,
  emptyLabel = "No time yet",
  maxLegend = 6,
  stacked = false,
  showLegend = true,
  large = false,
  foldLone = false,
}: {
  slices: Slice[];
  size?: number;
  title: string;
  emptyLabel?: string;
  maxLegend?: number;
  stacked?: boolean;
  showLegend?: boolean;
  large?: boolean;
  foldLone?: boolean;
}) {
  const reduced = useReducedMotion();
  const [hovered, setHovered] = useState<string | null>(null);
  const total = slices.reduce((sum, slice) => sum + slice.ms, 0);
  const sorted = [...slices].filter((s) => s.ms > 0).sort((a, b) => b.ms - a.ms);
  const keep = sorted.length > maxLegend + (foldLone ? 0 : 1) ? maxLegend : sorted.length;
  const shown = sorted.slice(0, keep);
  const rest = sorted.slice(keep);
  if (rest.length > 0) {
    shown.push({
      key: OTHER_KEY,
      label: `Other (${rest.length})`,
      color: "var(--fg-ghost)",
      ms: rest.reduce((sum, slice) => sum + slice.ms, 0),
    });
  }

  const stroke = CHART_TOKENS.donut.stroke;
  const radius = (size - stroke) / 2;
  const circumference = 2 * Math.PI * radius;
  const gap = shown.length > 1 ? GAP_PX : 0;
  let offset = 0;
  const focus = shown.find((slice) => slice.key === hovered);

  const spring = reduced ? { duration: 0 } : { type: "spring" as const, ...CHART_MOTION.spring };

  return (
    <figure className={`flex min-w-0 gap-4 ${stacked ? "flex-col" : "items-center"}`}>
      <motion.svg
        viewBox={`0 0 ${size} ${size}`}
        style={{ width: `${size / 16}rem`, height: `${size / 16}rem` }}
        role="group"
        aria-label={`${title}: ${shown
          .map((s) => `${s.label} ${formatDuration(s.ms)}`)
          .join(", ")}`}
        className={`shrink-0 ${stacked ? "self-center" : ""}`}
        initial={reduced ? false : { opacity: 0, scale: 0.9 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={spring}
      >
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="var(--bg-surface-2)"
          strokeWidth={stroke}
        />
        {total > 0 &&
          shown.map((slice) => {
            const length = (slice.ms / total) * circumference;
            const start = (offset + gap / 2) / radius - Math.PI / 2;
            const end = (offset + length - gap / 2) / radius - Math.PI / 2;
            offset += length;
            if (end <= start) return null;
            const tooltipContent = `${slice.label}: ${formatDuration(slice.ms)} (${Math.round((slice.ms / total) * 100)}%)`;
            const isHovered = hovered === slice.key;
            return (
              <Tooltip key={slice.key} content={tooltipContent}>
                <motion.path
                  d={roundedDonutPath(
                    size / 2,
                    radius,
                    isHovered ? stroke + 3 : stroke,
                    start,
                    end,
                  )}
                  fill={slice.color}
                  fillRule="evenodd"
                  initial={reduced ? false : { opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ ...spring, delay: reduced ? 0 : 0.05 }}
                  onMouseEnter={() => setHovered(slice.key)}
                  onMouseLeave={() => setHovered(null)}
                  onFocus={() => setHovered(slice.key)}
                  onBlur={() => setHovered(null)}
                  tabIndex={0}
                  role="button"
                  aria-label={tooltipContent}
                  className="cursor-pointer outline-none"
                  style={{
                    filter: isHovered ? "brightness(1.15)" : undefined,
                    transition: "filter 150ms ease",
                  }}
                />
              </Tooltip>
            );
          })}
        <motion.text
          x="50%"
          y="47%"
          textAnchor="middle"
          className="fill-fg-strong font-semibold text-[15px]"
          initial={reduced ? false : { opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ ...spring, delay: reduced ? 0 : 0.2 }}
        >
          {total > 0 ? formatDuration(showLegend ? (focus?.ms ?? total) : total) : "–"}
        </motion.text>
        <motion.text
          x="50%"
          y="61%"
          textAnchor="middle"
          className="fill-fg-soft text-[10px]"
          initial={reduced ? false : { opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ ...spring, delay: reduced ? 0 : 0.25 }}
        >
          {showLegend && focus
            ? `${Math.round((focus.ms / total) * 100)}%`
            : total > 0
              ? "total"
              : emptyLabel}
        </motion.text>
      </motion.svg>
      {showLegend && (
        <figcaption
          className={`min-w-0 max-w-sm flex-1 space-y-1 ${large ? "text-[14px]" : "text-[11.5px]"}`}
        >
          <div
            className={`mb-1.5 font-semibold text-fg-faint uppercase tracking-wider ${
              large ? "text-[12px]" : "text-[10.5px]"
            }`}
          >
            {title}
          </div>
          {shown.length === 0 && <div className="text-fg-faint">{emptyLabel}</div>}
          {shown.map((slice, i) =>
            slice.key === OTHER_KEY ? (
              <OtherRow
                key={slice.key}
                slice={slice}
                parts={rest}
                highlighted={hovered === slice.key}
                onHover={(on) => setHovered(on ? slice.key : null)}
                reduced={reduced}
              />
            ) : (
              <motion.div
                key={slice.key}
                initial={reduced ? false : { opacity: 0, x: -6 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ ...spring, delay: reduced ? 0 : 0.05 * i }}
                onMouseEnter={() => setHovered(slice.key)}
                onMouseLeave={() => setHovered(null)}
                className={`flex items-center gap-2 rounded px-1 ${
                  hovered === slice.key ? "bg-surface" : ""
                }`}
              >
                <span
                  className="size-2 shrink-0 rounded-full"
                  style={{ backgroundColor: slice.color }}
                />
                <span className="min-w-0 flex-1 truncate text-fg-muted">{slice.label}</span>
                <span className="shrink-0 font-mono text-fg-soft tabular-nums">
                  {formatDuration(slice.ms)}
                </span>
              </motion.div>
            ),
          )}
        </figcaption>
      )}
    </figure>
  );
}

function OtherRow({
  slice,
  parts,
  highlighted,
  onHover,
  reduced,
}: {
  slice: Slice;
  parts: Slice[];
  highlighted: boolean;
  onHover: (on: boolean) => void;
  reduced: boolean | null;
}) {
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const [pinned, setPinned] = useState(false);
  const panelId = useId();
  const open = hover || focus || pinned;
  const setHovering = (on: boolean) => {
    setHover(on);
    onHover(on);
  };
  const spring = reduced ? { duration: 0 } : { type: "spring" as const, ...CHART_MOTION.spring };

  return (
    <div
      className="relative"
      onMouseEnter={() => setHovering(true)}
      onMouseLeave={() => setHovering(false)}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) {
          setFocus(false);
          setPinned(false);
        }
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          setPinned(false);
          setFocus(false);
          setHover(false);
        }
      }}
    >
      <button
        type="button"
        aria-expanded={open}
        aria-controls={panelId}
        onFocus={() => setFocus(true)}
        onClick={() => setPinned((value) => !value)}
        className={`flex w-full items-center gap-2 rounded px-1 text-left ${
          highlighted || open ? "bg-surface" : ""
        }`}
      >
        <span className="size-2 shrink-0 rounded-full" style={{ backgroundColor: slice.color }} />
        <span className="min-w-0 flex-1 truncate text-fg-muted">{slice.label}</span>
        <span className="shrink-0 font-mono text-fg-soft tabular-nums">
          {formatDuration(slice.ms)}
        </span>
        <span aria-hidden="true" className="shrink-0 text-[9px] text-fg-faint">
          ▾
        </span>
      </button>
      {open && (
        <motion.div
          id={panelId}
          className="absolute right-0 left-0 z-20 mt-1 space-y-1.5 rounded-md border border-line bg-panel p-2.5 shadow-lg"
          initial={reduced ? false : { opacity: 0, y: -4 }}
          animate={{ opacity: 1, y: 0 }}
          transition={spring}
        >
          <StackedBar slices={parts} height={8} />
          <div className="space-y-1">
            {parts.map((part) => (
              <div key={part.key} className="flex items-center gap-2">
                <span
                  className="size-2 shrink-0 rounded-full"
                  style={{ backgroundColor: part.color }}
                />
                <span className="min-w-0 flex-1 truncate text-fg-muted">{part.label}</span>
                <span className="shrink-0 font-mono text-fg-soft tabular-nums">
                  {formatDuration(part.ms)}
                </span>
                <span className="w-8 shrink-0 text-right font-mono text-[10.5px] text-fg-faint tabular-nums">
                  {Math.round((part.ms / slice.ms) * 100)}%
                </span>
              </div>
            ))}
          </div>
        </motion.div>
      )}
    </div>
  );
}

/** A thin horizontal bar split by group, for a month cell or a row. */
export function StackedBar({
  slices,
  totalMs,
  height = 6,
}: {
  slices: Slice[];
  totalMs?: number;
  height?: number;
}) {
  const reduced = useReducedMotion();
  const sum = slices.reduce((acc, slice) => acc + slice.ms, 0);
  const total = Math.max(totalMs ?? sum, sum);
  if (total <= 0) return null;
  const spring = reduced ? { duration: 0 } : { type: "spring" as const, ...CHART_MOTION.spring };

  return (
    <motion.div
      className="flex w-full overflow-hidden rounded-full bg-surface-strong"
      style={{ height, gap: GAP_PX }}
      aria-hidden="true"
      initial={reduced ? false : { scaleX: 0, originX: 0 }}
      animate={{ scaleX: 1 }}
      transition={spring}
    >
      {slices
        .filter((slice) => slice.ms > 0)
        .map((slice, i) => (
          <Tooltip key={slice.key} content={`${slice.label}: ${formatDuration(slice.ms)}`}>
            <motion.span
              className="block h-full first:rounded-l-full last:rounded-r-full"
              style={{
                width: `${(slice.ms / total) * 100}%`,
                backgroundColor: slice.color,
              }}
              initial={reduced ? false : { opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ ...spring, delay: reduced ? 0 : 0.03 * i }}
            />
          </Tooltip>
        ))}
    </motion.div>
  );
}

export interface Column {
  key: string;
  label: string;
  slices: Slice[];
}

/** Clean hour ticks for a y-axis: 0, 2h, 4h, ... */
function hourTicks(maxMs: number): number[] {
  const maxHours = Math.max(1, maxMs / 3_600_000);
  const steps = [0.5, 1, 2, 4, 5, 10, 20, 25, 50, 100, 200, 500];
  const step = steps.find((candidate) => maxHours / candidate <= 4) ?? 1000;
  const ticks: number[] = [];
  for (let hours = 0; hours <= maxHours + step * 0.001; hours += step) {
    ticks.push(hours * 3_600_000);
  }
  if (ticks[ticks.length - 1] < maxMs) {
    ticks.push(ticks[ticks.length - 1] + step * 3_600_000);
  }
  return ticks;
}

/** Stacked columns, one per bucket (daily time by category). */
export function StackedColumns({
  columns,
  legend,
  title,
  height = 180,
}: {
  columns: Column[];
  legend: Slice[];
  title: string;
  height?: number;
}) {
  const reduced = useReducedMotion();
  const [tip, setTip] = useState<{
    column: Column;
    slice: Slice;
    x: number;
  } | null>(null);
  const totals = columns.map((column) => column.slices.reduce((sum, slice) => sum + slice.ms, 0));
  const ticks = hourTicks(Math.max(0, ...totals));
  const top = ticks[ticks.length - 1] || 1;
  const labelEvery = Math.max(1, Math.ceil(columns.length / 16));
  const spring = reduced ? { duration: 0 } : { type: "spring" as const, ...CHART_MOTION.spring };

  return (
    <figure className="min-w-0">
      <div className="mb-2 flex flex-wrap items-center gap-x-3 gap-y-1">
        <span className="font-semibold text-[10.5px] text-fg-faint uppercase tracking-wider">
          {title}
        </span>
        {legend.length > 1 &&
          legend.map((slice) => (
            <span
              key={slice.key}
              className="inline-flex items-center gap-1.5 text-[11px] text-fg-muted"
            >
              <span className="size-2 rounded-full" style={{ backgroundColor: slice.color }} />
              {slice.label}
            </span>
          ))}
      </div>
      <div className="relative flex gap-2" style={{ height }}>
        <div className="relative w-8 shrink-0 font-mono text-[10px] text-fg-faint">
          {ticks.map((tick) => (
            <span
              key={tick}
              className="absolute right-0 translate-y-1/2"
              style={{ bottom: `${(tick / top) * 100}%` }}
            >
              {formatHours(tick, "0")}
            </span>
          ))}
        </div>
        <div className="relative min-w-0 flex-1">
          {ticks.map((tick) => (
            <div
              key={tick}
              className="pointer-events-none absolute right-0 left-0 border-line-soft border-t"
              style={{ bottom: `${(tick / top) * 100}%` }}
            />
          ))}
          <div className="absolute inset-0 flex items-end justify-around">
            {columns.map((column, index) => (
              <motion.div
                key={column.key}
                className="flex h-full min-w-0 flex-1 flex-col-reverse items-center"
                initial={reduced ? false : { opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ ...spring, delay: reduced ? 0 : 0.02 * index }}
              >
                <motion.div
                  className="flex w-full max-w-6 flex-col-reverse overflow-hidden rounded-t"
                  style={{
                    gap: totals[index] > 0 ? GAP_PX : 0,
                  }}
                  initial={reduced ? false : { height: 0 }}
                  animate={{ height: `${(totals[index] / top) * 100}%` }}
                  transition={{ ...spring, delay: reduced ? 0 : 0.02 * index + 0.05 }}
                >
                  {column.slices
                    .filter((slice) => slice.ms > 0)
                    .map((slice) => (
                      <motion.span
                        key={slice.key}
                        onMouseEnter={(event) =>
                          setTip({
                            column,
                            slice,
                            x:
                              event.currentTarget.getBoundingClientRect().left -
                              (event.currentTarget.closest("figure")?.getBoundingClientRect()
                                .left ?? 0),
                          })
                        }
                        onMouseLeave={() => setTip(null)}
                        className="block w-full shrink-0"
                        style={{
                          flexGrow: slice.ms,
                          flexBasis: 0,
                          backgroundColor: slice.color,
                        }}
                        initial={reduced ? false : { opacity: 0 }}
                        animate={{ opacity: 1 }}
                        transition={{ ...spring, delay: reduced ? 0 : 0.02 * index + 0.1 }}
                      />
                    ))}
                </motion.div>
              </motion.div>
            ))}
          </div>
        </div>
        {tip && (
          <motion.div
            role="tooltip"
            className="pointer-events-none absolute top-0 z-10 rounded-md border border-line bg-panel px-2 py-1 text-[11px] shadow-lg"
            style={{ left: Math.max(0, tip.x - 20) }}
            initial={reduced ? false : { opacity: 0, y: -4 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.15 }}
          >
            <div className="font-semibold text-fg-strong">{tip.column.label}</div>
            <div className="flex items-center gap-1.5 text-fg-muted">
              <span className="size-2 rounded-full" style={{ backgroundColor: tip.slice.color }} />
              {tip.slice.label} · {formatDuration(tip.slice.ms)}
            </div>
          </motion.div>
        )}
      </div>
      <div className="mt-1 flex gap-2">
        <div className="w-8 shrink-0" />
        <div className="flex min-w-0 flex-1 justify-around">
          {columns.map((column, index) => (
            <span
              key={column.key}
              className="min-w-0 flex-1 truncate text-center text-[10px] text-fg-faint"
            >
              {index % labelEvery === 0 ? column.label : ""}
            </span>
          ))}
        </div>
      </div>
    </figure>
  );
}

/** A labelled bar for ranked lists ("top apps"). */
export function BarRow({
  label,
  ms,
  maxMs,
  color = "var(--accent)",
  trailing,
  large = false,
}: {
  label: ReactNode;
  ms: number;
  maxMs: number;
  color?: string;
  trailing?: ReactNode;
  large?: boolean;
}) {
  const reduced = useReducedMotion();
  const spring = reduced ? { duration: 0 } : { type: "spring" as const, ...CHART_MOTION.spring };
  const width = maxMs > 0 ? (ms / maxMs) * 100 : 0;

  return (
    <div className="space-y-1">
      <div
        className={`flex items-center justify-between gap-2 ${
          large ? "text-[14px]" : "text-[11.5px]"
        }`}
      >
        <span className="min-w-0 truncate text-fg-muted">{label}</span>
        <span className="shrink-0 font-mono text-fg-soft tabular-nums">
          {trailing ?? formatDuration(ms)}
        </span>
      </div>
      <div className="h-1.5 w-full overflow-hidden rounded-full bg-surface">
        <motion.div
          className="h-full rounded-full"
          style={{ backgroundColor: color }}
          initial={reduced ? false : { width: 0 }}
          animate={{ width: `${width}%` }}
          transition={spring}
        />
      </div>
    </div>
  );
}
