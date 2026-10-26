import { type KeyboardEvent, type PointerEvent, useMemo, useRef, useState } from "react";

import { EmptyState } from "@/components/layout/Page";
import { Tooltip } from "@/components/primitives/Tooltip";
import { type BreakEntry, breakEnd, takenBreaks } from "@/lib/breaks";
import { entryEnd } from "@/lib/entries";
import { formatTime } from "@/lib/format";
import type { ActivitySegment, Category, Client, Project, Task, TimeEntry } from "@/lib/types";
import type { CalendarColumn as CalendarColumnKey } from "./columns";
import { CalendarColumn, CalendarLane, BreakBand } from "./DayViewElements";
import { EntryBlock } from "./EntryBlock";
import { SplitPicker } from "./SplitPicker";
import { place, type Timeline } from "./timeline";

const SNAP_MS = 300_000;

interface DayViewLanesProps {
  visibleColumns: CalendarColumnKey[];
  dayStart: number;
  dayEnd: number;
  hourHeight: number;
  timeline: Timeline;
  entries: TimeEntry[];
  segments: ActivitySegment[];
  breaks: BreakEntry[];
  loading: boolean;
  selectedId?: string;
  categoryById: Map<string, Category>;
  projectById: Map<string, Project>;
  clientById: Map<string, Client>;
  tasks: Task[];
  recordingId?: string;
  splitPick?: { id: string; at: number };
  onPickSplit?: (at: number) => void;
  onSelect: (id: string) => void;
  onCreate: (startMs: number, endMs: number) => void;
  now: number;
}

interface ProjectMetadata {
  tasksByProject: Map<string, Task[]>;
  labelsByProject: Map<string, string[]>;
}

function projectMetadata(tasks: Task[]): ProjectMetadata {
  const tasksByProject = new Map<string, Task[]>();
  const labelsByProject = new Map<string, string[]>();
  for (const task of tasks) {
    tasksByProject.set(task.projectId, [...(tasksByProject.get(task.projectId) ?? []), task]);
    const labels = labelsByProject.get(task.projectId) ?? [];
    for (const label of task.labels) {
      if (!labels.includes(label.name)) labels.push(label.name);
    }
    labelsByProject.set(task.projectId, labels);
  }
  return { tasksByProject, labelsByProject };
}

function dragRange(
  startY: number,
  startTime: number,
  currentY: number,
  currentTime: number,
  dayStart: number,
  dayEnd: number,
): { start: number; end: number } | null {
  if (Math.abs(currentY - startY) < 3) return null;
  if (currentY >= startY) {
    const start = Math.max(dayStart, Math.floor(startTime / SNAP_MS) * SNAP_MS);
    const end = Math.min(dayEnd, Math.round(currentTime / SNAP_MS) * SNAP_MS);
    return end - start >= SNAP_MS ? { start, end } : null;
  }
  const end = Math.min(dayEnd, Math.ceil(startTime / SNAP_MS) * SNAP_MS);
  const start = Math.max(dayStart, Math.round(currentTime / SNAP_MS) * SNAP_MS);
  return end - start >= SNAP_MS ? { start, end } : null;
}

export function DayViewLanes(props: DayViewLanesProps) {
  const { tasksByProject, labelsByProject } = useMemo(
    () => projectMetadata(props.tasks),
    [props.tasks],
  );
  const entryById = useMemo(
    () => new Map(props.entries.map((entry) => [entry.id, entry])),
    [props.entries],
  );
  const splitting = props.splitPick ? entryById.get(props.splitPick.id) : undefined;
  const getLiveEnd = (entry: TimeEntry): number =>
    entry.status === "building"
      ? Math.min(Math.max(entry.startedAt, props.now), props.dayEnd)
      : entry.endedAt;
  const getPosition = (entry: TimeEntry): { top: number; height: number } =>
    place(props.timeline, entry.startedAt, getLiveEnd(entry), props.dayStart, "elapsed");

  return props.visibleColumns.map((column) => {
    if (column === "activity") {
      return <ActivityLane key={column} {...props} entryById={entryById} />;
    }
    if (column === "timeEntries") {
      return (
        <TimeEntriesLane
          key={column}
          {...props}
          getLiveEnd={getLiveEnd}
          getPosition={getPosition}
          splitting={splitting}
        />
      );
    }
    const getLabel = (entry: TimeEntry): string | undefined => {
      if (!entry.projectId && column !== "sessions" && column !== "calendar") return undefined;
      switch (column) {
        case "labels":
          return labelsByProject.get(entry.projectId ?? "")?.join(", ");
        case "tasks":
          return tasksByProject
            .get(entry.projectId ?? "")
            ?.map((task) => task.title)
            .join(", ");
        case "projects":
          return props.projectById.get(entry.projectId ?? "")?.name;
        case "clients": {
          const project = props.projectById.get(entry.projectId ?? "");
          return project?.clientId ? props.clientById.get(project.clientId)?.name : undefined;
        }
        case "sessions":
        case "calendar":
          return `${formatTime(entry.startedAt)}–${formatTime(getLiveEnd(entry))}`;
        default:
          return undefined;
      }
    };
    const getColor =
      column === "projects"
        ? (entry: TimeEntry) => props.projectById.get(entry.projectId ?? "")?.color
        : undefined;
    return (
      <CalendarLane
        key={column}
        entries={props.entries}
        getLabel={getLabel}
        getColor={getColor}
        getPosition={getPosition}
        onSelect={props.onSelect}
        selectedId={props.selectedId}
      />
    );
  });
}

function ActivityLane({
  segments,
  entryById,
  timeline,
  dayStart,
  dayEnd,
  now,
}: DayViewLanesProps & { entryById: Map<string, TimeEntry> }) {
  return (
    <CalendarColumn>
      {segments.map((segment) => {
        if (segment.kind === "break") return null;
        const end = segment.endedAt ?? Math.min(now, dayEnd);
        const { top, height } = place(timeline, segment.startedAt, end, dayStart, "elapsed", 3);
        const owner = segment.entryId ? entryById.get(segment.entryId) : undefined;
        const timeLabel = `${formatTime(segment.startedAt)}–${formatTime(end)}`;
        return (
          <Tooltip key={segment.id} content={`${segment.app}: ${segment.title}\n${timeLabel}`}>
            <div
              className="calendar-entry absolute right-1 left-0.5 flex flex-col overflow-hidden rounded-md border border-line px-1.5 py-1 text-left transition-all"
              style={{
                top: `${top}px`,
                height: `${height}px`,
                backgroundColor: owner
                  ? "color-mix(in srgb, var(--accent) 15%, var(--bg-panel))"
                  : "var(--bg-panel)",
                color: owner ? "var(--accent)" : "var(--fg-faint)",
              }}
              aria-label={`${segment.app}: ${segment.title}, ${timeLabel}`}
            >
              <span className="truncate font-semibold text-[10.5px] leading-tight">
                {segment.app}
              </span>
              <span className="truncate text-[10px] text-fg-soft leading-tight">
                {segment.title}
              </span>
              <span className="truncate text-[9px] text-fg-faint tabular-nums leading-tight">
                {timeLabel}
              </span>
            </div>
          </Tooltip>
        );
      })}
    </CalendarColumn>
  );
}

interface TimeEntriesLaneProps extends DayViewLanesProps {
  getLiveEnd: (entry: TimeEntry) => number;
  getPosition: (entry: TimeEntry) => { top: number; height: number };
  splitting?: TimeEntry;
}

function TimeEntriesLane(props: TimeEntriesLaneProps) {
  const column = useRef<HTMLButtonElement>(null);
  const press = useRef<{ y: number; time: number } | null>(null);
  const [draft, setDraft] = useState<{ start: number; end: number } | null>(null);
  const timeAt = (clientY: number): number => {
    const bounds = column.current?.getBoundingClientRect();
    const offset = bounds ? (clientY - bounds.top) / props.hourHeight : 0;
    return Math.max(props.dayStart, Math.min(props.dayEnd, props.dayStart + offset * 3_600_000));
  };
  const rangeAt = (clientY: number): { start: number; end: number } | null => {
    if (!press.current) return null;
    return dragRange(
      press.current.y,
      press.current.time,
      clientY,
      timeAt(clientY),
      props.dayStart,
      props.dayEnd,
    );
  };
  const handlePointerDown = (event: PointerEvent<HTMLButtonElement>): void => {
    if (event.button !== 0) return;
    press.current = { y: event.clientY, time: timeAt(event.clientY) };
    setDraft(null);
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  const handlePointerUp = (event: PointerEvent<HTMLButtonElement>): void => {
    const range = rangeAt(event.clientY);
    press.current = null;
    setDraft(null);
    if (range) props.onCreate(range.start, range.end);
  };
  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>): void => {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    const time = Math.max(props.dayStart, Math.min(props.now, props.dayEnd - SNAP_MS));
    const start = Math.min(props.dayEnd - SNAP_MS, Math.floor(time / SNAP_MS) * SNAP_MS);
    props.onCreate(start, start + SNAP_MS);
  };

  return (
    <CalendarColumn>
      <Tooltip content="Drag to add a session">
        <button
          ref={column}
          type="button"
          aria-label="Drag to add a session"
          className="absolute inset-0 w-full cursor-cell touch-none"
          onPointerDown={handlePointerDown}
          onPointerMove={(event) => {
            if (press.current) setDraft(rangeAt(event.clientY));
          }}
          onPointerUp={handlePointerUp}
          onPointerCancel={() => {
            press.current = null;
            setDraft(null);
          }}
          onKeyDown={handleKeyDown}
        />
      </Tooltip>
      {props.visibleColumns.includes("sessions") &&
        takenBreaks(props.breaks).map((entry) => {
          const start = entry.startedAt ?? props.dayStart;
          const { top, height } = place(
            props.timeline,
            start,
            Math.min(breakEnd(entry, props.now), props.dayEnd),
            props.dayStart,
            "elapsed",
          );
          return (
            <BreakBand key={entry.id} entry={entry} top={top} height={height} now={props.now} />
          );
        })}
      {props.entries.map((entry) => {
        const { top, height } = props.getPosition(entry);
        return (
          <EntryBlock
            key={entry.id}
            entry={entry}
            top={top}
            height={height}
            selected={entry.id === props.selectedId}
            category={entry.categoryId ? props.categoryById.get(entry.categoryId) : undefined}
            project={entry.projectId ? props.projectById.get(entry.projectId) : undefined}
            onSelect={props.onSelect}
            now={props.now}
            recording={entry.id === props.recordingId}
          />
        );
      })}
      {props.splitting && props.splitPick && props.onPickSplit && (
        <SplitPicker
          {...place(
            props.timeline,
            props.splitting.startedAt,
            props.getLiveEnd(props.splitting),
            props.dayStart,
            "elapsed",
          )}
          startedAt={props.splitting.startedAt}
          endsAt={entryEnd(props.splitting, props.now)}
          initialAt={props.splitPick.at}
          timeline={props.timeline}
          dayStart={props.dayStart}
          placement="elapsed"
          onPick={props.onPickSplit}
        />
      )}
      {draft && (
        <div
          className="pointer-events-none absolute right-4 left-0 z-20 rounded-md border border-accent bg-accent/20"
          style={{
            top: `${((draft.start - props.dayStart) / 3_600_000) * props.hourHeight}px`,
            height: `${((draft.end - draft.start) / 3_600_000) * props.hourHeight}px`,
          }}
        />
      )}
      {props.entries.length === 0 && !props.loading && (
        <div className="pointer-events-none absolute inset-x-0 top-16">
          <EmptyState
            title={
              props.dayStart > props.now ? "Nothing here yet" : "No entries recorded for this day"
            }
            hint={
              props.dayStart > props.now
                ? "This day hasn't happened yet."
                : "Activity appears here automatically as you use your computer."
            }
          />
        </div>
      )}
    </CalendarColumn>
  );
}
