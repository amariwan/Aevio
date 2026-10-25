import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  BellOff,
  CalendarDays,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CirclePause,
  Headphones,
  ListMusic,
  Music2,
  Pause,
  Pencil,
  Play,
  Plus,
  SkipBack,
  SkipForward,
  Settings2,
  ShieldCheck,
  Square,
  Volume2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useBreaks } from "@/hooks/useBreaks";
import { useI18n } from "@/hooks/useI18n";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import * as api from "@/lib/api";
import { describeError } from "@/lib/api";
import { formatDuration } from "@/lib/format";
import type { ActivitySegment, ActivitySnapshot, ActivityTick, MusicTrack } from "@/lib/types";

const WEEK_MS = 7 * 24 * 60 * 60_000;
const DAY_MS = 24 * 60 * 60_000;
const EMPTY_MUSIC_TRACK: MusicTrack = {
  id: "silence",
  title: "Silence",
  description: "A quiet room for deep work.",
  kind: "silence",
};

const EMPTY: ActivitySnapshot = {
  current: null,
  segments: [],
  trackedMs: 0,
  focusMs: 0,
  breakMs: 0,
  unreviewed: 0,
  idleMs: 0,
  idleThresholdMs: 5 * 60_000,
  captureEnabled: true,
};

function elapsed(segment: ActivitySegment | null, now: number): number {
  return segment ? Math.max(0, (segment.endedAt ?? now) - segment.startedAt) : 0;
}

function timeLabel(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function dayStart(timestamp: number): number {
  const date = new Date(timestamp);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}

function formatHours(minutes: number): string {
  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return hours > 0 ? `${hours} hr ${remainder} min` : `${remainder} min`;
}

function segmentMinutes(segment: ActivitySegment, rangeStart: number, rangeEnd: number): number {
  const start = Math.max(segment.startedAt, rangeStart);
  const end = Math.min(segment.endedAt ?? rangeEnd, rangeEnd);
  return Math.max(0, end - start) / 60_000;
}

type MusicEngine = {
  context?: AudioContext;
  master?: GainNode;
  timer?: number;
  audio?: HTMLAudioElement;
};

function createMusicEngine(track: MusicTrack, volume: number): MusicEngine | null {
  if (track.sourceUrl) {
    const audio = new Audio(track.sourceUrl);
    audio.loop = true;
    audio.volume = Math.max(0, Math.min(100, volume)) / 100;
    void audio.play();
    return { audio };
  }
  if (typeof window.AudioContext === "undefined") return null;

  const context = new window.AudioContext();
  const master = context.createGain();
  const filter = context.createBiquadFilter();
  const root =
    110 + ([...track.id].reduce((sum, character) => sum + character.charCodeAt(0), 0) % 120);

  master.gain.value = (Math.max(0, Math.min(100, volume)) / 100) * 0.16;
  filter.type = "lowpass";
  filter.frequency.value = 2_400;
  filter.Q.value = 0.5;
  master.connect(filter).connect(context.destination);

  if (track.kind === "silence") {
    void context.resume();
    return { context, master, timer: 0 };
  }

  const padGain = context.createGain();
  const pad = context.createOscillator();
  pad.type = "sine";
  pad.frequency.value = root;
  pad.detune.value = -5;
  padGain.gain.value = 0.14;
  pad.connect(padGain).connect(master);
  pad.start();

  const highPad = context.createOscillator();
  const highPadGain = context.createGain();
  highPad.type = "triangle";
  highPad.frequency.value = root * 2;
  highPad.detune.value = 7;
  highPadGain.gain.value = 0.035;
  highPad.connect(highPadGain).connect(master);
  highPad.start();

  const noiseBuffer = context.createBuffer(1, context.sampleRate * 2, context.sampleRate);
  const noiseData = noiseBuffer.getChannelData(0);
  for (let index = 0; index < noiseData.length; index += 1) {
    noiseData[index] = (Math.random() * 2 - 1) * 0.18;
  }
  const noise = context.createBufferSource();
  const noiseGain = context.createGain();
  noise.buffer = noiseBuffer;
  noise.loop = true;
  noiseGain.gain.value = 0.045;
  noise.connect(noiseGain).connect(master);
  noise.start();

  const notes = [root, root * 1.189, root * 1.498, root * 1.782];
  let step = 0;
  const playNote = (): void => {
    const note = context.createOscillator();
    const noteGain = context.createGain();
    note.type = "sine";
    note.frequency.value = notes[step % notes.length];
    noteGain.gain.setValueAtTime(0.0001, context.currentTime);
    noteGain.gain.exponentialRampToValueAtTime(0.08, context.currentTime + 0.08);
    noteGain.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + 0.78);
    note.connect(noteGain).connect(master);
    note.start();
    note.stop(context.currentTime + 0.82);
    step += 1;
  };

  playNote();
  const timer = window.setInterval(playNote, 900);
  void context.resume();
  return { context, master, timer };
}

function stopMusicEngine(engine: MusicEngine | null): void {
  if (!engine) return;
  if (engine.audio) {
    engine.audio.pause();
    engine.audio.removeAttribute("src");
    engine.audio.load();
    return;
  }
  if (engine.timer !== undefined) window.clearInterval(engine.timer);
  if (engine.context) void engine.context.close();
}

export function Focus() {
  const { t } = useI18n();
  const [snapshot, setSnapshot] = useState<ActivitySnapshot>(EMPTY);
  const [now, setNow] = useState(() => Date.now());
  const [label, setLabel] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [selectedDay, setSelectedDay] = useState(() => dayStart(Date.now()));
  const [musicOpen, setMusicOpen] = useState(false);
  const [musicPlaying, setMusicPlaying] = useState(false);
  const [musicTracks, setMusicTracks] = useState<MusicTrack[]>([]);
  const [musicTrackId, setMusicTrackId] = useState("silence");
  const [musicProgress, setMusicProgress] = useState(0);
  const [musicVolume, setMusicVolume] = useState(72);
  const musicEngine = useRef<MusicEngine | null>(null);
  const breaks = useBreaks();

  const refresh = useCallback(async () => {
    try {
      setSnapshot(await api.fetchActivitySnapshot(Date.now() - WEEK_MS));
      setError(null);
    } catch (cause) {
      setError(describeError(cause));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(interval);
  }, [refresh]);
  useEffect(() => {
    void api
      .listMusicTracks()
      .then((tracks) => {
        setMusicTracks(tracks);
        if (tracks[0])
          setMusicTrackId((current) =>
            tracks.some((track) => track.id === current) ? current : tracks[0].id,
          );
      })
      .catch((cause) => setError(describeError(cause)));
  }, []);

  const importMusic = (): void => {
    void api
      .importMusicTrack()
      .then((track) => {
        if (!track) return;
        setMusicTracks((current) => [...current, track]);
        setMusicTrackId(track.id);
        setMusicProgress(0);
        startMusic(track.id);
      })
      .catch((cause) => setError(describeError(cause)));
  };

  useEffect(() => {
    if (!musicPlaying) return;
    const interval = window.setInterval(() => {
      setMusicProgress((progress) => (progress >= 100 ? 0 : progress + 0.08));
    }, 1000);
    return () => window.clearInterval(interval);
  }, [musicPlaying]);
  useEffect(() => {
    const engine = musicEngine.current;
    if (engine?.audio) {
      engine.audio.volume = Math.max(0, Math.min(100, musicVolume)) / 100;
    } else if (engine?.master && engine.context) {
      engine.master.gain.setTargetAtTime(
        (Math.max(0, Math.min(100, musicVolume)) / 100) * 0.16,
        engine.context.currentTime,
        0.04,
      );
    }
  }, [musicVolume]);
  useEffect(() => () => stopMusicEngine(musicEngine.current), []);
  useTauriEvent<ActivitySnapshot>(api.ACTIVITY_CHANGED, setSnapshot);
  useTauriEvent<ActivityTick>(api.ACTIVITY_TICK, (tick) =>
    setSnapshot((previous) => ({ ...previous, ...tick })),
  );

  const current = snapshot.current;
  const active = current?.kind === "focus" || current?.kind === "meeting";
  const onBreak = current?.kind === "break";
  const currentDuration = elapsed(current, now);
  const currentProgress = active
    ? Math.min(98, Math.max(8, (currentDuration / 3_600_000) * 100))
    : 0;
  const todayStart = dayStart(now);
  const todaySegments = useMemo(
    () => snapshot.segments.filter((segment) => segment.startedAt >= todayStart),
    [snapshot.segments, todayStart],
  );
  const completedFocus = todaySegments.filter(
    (segment) => segment.kind === "focus" || segment.kind === "meeting",
  );
  const todayFocusMs = completedFocus.reduce((total, segment) => total + elapsed(segment, now), 0);
  const todayBreakMs = todaySegments
    .filter((segment) => segment.kind === "break")
    .reduce((total, segment) => total + elapsed(segment, now), 0);
  const selectedSegments = useMemo(
    () =>
      snapshot.segments
        .filter(
          (segment) => segment.startedAt >= selectedDay && segment.startedAt < selectedDay + DAY_MS,
        )
        .filter((segment) => segment.endedAt !== null)
        .slice(-5)
        .reverse(),
    [selectedDay, snapshot.segments],
  );
  const week = useMemo(
    () =>
      Array.from({ length: 7 }, (_, index) => {
        const start = todayStart - (6 - index) * DAY_MS;
        const end = start + DAY_MS;
        const minutes = snapshot.segments
          .filter((segment) => segment.kind === "focus" || segment.kind === "meeting")
          .reduce((total, segment) => total + segmentMinutes(segment, start, end), 0);
        return { label: new Date(start).toLocaleDateString([], { weekday: "short" }), minutes };
      }),
    [snapshot.segments, todayStart],
  );
  const weekTotal = week.reduce((total, day) => total + day.minutes, 0);
  const maxWeekMinutes = Math.max(60, ...week.map((day) => day.minutes));

  const run = async (action: () => Promise<unknown>): Promise<void> => {
    setBusy(true);
    try {
      await action();
      await refresh();
    } catch (cause) {
      setError(describeError(cause));
    } finally {
      setBusy(false);
    }
  };

  const startFocus = (): void => {
    const value = label.trim();
    void run(() => api.startSession("focus", value || undefined));
  };

  const toggleSilent = (): void => {
    const paused = breaks.state.pausedUntil !== null && breaks.state.pausedUntil > now;
    void run(() => api.pauseBreakReminders(paused ? null : now + 60 * 60_000));
  };

  const startMusic = (trackId: string): void => {
    const track = musicTracks.find((candidate) => candidate.id === trackId);
    if (!track) return;
    stopMusicEngine(musicEngine.current);
    const engine = createMusicEngine(track, musicVolume);
    if (!engine) {
      setError("Music is not supported in this app window.");
      setMusicPlaying(false);
      return;
    }
    musicEngine.current = engine;
    setMusicPlaying(true);
  };

  const toggleMusic = (): void => {
    if (musicPlaying) {
      stopMusicEngine(musicEngine.current);
      musicEngine.current = null;
      setMusicPlaying(false);
      return;
    }
    startMusic(musicTrackId);
  };

  const selectedDate = new Date(selectedDay).toLocaleDateString([], {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: "numeric",
  });
  const musicTrack =
    musicTracks.find((track) => track.id === musicTrackId) ?? musicTracks[0] ?? EMPTY_MUSIC_TRACK;
  const musicMinutes = Math.floor((musicProgress * 25) / 100);
  const musicSeconds = Math.floor(((musicProgress * 25) % 100) * 0.6);
  const musicTime = `${musicMinutes}:${String(musicSeconds).padStart(2, "0")}`;

  return (
    <main className="flex min-h-0 flex-1 overflow-hidden bg-canvas text-fg">
      <section className="relative flex min-w-0 flex-1 flex-col items-center justify-center overflow-auto px-5 py-8 lg:px-10">
        <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_42%,var(--glow-1),transparent_38%)]" />
        <div className="relative flex w-full max-w-3xl flex-col items-center">
          <div className="mb-8 flex w-full items-center justify-between lg:absolute lg:top-[-4.5rem]">
            <div className="flex items-center gap-2 text-sm font-semibold text-fg-strong">
              <span className="flex size-7 items-center justify-center rounded-lg bg-accent-soft text-accent">
                <Headphones className="size-4" />
              </span>
              Focus
            </div>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" onClick={toggleSilent} disabled={busy}>
                <BellOff />
                Silent mode
              </Button>
              <Button variant="outline" size="sm">
                <Settings2 />
                Blocker settings
              </Button>
            </div>
          </div>

          <div className="relative flex size-[min(68vw,420px)] min-h-72 min-w-72 items-center justify-center rounded-full p-4 shadow-[0_0_80px_var(--glow-1)]">
            <div
              className="absolute inset-0 rounded-full p-[13px]"
              style={{
                background: `conic-gradient(from -90deg, var(--accent) ${currentProgress}%, var(--line) ${currentProgress}% 100%)`,
              }}
            >
              <div className="flex size-full flex-col items-center justify-center rounded-full bg-canvas">
                <span className="font-mono text-[clamp(3.5rem,8vw,5.25rem)] font-medium tracking-[-0.06em] text-fg-strong tabular-nums">
                  {formatDuration(currentDuration)}
                </span>
                <span className="mt-1 text-base text-fg-muted">
                  {active
                    ? t("focus.timeElapsed")
                    : onBreak
                      ? t("focus.breakInProgress")
                      : t("focus.readyToFocus")}
                </span>
                <Pencil className="mt-4 size-4 text-fg-faint" />
              </div>
            </div>
          </div>

          <div className="mt-7 flex items-center gap-3">
            <Button variant="outline" size="icon-lg" aria-label={t("focus.blockDistractions")}>
              <ShieldCheck />
            </Button>
            <Button
              variant="outline"
              size="icon-lg"
              aria-label={t("focus.startBreak")}
              disabled={busy || Boolean(current)}
              onClick={() => void run(() => api.startSession("break", "Break"))}
            >
              <CirclePause />
            </Button>
            <Button
              size="icon-lg"
              className="size-[72px] rounded-full bg-accent text-accent-fg shadow-[0_0_30px_var(--accent-soft)] hover:bg-accent/90"
              aria-label={current ? t("focus.stopSession") : t("focus.startFocus")}
              onClick={() =>
                void run(
                  current
                    ? api.stopSession
                    : () => api.startSession("focus", label.trim() || undefined),
                )
              }
              disabled={busy}
            >
              {current ? <Square className="fill-current" /> : <Play className="fill-current" />}
            </Button>
            <Button variant="outline" size="icon-lg" aria-label={t("focus.addSession")}>
              <Plus />
            </Button>
            <Button
              variant={musicOpen ? "secondary" : "outline"}
              size="icon-lg"
              aria-label={t("focus.music")}
              aria-expanded={musicOpen}
              onClick={() => setMusicOpen((open) => !open)}
            >
              <Music2 />
            </Button>
          </div>

          <Card className="mt-6 w-full max-w-md overflow-hidden border-line bg-surface shadow-lg">
            <div className="flex items-center gap-3 px-3 py-3">
              <div className="relative flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-accent-soft text-accent">
                <div className="absolute inset-0 bg-[radial-gradient(circle_at_30%_30%,var(--accent),transparent_50%)] opacity-50" />
                <Music2 className="relative size-5" />
              </div>
              <button
                className="min-w-0 flex-1 text-left"
                onClick={() => setMusicOpen((open) => !open)}
                aria-expanded={musicOpen}
              >
                <p className="text-sm font-medium text-fg">{musicTrack.title}</p>
                <p className="truncate text-xs text-fg-muted">{musicTrack.description}</p>
              </button>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={musicPlaying ? t("focus.pauseMusic") : t("focus.playMusic")}
                onClick={toggleMusic}
              >
                {musicPlaying ? (
                  <Pause className="fill-current" />
                ) : (
                  <Play className="fill-current" />
                )}
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={t("focus.togglePlaylist")}
                onClick={() => setMusicOpen((open) => !open)}
              >
                <ChevronDown className={`transition-transform ${musicOpen ? "rotate-180" : ""}`} />
              </Button>
            </div>
            <div className="px-3 pb-3">
              <div className="relative h-1 overflow-hidden rounded-full bg-surface-strong">
                <div
                  className="absolute inset-y-0 left-0 rounded-full bg-accent transition-[width]"
                  style={{ width: `${musicProgress}%` }}
                />
              </div>
              <div className="mt-1 flex justify-between text-[10px] tabular-nums text-fg-faint">
                <span>{musicTime}</span>
                <span>25:00</span>
              </div>
            </div>
            {musicOpen && (
              <div className="border-t border-line bg-surface-strong/40 px-3 py-3">
                <div className="mb-2 flex items-center justify-between text-xs font-medium text-fg-muted">
                  <span className="flex items-center gap-1.5">
                    <ListMusic className="size-3.5" /> Focus mix
                  </span>
                  <span className="flex items-center gap-2">
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      onClick={importMusic}
                      aria-label="Import local audio"
                    >
                      <Plus />
                    </Button>
                    <span>{musicTracks.length} tracks</span>
                  </span>
                </div>
                <div className="space-y-1">
                  {musicTracks.map((track, index) => {
                    const selected = track.id === musicTrackId;
                    return (
                      <button
                        key={track.id}
                        className={`flex w-full items-center gap-3 rounded-lg px-2 py-2 text-left transition-colors ${selected ? "bg-accent-soft text-fg-strong" : "text-fg-muted hover:bg-surface hover:text-fg"}`}
                        onClick={() => {
                          setMusicTrackId(track.id);
                          setMusicProgress(0);
                          startMusic(track.id);
                        }}
                      >
                        <span
                          className={`flex size-6 items-center justify-center rounded-md text-[10px] font-semibold ${selected ? "bg-accent text-accent-fg" : "bg-surface text-fg-faint"}`}
                        >
                          {selected && musicPlaying ? (
                            <Volume2 className="size-3" />
                          ) : (
                            String(index + 1).padStart(2, "0")
                          )}
                        </span>
                        <span className="min-w-0 flex-1">
                          <span className="block truncate text-xs font-medium">{track.title}</span>
                          <span className="block truncate text-[10px] text-fg-faint">
                            {track.description}
                          </span>
                        </span>
                        {selected && <span className="text-[10px] text-accent">active</span>}
                      </button>
                    );
                  })}
                </div>
                <div className="mt-3 flex items-center gap-2 border-t border-line pt-3">
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    aria-label={t("focus.previousTrack")}
                    onClick={() => {
                      const index = musicTracks.findIndex((track) => track.id === musicTrackId);
                      const track =
                        musicTracks[(index - 1 + musicTracks.length) % musicTracks.length];
                      if (!track) return;
                      setMusicTrackId(track.id);
                      setMusicProgress(0);
                      startMusic(track.id);
                    }}
                  >
                    <SkipBack />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    aria-label={t("focus.nextTrack")}
                    onClick={() => {
                      const index = musicTracks.findIndex((track) => track.id === musicTrackId);
                      const track = musicTracks[(index + 1) % musicTracks.length];
                      if (!track) return;
                      setMusicTrackId(track.id);
                      setMusicProgress(0);
                      startMusic(track.id);
                    }}
                  >
                    <SkipForward />
                  </Button>
                  <Volume2 className="ml-auto size-3.5 text-fg-muted" />
                  <input
                    className="h-1 w-20 accent-[var(--accent)]"
                    type="range"
                    min="0"
                    max="100"
                    value={musicVolume}
                    onChange={(event) => setMusicVolume(Number(event.target.value))}
                    aria-label={t("focus.musicVolume")}
                  />
                  <span className="w-7 text-right text-[10px] tabular-nums text-fg-faint">
                    {musicVolume}
                  </span>
                </div>
              </div>
            )}
          </Card>

          {error && (
            <p className="mt-4 rounded-lg bg-danger-soft px-3 py-2 text-sm text-danger">{error}</p>
          )}
          {!current && (
            <div className="mt-4 flex w-full max-w-md gap-2">
              <Input
                value={label}
                onChange={(event) => setLabel(event.target.value)}
                placeholder={t("focus.whatWorking")}
                aria-label={t("focus.topic")}
              />
              <Button onClick={startFocus} disabled={busy}>
                Start
              </Button>
            </div>
          )}
        </div>
      </section>

      <aside className="hidden w-[375px] shrink-0 overflow-y-auto border-l border-line bg-panel xl:block">
        <div className="flex border-b border-line px-4 pt-4 text-sm">
          <button className="border-b-2 border-accent px-1 pb-3 font-medium text-fg-strong">
            Overview
          </button>
          <button className="px-4 pb-3 text-fg-muted">{t("focus.timeline")}</button>
          <button className="px-1 pb-3 text-fg-muted">{t("focus.currentSession")}</button>
          <span className="ml-auto px-1 pb-3 text-fg-muted">»</span>
        </div>
        <div className="border-b border-line px-4 py-4">
          <div className="flex items-center">
            <Button
              variant="outline"
              size="icon-sm"
              onClick={() => setSelectedDay((day) => day - DAY_MS)}
              aria-label={t("focus.previousDay")}
            >
              <ChevronLeft />
            </Button>
            <Button
              variant="outline"
              size="icon-sm"
              className="rounded-l-none border-l-0"
              onClick={() => setSelectedDay((day) => Math.min(todayStart, day + DAY_MS))}
              aria-label={t("focus.nextDay")}
            >
              <ChevronRight />
            </Button>
            <div className="ml-2 flex items-center gap-2 rounded-lg border border-line bg-surface px-3 py-1.5 text-xs text-fg-strong">
              <CalendarDays className="size-3.5 text-fg-muted" />
              {selectedDate}
            </div>
          </div>
        </div>
        <div className="border-b border-line px-4 py-5">
          <p className="text-sm font-semibold text-fg-muted">Today's focus</p>
          <p className="mt-2 text-3xl font-semibold tracking-tight text-fg-strong">
            {formatHours(Math.round(todayFocusMs / 60_000))}
          </p>
          <p className="mt-1 text-xs text-fg-muted">
            {completedFocus.length} focus sessions · {Math.round(todayBreakMs / 60_000)} min of
            breaks
          </p>
        </div>
        <div className="border-b border-line px-4 py-5">
          <div className="flex items-center justify-between">
            <p className="text-sm font-semibold text-fg-muted">{t("focus.thisWeek")}</p>
            <span className="text-xs text-fg-muted">{formatHours(Math.round(weekTotal))}</span>
          </div>
          <div className="mt-5 flex h-32 items-end justify-between gap-2 border-b border-line-soft bg-[linear-gradient(to_bottom,transparent_24%,var(--line-soft)_25%,transparent_26%,transparent_49%,var(--line-soft)_50%,transparent_51%,transparent_74%,var(--line-soft)_75%,transparent_76%)] px-2">
            {week.map((day) => (
              <div
                key={day.label}
                className="flex h-full flex-1 flex-col items-center justify-end gap-2"
              >
                <div
                  className="w-full max-w-5 rounded-t bg-accent"
                  style={{
                    height: `${Math.max(day.minutes > 0 ? 5 : 0, (day.minutes / maxWeekMinutes) * 100)}%`,
                  }}
                />
                <span className="text-[11px] text-fg-muted">{day.label}</span>
              </div>
            ))}
          </div>
          <div className="mt-4 flex items-center justify-between text-xs text-fg-muted">
            <span>{t("focus.avgSession")}</span>
            <span className="font-medium text-fg-strong">
              {completedFocus.length
                ? formatHours(Math.round(todayFocusMs / 60_000 / completedFocus.length))
                : "0 min"}
            </span>
          </div>
        </div>
        <div className="border-b border-line px-4 py-5">
          <p className="text-sm font-semibold text-fg-muted">{t("focus.topDistractions")}</p>
          <p className="mt-3 text-xs text-fg-muted">No distractions.</p>
        </div>
        <div className="px-4 py-5">
          <p className="text-sm font-semibold text-fg-muted">{t("focus.sessions")}</p>
          <div className="mt-3 divide-y divide-line">
            {selectedSegments.length === 0 ? (
              <p className="py-2 text-xs text-fg-muted">No sessions on this day.</p>
            ) : (
              selectedSegments.map((segment) => (
                <div
                  key={segment.id}
                  className="flex items-center justify-between gap-3 py-2 text-xs"
                >
                  <span className="truncate text-fg">{segment.label ?? segment.title}</span>
                  <span className="shrink-0 text-fg-muted">
                    {timeLabel(segment.startedAt)} · {formatDuration(elapsed(segment, now))}
                  </span>
                </div>
              ))
            )}
          </div>
        </div>
      </aside>
    </main>
  );
}
