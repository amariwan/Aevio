import type { ReactNode } from "react";

import { useI18n } from "@/hooks/useI18n";
import { Tooltip } from "@/components/primitives/Tooltip";
import { type BreakEntry, breakEnd } from "@/lib/breaks";
import { formatDuration, formatTime } from "@/lib/format";
import type { TimeEntry } from "@/lib/types";
import { clampStyle } from "./entryBlockLayout";

export function CalendarColumn({ children }: { children: ReactNode }) {
  return <div className="relative min-w-0 flex-1 border-line border-l">{children}</div>;
}

export function LaneHeader({ label, icon }: { label: string; icon: ReactNode }) {
  return (
    <span className="flex min-w-0 flex-1 items-center gap-1.5 truncate border-line border-l px-2 font-semibold text-[10.5px] text-fg-faint uppercase tracking-wider">
      {icon}
      {label}
    </span>
  );
}

export function LaneIconHeader({ label, children }: { label: string; children: ReactNode }) {
  return (
    <span className="flex min-w-0 flex-1 items-center gap-1.5 border-line border-l px-2 font-semibold text-[10.5px] text-fg-faint uppercase tracking-wider">
      <span className="sr-only">{label}</span>
      <span className="size-3.5">{children}</span>
    </span>
  );
}

interface CalendarLaneProps {
  entries: TimeEntry[];
  getLabel: (entry: TimeEntry) => string | undefined;
  getPosition: (entry: TimeEntry) => { top: number; height: number };
  getColor?: (entry: TimeEntry) => string | undefined;
  onSelect: (id: string) => void;
  selectedId?: string;
}

export function CalendarLane({
  entries,
  getLabel,
  getPosition,
  getColor,
  onSelect,
  selectedId,
}: CalendarLaneProps) {
  return (
    <CalendarColumn>
      {entries.map((entry) => {
        const label = getLabel(entry);
        if (!label) return null;
        const { top, height } = getPosition(entry);
        return (
          <LaneBlock
            key={entry.id}
            entryId={entry.id}
            top={top}
            height={height}
            selected={entry.id === selectedId}
            color={getColor?.(entry)}
            label={label}
            onSelect={onSelect}
          />
        );
      })}
    </CalendarColumn>
  );
}

interface BreakBandProps {
  entry: BreakEntry;
  top: number;
  height: number;
  now: number;
}

const TEXT_MIN_HEIGHT = 12;

export function BreakBand({ entry, top, height, now }: BreakBandProps) {
  const { t } = useI18n();
  const start = entry.startedAt ?? now;
  const length = formatDuration(Math.max(0, breakEnd(entry, now) - start));
  return (
    <Tooltip content={`${formatTime(start)} · ${length}`}>
      <div
        className="pointer-events-none absolute right-4 left-0 z-0 overflow-hidden rounded-md border border-line-strong bg-surface-strong"
        style={{ top: `${top}px`, height: `${height}px` }}
        role="img"
        aria-label={`Break, ${length}`}
      >
        {height >= TEXT_MIN_HEIGHT && (
          <div className="flex h-full items-center gap-1.5 px-2 font-medium text-[10.5px] text-fg-soft leading-none">
            <span className="shrink-0">{t("calendar.break")}</span>
            <span className="truncate tabular-nums">{length}</span>
          </div>
        )}
      </div>
    </Tooltip>
  );
}

const LANE_LINE_HEIGHT = 13.125;
const LANE_PADDING = 4;

interface LaneBlockProps {
  entryId: string;
  top: number;
  height: number;
  selected: boolean;
  color?: string;
  label: string;
  onSelect: (id: string) => void;
}

function LaneBlock({ entryId, top, height, selected, color, label, onSelect }: LaneBlockProps) {
  const tone = color ?? "var(--fg-faint)";
  const lineCount = Math.max(1, Math.floor((height - LANE_PADDING) / LANE_LINE_HEIGHT));
  return (
    <Tooltip content={label}>
      <button
        type="button"
        onClick={() => onSelect(entryId)}
        aria-label={label}
        className={`calendar-entry absolute right-1 left-0.5 flex items-center overflow-hidden rounded-md px-1.5 text-left font-medium text-[10.5px] leading-tight transition-all ${
          selected ? "z-20 ring-1 ring-accent" : "z-10 hover:border-fg-soft/40"
        }`}
        style={{
          top: `${top}px`,
          height: `${height}px`,
          backgroundColor: `color-mix(in srgb, ${tone} 15%, var(--bg-panel))`,
          color: tone,
        }}
      >
        {height >= TEXT_MIN_HEIGHT && (
          <span
            className="min-w-0"
            style={{
              ...clampStyle(lineCount),
              ...(height < LANE_LINE_HEIGHT + LANE_PADDING && { lineHeight: 1 }),
            }}
          >
            {label}
          </span>
        )}
      </button>
    </Tooltip>
  );
}

interface NowLineProps {
  top: number;
  recording?: { startedAt: number; now: number };
  compact?: boolean;
}

export function NowLine({ top, recording, compact = false }: NowLineProps) {
  if (!recording) {
    return (
      <div
        className="pointer-events-none absolute right-0 left-0 z-30 flex -translate-y-1/2 items-center"
        style={{ top: `${top}px` }}
        aria-hidden="true"
      >
        <span className="-ml-1 size-2 rounded-full bg-danger" />
        <span className="h-px flex-1 bg-danger/70" />
      </div>
    );
  }
  const elapsed = formatDuration(recording.now - recording.startedAt);
  return (
    <Tooltip content={`Recording since ${formatTime(recording.startedAt)}`}>
      <div
        className="pointer-events-none absolute right-0 left-0 z-30 flex -translate-y-1/2 items-center"
        style={{ top: `${top}px` }}
      >
        <span className="recording-dot -ml-1 size-2 shrink-0 rounded-full bg-danger" />
        <span className="h-0.5 flex-1 rounded-full bg-danger" />
        {compact ? (
          <span className="sr-only">Recording, {elapsed}</span>
        ) : (
          <span className="-ml-px flex shrink-0 items-center gap-1 rounded-full bg-danger py-0.5 pr-2 pl-1.5 font-semibold text-[10px] text-canvas leading-none tabular-nums shadow-sm">
            <span className="recording-dot size-1.5 rounded-full bg-canvas" />
            Recording · {elapsed}
          </span>
        )}
      </div>
    </Tooltip>
  );
}
