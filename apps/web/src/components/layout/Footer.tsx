import { useCallback, useEffect, useRef, useState } from "react";
import { Tooltip } from "../primitives/Tooltip";
import { useI18n } from "@/hooks/useI18n";

interface FooterProps {
  trackingActive?: boolean;
  captureEnabled?: boolean;
  currentApp?: string;
  onToggleCapture?: () => void;
  pendingCount?: number;
  version?: string;
}

function formatTime(date: Date): string {
  const hh = String(date.getHours()).padStart(2, "0");
  const mm = String(date.getMinutes()).padStart(2, "0");
  return `${hh}:${mm}`;
}

function formatDate(date: Date, locale: string): string {
  return date.toLocaleDateString(locale, { weekday: "short", day: "2-digit", month: "short" });
}

/**
 * App-wide status bar pinned to the bottom of the window: live capture
 * status, a compact music player, important at-a-glance info (pending
 * reviews, clock, version).
 */
export function Footer({
  trackingActive = false,
  captureEnabled = true,
  currentApp,
  onToggleCapture,
  pendingCount = 0,
  version,
}: FooterProps) {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const [trackName, setTrackName] = useState<string | null>(null);
  const [trackUrl, setTrackUrl] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const [now, setNow] = useState(() => new Date());
  const { t, locale } = useI18n();

  // Live clock.
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 30_000);
    return () => window.clearInterval(id);
  }, []);

  // Keep audio element in sync with chosen track.
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
      const url = URL.createObjectURL(file);
      setTrackUrl(url);
      setTrackName(file.name.replace(/\.[^.]+$/, ""));
      setPlaying(true);
      // reset input so the same file can be picked again
      event.target.value = "";
    },
    [trackUrl],
  );

  const togglePlay = useCallback(() => {
    if (!trackUrl) {
      fileInputRef.current?.click();
      return;
    }
    setPlaying((p) => !p);
  }, [trackUrl]);

  const statusText = trackingActive
    ? currentApp
      ? t("status.tracking", { app: currentApp })
      : t("status.trackingActive")
    : !captureEnabled
      ? t("status.trackingPaused")
      : t("status.outsideTrackingHours");

  return (
    <footer className="flex h-9 shrink-0 items-center gap-3 border-t border-line bg-bar px-3 text-[11.5px]">
      <audio ref={audioRef} onEnded={() => setPlaying(false)} preload="none" />

      {/* Capture status */}
      <div className="flex min-w-0 items-center gap-2">
        <span
          className={`size-2 shrink-0 rounded-full ${
            trackingActive ? "bg-accent animate-pulse" : "bg-fg-soft"
          }`}
          aria-hidden="true"
        />
        <span className="min-w-0 truncate font-medium text-fg-muted">{statusText}</span>
      </div>

      {pendingCount > 0 && (
        <span className="rounded-full bg-review/20 px-2 py-0.5 font-bold text-review">
          {t("review.pending", { count: pendingCount })}
        </span>
      )}

      {/* Music player */}
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
            className="grid size-6 place-items-center rounded-md text-fg-soft transition-colors hover:bg-surface-strong hover:text-fg"
          >
            {playing ? (
              <svg viewBox="0 0 24 24" className="size-3.5 fill-current" aria-hidden="true">
                <rect x="6" y="4" width="4" height="16" rx="1" />
                <rect x="14" y="4" width="4" height="16" rx="1" />
              </svg>
            ) : (
              <svg viewBox="0 0 24 24" className="size-3.5 fill-current" aria-hidden="true">
                <polygon points="6 4 20 12 6 20 6 4" />
              </svg>
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
            className="grid size-6 place-items-center rounded-md text-fg-soft transition-colors hover:bg-surface-strong hover:text-fg"
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
        <span className="min-w-0 max-w-64 truncate text-fg-muted">
          {trackName ?? "No track selected"}
        </span>
      </div>

      {/* Important info */}
      <div className="ml-auto flex shrink-0 items-center gap-3 text-fg-faint">
        {onToggleCapture && (
          <Tooltip
            content={
              trackingActive
                ? "Pause tracking"
                : captureEnabled
                  ? "Start tracking"
                  : "Resume tracking"
            }
          >
            <button
              type="button"
              onClick={onToggleCapture}
              aria-label={
                trackingActive
                  ? "Pause tracking"
                  : captureEnabled
                    ? "Start tracking"
                    : "Resume tracking"
              }
              className="rounded p-1 text-fg-soft transition-colors hover:bg-surface-strong hover:text-fg"
            >
              {trackingActive ? (
                <svg viewBox="0 0 24 24" className="size-3.5 fill-current" aria-hidden="true">
                  <rect x="6" y="4" width="4" height="16" rx="1" />
                  <rect x="14" y="4" width="4" height="16" rx="1" />
                </svg>
              ) : (
                <svg viewBox="0 0 24 24" className="size-3.5 fill-current" aria-hidden="true">
                  <polygon points="5 3 19 12 5 21 5 3" />
                </svg>
              )}
            </button>
          </Tooltip>
        )}
        <span className="font-mono tabular-nums select-none">
          {formatDate(now, locale)} · {formatTime(now)}
        </span>
        {version && <span className="font-mono select-none">v{version}</span>}
      </div>
    </footer>
  );
}
