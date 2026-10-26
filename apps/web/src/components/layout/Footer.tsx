import { useCallback, useEffect, useRef, useState } from "react";
import { BriefcaseBusiness, ChevronDown, Coffee, Pause, Play, Power, Target } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tooltip } from "../primitives/Tooltip";
import { useI18n } from "@/hooks/useI18n";
import type { SessionKind } from "@/lib/types";

interface FooterProps {
  trackingActive?: boolean;
  captureEnabled?: boolean;
  currentApp?: string;
  currentKind?: SessionKind;
  onToggleCapture?: () => void;
  onStartSession?: (kind: "focus" | "meeting" | "break") => void;
  onStartBreak?: () => void;
  pendingCount?: number;
  version?: string;
}

function formatElapsed(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3_600);
  const minutes = Math.floor((totalSeconds % 3_600) / 60);
  const seconds = totalSeconds % 60;
  return [hours, minutes, seconds].map((value) => String(value).padStart(2, "0")).join(":");
}

/** Compact app status bar with version, music, and focus controls. */
export function Footer({
  trackingActive = false,
  currentKind,
  onToggleCapture,
  onStartSession,
  onStartBreak,
  pendingCount = 0,
  version,
}: FooterProps) {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const [trackName, setTrackName] = useState<string | null>(null);
  const [trackUrl, setTrackUrl] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const [activeSince, setActiveSince] = useState<number | null>(trackingActive ? Date.now() : null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [breakDialogOpen, setBreakDialogOpen] = useState(false);
  const [breakDuration, setBreakDuration] = useState("5");
  const { t } = useI18n();

  useEffect(() => {
    if (trackingActive) {
      setActiveSince((startedAt) => startedAt ?? Date.now());
      return;
    }
    setActiveSince(null);
    setElapsedSeconds(0);
  }, [trackingActive]);

  useEffect(() => {
    if (!activeSince) return;
    const updateElapsed = () => setElapsedSeconds(Math.floor((Date.now() - activeSince) / 1_000));
    updateElapsed();
    const id = window.setInterval(updateElapsed, 1_000);
    return () => window.clearInterval(id);
  }, [activeSince]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    if (trackUrl) {
      audio.src = trackUrl;
      if (playing) void audio.play().catch(() => setPlaying(false));
    } else {
      audio.removeAttribute("src");
      audio.load();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [trackUrl]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    if (playing) void audio.play().catch(() => setPlaying(false));
    else audio.pause();
  }, [playing]);

  const handlePickFile = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      if (!file) return;
      if (trackUrl) URL.revokeObjectURL(trackUrl);
      setTrackUrl(URL.createObjectURL(file));
      setTrackName(file.name.replace(/\.[^.]+$/, ""));
      setPlaying(true);
      event.target.value = "";
    },
    [trackUrl],
  );

  const togglePlay = useCallback(() => {
    if (!trackUrl) {
      fileInputRef.current?.click();
      return;
    }
    setPlaying((isPlaying) => !isPlaying);
  }, [trackUrl]);

  const selectSession = (kind: "focus" | "meeting"): void => {
    onStartSession?.(kind);
  };

  const startBreak = (): void => {
    onStartBreak?.();
    setBreakDialogOpen(false);
  };

  const statusLabel =
    currentKind === "focus"
      ? "Focus"
      : currentKind === "meeting"
        ? "Meeting"
        : currentKind === "break"
          ? "Break"
          : trackingActive
            ? "Tracking"
            : "Paused";

  return (
    <footer className="flex h-11 shrink-0 items-center gap-3 border-t border-line bg-bar px-3 text-[11px]">
      <audio ref={audioRef} onEnded={() => setPlaying(false)} preload="none" />

      <div className="flex min-w-0 shrink-0 items-center gap-2 text-fg-faint">
        {onToggleCapture && (
          <Tooltip content={trackingActive ? "Pause tracking" : "Start tracking"}>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={onToggleCapture}
              aria-label={trackingActive ? "Pause tracking" : "Start tracking"}
              className="rounded-md border border-line bg-surface text-fg-muted hover:bg-surface-strong hover:text-fg"
            >
              <Power
                className={trackingActive ? "text-accent" : "text-fg-faint"}
                aria-hidden="true"
              />
            </Button>
          </Tooltip>
        )}
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button variant="outline" size="sm" className="gap-2 border-line bg-surface px-3" />
            }
          >
            <span className="hidden sm:inline">End Focus</span>
            <ChevronDown aria-hidden="true" />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" side="top" className="min-w-40">
            <DropdownMenuItem onClick={() => selectSession("focus")}>
              <Target aria-hidden="true" />
              Focus
              <span className="ml-auto text-xs text-fg-faint">⌘ F</span>
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => selectSession("meeting")}>
              <BriefcaseBusiness aria-hidden="true" />
              Meeting
              <span className="ml-auto text-xs text-fg-faint">⌘ M</span>
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setBreakDialogOpen(true)}>
              <Coffee aria-hidden="true" />
              Break
              <span className="ml-auto text-xs text-fg-faint">⌘ B</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        <div className="hidden items-center gap-2 sm:flex">
          <span className="font-mono font-semibold text-fg tabular-nums">
            {formatElapsed(elapsedSeconds)}
          </span>
          <span className="text-fg-muted">{statusLabel}</span>
        </div>
      </div>

      <div className="flex min-w-0 flex-1 items-center justify-center gap-2">
        <Tooltip content={trackUrl ? (playing ? "Pause" : "Play") : "Choose a track"}>
          <button
            type="button"
            onClick={togglePlay}
            aria-label={
              trackUrl
                ? playing
                  ? t("footer.pauseMusic")
                  : t("footer.playMusic")
                : t("footer.chooseTrack")
            }
            className="grid size-7 place-items-center rounded-md border border-line bg-surface text-fg-soft transition-colors hover:bg-surface-strong hover:text-fg"
          >
            {playing ? (
              <Pause className="size-3.5 fill-current" aria-hidden="true" />
            ) : (
              <Play className="size-3.5 fill-current" aria-hidden="true" />
            )}
          </button>
        </Tooltip>
        <input
          ref={fileInputRef}
          type="file"
          accept="audio/*"
          className="hidden"
          onChange={handlePickFile}
        />
        <Tooltip content={t("footer.pickAudioFile")}>
          <button
            type="button"
            onClick={() => fileInputRef.current?.click()}
            aria-label={t("footer.pickAudioFile")}
            className="grid size-7 place-items-center rounded-md border border-line bg-surface text-fg-soft transition-colors hover:bg-surface-strong hover:text-fg"
          >
            <svg
              viewBox="0 0 24 24"
              className="size-3.5"
              fill="none"
              stroke="currentColor"
              strokeWidth={2}
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d="M9 18V5l12-2v13" />
              <circle cx="6" cy="18" r="3" />
              <circle cx="18" cy="16" r="3" />
            </svg>
          </button>
        </Tooltip>
        <span className="hidden max-w-48 truncate text-fg-muted sm:block">
          {trackName ?? "No track selected"}
        </span>
      </div>

      <div className="flex shrink-0 items-center gap-3 text-fg-faint">
        {pendingCount > 0 && (
          <span className="rounded-full bg-review/20 px-2 py-0.5 font-bold text-review">
            {t("review.pending", { count: pendingCount })}
          </span>
        )}
        {version && <span className="font-mono text-fg-faint select-none">v{version}</span>}
      </div>

      <Dialog open={breakDialogOpen} onOpenChange={setBreakDialogOpen}>
        <DialogContent className="max-w-[520px] gap-0 p-0">
          <DialogHeader className="border-b border-line px-5 py-4">
            <DialogTitle>Start Break</DialogTitle>
          </DialogHeader>
          <div className="space-y-5 px-5 py-5">
            <div className="space-y-2">
              <label className="flex items-center gap-1 text-sm font-medium text-fg-muted">
                Duration <span className="text-fg-faint">?</span>
              </label>
              <DialogDescription>
                Breaks should be free of obligations like checking email or social media. Engage in
                mentally restful activities like walking or stretching.
              </DialogDescription>
              <Select
                value={breakDuration}
                onValueChange={(value) => {
                  if (value) setBreakDuration(value);
                }}
              >
                <SelectTrigger className="w-full bg-surface">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {[5, 10, 15, 20, 30].map((minutes) => (
                    <SelectItem key={minutes} value={String(minutes)}>
                      {minutes} min
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <label className="flex items-center gap-1 text-sm font-medium text-fg-muted">
                Music <span className="text-fg-faint">?</span>
              </label>
              <div className="flex items-center gap-2">
                <div className="flex min-w-0 flex-1 items-center gap-3 rounded-lg border border-line bg-surface px-3 py-2">
                  <div
                    className="size-8 shrink-0 rounded-md bg-surface-strong"
                    aria-hidden="true"
                  />
                  <span className="truncate text-sm text-fg">{trackName ?? "Silence"}</span>
                </div>
                <Button variant="outline" onClick={() => fileInputRef.current?.click()}>
                  Change
                </Button>
              </div>
            </div>
          </div>
          <DialogFooter className="flex-row justify-end gap-2">
            <Button variant="ghost" onClick={() => setBreakDialogOpen(false)}>
              Cancel
            </Button>
            <Button onClick={startBreak}>Start Break</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </footer>
  );
}
