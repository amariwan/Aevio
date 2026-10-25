import { useCallback, useEffect, useMemo, useState } from "react";
import { Globe2, LockKeyhole, Share2, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import * as api from "@/lib/api";
import type { TimelineEvent, TimelineSyncState } from "@/lib/types";
import { TimelineSyncControls } from "./TimelineSyncControls";

function dayBounds(date: string): { start: number; end: number } {
  const start = new Date(`${date}T00:00:00`).getTime();
  return { start, end: start + 86_400_000 };
}

function formatTime(value: number): string {
  return new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" }).format(value);
}

function formatDuration(value: number): string {
  const minutes = Math.max(0, Math.round(value / 60_000));
  return minutes < 60 ? `${minutes} min` : `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
}

function EventRow({ event, onToggle }: { event: TimelineEvent; onToggle: () => void }) {
  const shared = event.visibility === "shared";
  return (
    <div className="flex items-center gap-3 border-b border-line px-4 py-3 last:border-b-0">
      <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-surface text-fg-muted">
        {shared ? <Globe2 className="size-4" /> : <LockKeyhole className="size-4" />}
      </div>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-fg">{event.title || event.app}</p>
        <p className="text-xs text-fg-muted">
          {event.app} · {formatTime(event.startedAt)}–{formatTime(event.endedAt)} ·{" "}
          {formatDuration(event.durationMs)}
        </p>
      </div>
      <Button variant="ghost" size="sm" onClick={onToggle}>
        {shared ? "Make private" : "Share"}
      </Button>
    </div>
  );
}

export function Timeline() {
  const now = new Date();
  const today = [now.getFullYear(), now.getMonth() + 1, now.getDate()]
    .map((part, index) => (index === 0 ? String(part) : String(part).padStart(2, "0")))
    .join("-");
  const [date, setDate] = useState(today);
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [sync, setSync] = useState<TimelineSyncState>();
  const [workspaceId, setWorkspaceId] = useState("");
  const [error, setError] = useState<string>();
  const bounds = useMemo(() => dayBounds(date), [date]);

  const load = useCallback(async () => {
    try {
      setError(undefined);
      const [nextEvents, nextSync] = await Promise.all([
        api.timelineDay(bounds.start, bounds.end, workspaceId.trim() || undefined),
        api.timelineSyncState(),
      ]);
      setEvents(nextEvents);
      setSync(nextSync);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }, [bounds, workspaceId]);

  useEffect(() => {
    void load();
  }, [load]);

  const toggle = async (event: TimelineEvent) => {
    try {
      if (event.visibility === "shared") {
        await api.unpublishTimelineSegment(Number(event.id.replace("segment-", "")));
      } else {
        if (!workspaceId.trim()) {
          throw new Error("Enter a workspace ID before sharing activity.");
        }
        await api.publishTimelineSegment({
          segmentId: Number(event.id.replace("segment-", "")),
          workspaceId: workspaceId.trim(),
          title: event.title || `${event.app} activity`,
        });
      }
      await load();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const trackedMs = events.reduce((total, event) => total + event.durationMs, 0);
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto p-5">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <p className="text-xs font-semibold tracking-wider text-fg-muted uppercase">
            Personal timeline
          </p>
          <h1 className="mt-1 text-2xl font-semibold text-fg">What you did today</h1>
          <p className="mt-1 text-sm text-fg-muted">
            Automatically tracked on your devices. Private until you share it.
          </p>
        </div>
        <input
          className="rounded-md border border-line bg-surface px-3 py-2 text-sm text-fg"
          type="date"
          value={date}
          onChange={(event) => setDate(event.target.value)}
        />
      </div>
      <Card>
        <CardContent className="flex flex-wrap items-center gap-3 p-4">
          <div className="min-w-48 flex-1">
            <label className="text-sm font-medium text-fg" htmlFor="timeline-workspace">
              Workspace for shared activity
            </label>
            <p className="mt-1 text-xs text-fg-muted">
              Private activity stays local. Only explicit shares enter this workspace.
            </p>
          </div>
          <Input
            id="timeline-workspace"
            className="max-w-xs"
            placeholder="workspace ID"
            value={workspaceId}
            onChange={(event) => setWorkspaceId(event.target.value)}
          />
        </CardContent>
      </Card>
      <TimelineSyncControls workspaceId={workspaceId} onSynced={load} />
      <div className="grid gap-3 md:grid-cols-3">
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Tracked</CardTitle>
          </CardHeader>
          <CardContent className="text-2xl font-semibold">{formatDuration(trackedMs)}</CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Private</CardTitle>
          </CardHeader>
          <CardContent className="text-2xl font-semibold">
            {events.filter((event) => event.visibility !== "shared").length}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Sync</CardTitle>
          </CardHeader>
          <CardContent className="flex items-center gap-2 text-sm">
            <ShieldCheck className="size-4 text-accent" />
            {sync?.mode === "server" ? sync.status : "Local only"}
          </CardContent>
        </Card>
      </div>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Share2 className="size-4" />
            Activity timeline
          </CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          {error && <p className="border-b border-line px-4 py-3 text-sm text-review">{error}</p>}
          {events.length === 0 ? (
            <p className="px-4 py-8 text-center text-sm text-fg-muted">
              No tracked activity for this day.
            </p>
          ) : (
            events.map((event) => (
              <EventRow key={event.id} event={event} onToggle={() => void toggle(event)} />
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
