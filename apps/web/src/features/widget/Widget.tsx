import { type PointerEvent, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  BriefcaseBusiness,
  ChevronDown,
  ChevronUp,
  CirclePause,
  Coffee,
  Menu,
  Play,
  Square,
  Target,
} from "lucide-react";
import { CoachWidgetContext } from "@/features/coach/components/WidgetContext";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { useTimers } from "@/features/timers/hooks/useTimers";
import { SettingsProvider, useSettings } from "@/hooks/useSettings";
import { I18nProvider, useI18n } from "@/hooks/useI18n";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import * as api from "@/lib/api";
import { startOfToday } from "@/lib/activity";
import { dailyTargetMs } from "@/lib/settings";
import { elapsedMs, formatDuration } from "@/lib/timers";
import type { ActivitySegment, ActivitySnapshot, ActivityTick } from "@/lib/types";
import { cn } from "cn";

const COMPACT_HEIGHT = 54;
const EXPANDED_HEIGHT = 320;

export default function Widget() {
  return (
    <SettingsProvider>
      <I18nProvider>
        <WidgetContent />
      </I18nProvider>
    </SettingsProvider>
  );
}

function WidgetContent() {
  const { settings } = useSettings();
  const { t } = useI18n();
  const timers = useTimers();
  const [live, setLive] = useState<ActivityTick | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [expanded, setExpanded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const interval = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    void api
      .fetchActivitySnapshot(startOfToday())
      .then(setLive)
      .catch((cause: unknown) => setError(api.describeError(cause)));
  }, []);

  useEffect(() => {
    void api.resizeWidget(expanded ? EXPANDED_HEIGHT : COMPACT_HEIGHT);
  }, [expanded]);

  useTauriEvent<ActivityTick>(api.ACTIVITY_TICK, setLive);
  useTauriEvent<ActivitySnapshot>(api.ACTIVITY_CHANGED, setLive);

  const current = live?.current;
  const currentElapsed = current ? Math.max(0, now - current.startedAt) : 0;
  const target = dailyTargetMs(settings);
  const progress =
    target > 0 ? Math.min(100, Math.round(((live?.trackedMs ?? 0) / target) * 100)) : 0;

  const run = async (action: () => Promise<unknown>): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };

  const toggleSession = (): void => {
    void run(current ? api.stopSession : () => api.startSession("focus", "Focus"));
  };

  const startSession = (kind: "focus" | "meeting" | "break"): void => {
    const label = kind[0].toUpperCase() + kind.slice(1);
    void run(() => api.startSession(kind, label));
  };

  const startDragging = (event: PointerEvent<HTMLElement>): void => {
    if (event.target instanceof Element && event.target.closest("[data-widget-action]")) return;
    try {
      void getCurrentWindow().startDragging();
    } catch {
      // Not in Tauri context (e.g. browser dev).
    }
  };

  return (
    <main
      className={cn("floating-widget", expanded && "is-expanded")}
      aria-label="Aevio time tracking"
      data-tauri-drag-region
      onPointerDown={startDragging}
    >
      <header className="widget-header">
        <div className="widget-presence">
          <div className={cn("widget-pulse", current && "is-active")} aria-hidden="true" />
          <div className="widget-presence-copy">
            <span className="widget-label">
              {current ? t("widget.trackingNow") : t("widget.readyWhenYouAre")}
            </span>
            <strong>{current ? sessionLabel(current) : t("widget.noActiveSession")}</strong>
          </div>
        </div>
        <div className="widget-time-block">
          <span className="widget-label">{t("widget.elapsed")}</span>
          <strong className="widget-time">{formatDuration(currentElapsed)}</strong>
        </div>
        <Separator orientation="vertical" className="widget-divider" />
        <div className="widget-stat">
          <span className="widget-label">{t("widget.focusToday")}</span>
          <strong>{formatDuration(live?.focusMs ?? 0)}</strong>
        </div>
        <div className="widget-stat widget-progress">
          <span className="widget-label">{t("widget.dailyGoal")}</span>
          <strong>{progress}%</strong>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          data-widget-action
          className="widget-session-action"
          aria-label={current ? t("widget.stopSession") : t("widget.startFocus")}
          disabled={busy}
          onClick={toggleSession}
        >
          {current ? <Square className="fill-current" /> : <Play className="fill-current" />}
        </Button>
        <Button
          variant="ghost"
          size="icon-lg"
          data-widget-action
          className="widget-expand"
          aria-label={expanded ? t("widget.collapse") : t("widget.expand")}
          aria-expanded={expanded}
          onClick={() => setExpanded((value) => !value)}
        >
          {expanded ? <ChevronUp /> : <ChevronDown />}
        </Button>
        <Button
          variant="ghost"
          size="icon-lg"
          data-widget-action
          className="widget-open"
          aria-label={t("widget.openAevio")}
          onClick={() => void api.openMainWindow(false)}
        >
          <Menu aria-hidden="true" />
        </Button>
      </header>

      {expanded && (
        <section className="widget-expanded" aria-label={t("widget.controls")}>
          <CoachWidgetContext />
          <div className="widget-hero">
            <div>
              <span className="widget-section-label">TODAY&apos;S MOMENTUM</span>
              <strong>{Math.round(((live?.trackedMs ?? 0) / 3_600_000) * 10) / 10}h tracked</strong>
            </div>
            <div className="widget-hero-progress" aria-label={`${progress}% of daily goal`}>
              <span style={{ width: `${progress}%` }} />
            </div>
            <span className="widget-hero-goal">{progress}%</span>
          </div>

          <Button
            variant="outline"
            size="sm"
            data-widget-action
            className="w-full justify-between"
            onClick={() => void api.openProductivitySummary()}
          >
            Review today
            <ChevronDown data-icon="inline-end" />
          </Button>

          <div className="widget-control-group">
            <span className="widget-section-label">{t("widget.startSession")}</span>
            <div className="widget-session-controls">
              {current ? (
                <Button
                  variant="outline"
                  size="sm"
                  data-widget-action
                  disabled={busy}
                  onClick={() => void run(api.stopSession)}
                >
                  <Square data-icon="inline-start" className="fill-current" />
                  End {sessionLabel(current)}
                </Button>
              ) : (
                (["focus", "meeting", "break"] as const).map((kind) => (
                  <Button
                    key={kind}
                    variant={kind === "focus" ? "default" : "outline"}
                    size="sm"
                    data-widget-action
                    disabled={busy}
                    onClick={() => startSession(kind)}
                  >
                    {kind === "focus" && <Target data-icon="inline-start" />}
                    {kind === "meeting" && <BriefcaseBusiness data-icon="inline-start" />}
                    {kind === "break" && <Coffee data-icon="inline-start" />}
                    {kind[0].toUpperCase() + kind.slice(1)}
                  </Button>
                ))
              )}
            </div>
          </div>

          <Separator />

          <div className="widget-control-group">
            <div className="widget-section-heading">
              <span className="widget-section-label">{t("widget.stopwatches")}</span>
              <span className="widget-section-hint">{t("widget.quickAccess")}</span>
            </div>
            <div className="widget-timer-list">
              {timers.timers.length === 0 ? (
                <Button
                  variant="outline"
                  size="sm"
                  data-widget-action
                  onClick={() => void api.openMainWindow(false)}
                >
                  Create a stopwatch in Aevio
                </Button>
              ) : (
                timers.timers.slice(0, 3).map((timer) => {
                  const running = timer.startedAt !== null;
                  return (
                    <div key={timer.id} className={cn("widget-timer-row", running && "is-running")}>
                      <button
                        type="button"
                        className="widget-timer-name"
                        data-widget-action
                        onClick={() => void api.openMainWindow(false, timer.id)}
                      >
                        <span>{timer.label}</span>
                        <strong>{formatDuration(elapsedMs(timer, timers.now))}</strong>
                      </button>
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        data-widget-action
                        aria-label={running ? `Pause ${timer.label}` : `Start ${timer.label}`}
                        onClick={() => (running ? timers.pause(timer.id) : timers.start(timer.id))}
                      >
                        {running ? <CirclePause /> : <Play className="fill-current" />}
                      </Button>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {(error || timers.error) && <p className="widget-error">{error ?? timers.error}</p>}
        </section>
      )}
    </main>
  );
}

function sessionLabel(current: ActivitySegment): string {
  return current.label ?? current.title ?? current.kind;
}
