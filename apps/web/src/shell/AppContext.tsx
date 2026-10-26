"use client";

import { type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { getVersion } from "@tauri-apps/api/app";
import { AnimatePresence, MotionConfig, motion } from "motion/react";

import { Footer } from "@/components/layout/Footer";
import { Sidebar } from "@/components/layout/Sidebar";
import { TopBar } from "@/components/layout/TopBar";
import { MorningOffer } from "@/features/coach/components/MorningOffer";
import { useBreaks } from "@/hooks/useBreaks";
import { useMediaQuery } from "@/hooks/useMediaQuery";
import { usePluginExtensions } from "@/hooks/usePluginExtensions";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { useUpdates } from "@/hooks/useUpdates";
import * as api from "@/lib/api";
import { currentCalendarDay, localDateString } from "@/lib/dates";
import { isTyping } from "@/lib/entries";
import { motionSpring, motionTransition, pageVariants } from "@/lib/motion";
import { hasPrimaryModifier } from "@/lib/platform";
import { isPluginRoute } from "@/lib/plugins";
import { isTauri } from "@/lib/tauri";
import type { ActivitySegment, ActivitySnapshot, ActivityTick, Route } from "@/lib/types";
import { resolveLanguage, type ResolvedLanguage } from "@/lib/i18n";
import { useSettings } from "@/hooks/useSettings";
import { routeToUrl, urlToRoute } from "@/shell/route-url";

const SIDEBAR_COLLAPSED_KEY = "aevio.sidebarCollapsed";
const AUTO_COLLAPSE_QUERY = "(max-width: 959px)";

function readSidebarChoice(): boolean | undefined {
  try {
    const raw = window.localStorage.getItem(SIDEBAR_COLLAPSED_KEY);
    return raw === null ? undefined : raw === "1";
  } catch {
    return undefined;
  }
}

export interface AppShellContextValue {
  lang: ResolvedLanguage;
  currentRoute: Route;
  navigate: (next: Route) => void;
  replace: (next: Route) => void;
  back: () => void;
  forward: () => void;
  canGoBack: boolean;
  canGoForward: boolean;
  revealTimer: { id: string } | undefined;
  setRevealTimer: (v: { id: string } | undefined) => void;
  calendarMount: number;
  setCalendarMount: (fn: (c: number) => number) => void;
  revealUpdates: number;
  setRevealUpdates: (fn: (c: number) => number) => void;
  undoError: string | null;
  setUndoError: (v: string | null) => void;
  undoNotice: { id: number; text: string } | null;
  clearUndoNotice: () => void;
  trackingActive: boolean;
  captureEnabled: boolean;
  currentApp: string | undefined;
  currentKind: ActivitySegment["kind"] | undefined;
  pendingCount: number;
  breaks: ReturnType<typeof useBreaks>;
  updates: ReturnType<typeof useUpdates>;
  pluginNavigation: ReturnType<typeof usePluginExtensions>["navigation"];
  pluginsReady: boolean;
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  appVersion: string | undefined;
  revealTimerState: { id: string } | undefined;
}

import { createContext, useContext } from "react";
const Ctx = createContext<AppShellContextValue | null>(null);

export function useAppShellContext(): AppShellContextValue {
  const v = useContext(Ctx);
  if (!v) throw new Error("useAppShellContext outside provider");
  return v;
}

export function AppShellProvider({ children }: { children: ReactNode }) {
  const router = useRouter();
  const pathname = usePathname() ?? "/";
  const searchParams = useSearchParams();
  const settings = useSettings();
  const resolvedSettingsLang = settings.settings.language;
  const lang: ResolvedLanguage =
    (pathname.split("/").filter(Boolean)[0] as ResolvedLanguage) === "de" ||
    (pathname.split("/").filter(Boolean)[0] as ResolvedLanguage) === "en"
      ? (pathname.split("/").filter(Boolean)[0] as ResolvedLanguage)
      : resolveLanguage(
          resolvedSettingsLang,
          typeof navigator === "undefined" ? "en-US" : navigator.language,
        );

  // sync lang from settings if mismatched
  useEffect(() => {
    const urlLang = pathname.split("/").filter(Boolean)[0];
    const preferred = resolveLanguage(
      resolvedSettingsLang,
      typeof navigator === "undefined" ? "en-US" : navigator.language,
    );
    if ((urlLang === "de" || urlLang === "en") && urlLang !== preferred) {
      // only redirect if settings explicitly chosen (not system defaulting?) - redirect to preferred
      // Avoid loops: only when settings language is explicit de/en
      if (resolvedSettingsLang === "de" || resolvedSettingsLang === "en") {
        const rest = pathname.split("/").filter(Boolean).slice(1).join("/");
        const qs = searchParams?.toString();
        router.replace(`/${preferred}/${rest}/${qs ? `?${qs}` : ""}`);
      }
    }
  }, [resolvedSettingsLang, pathname, searchParams, router]);

  const [appVersion, setAppVersion] = useState<string>();
  useEffect(() => {
    if (!isTauri()) {
      setAppVersion(undefined);
      return;
    }
    void getVersion()
      .then(setAppVersion)
      .catch(() => setAppVersion(undefined));
  }, []);

  const [revealTimer, setRevealTimer] = useState<{ id: string }>();
  const [undoError, setUndoError] = useState<string | null>(null);
  const [undoNotice, setUndoNotice] = useState<{ id: number; text: string } | null>(null);
  const clearUndoNotice = useCallback(() => setUndoNotice(null), []);
  const undoBusy = useRef(false);

  const [captureEnabled, setCaptureEnabledState] = useState(true);
  const [trackingActive, setTrackingActive] = useState(true);
  const [currentApp, setCurrentApp] = useState<string | undefined>(undefined);
  const [currentKind, setCurrentKind] = useState<ActivitySegment["kind"]>();
  const [pendingCount, setPendingCount] = useState(0);
  const [calendarMount, setCalendarMountState] = useState(0);
  const setCalendarMount = useCallback(
    (fn: (c: number) => number) => setCalendarMountState(fn),
    [],
  );
  const [revealUpdates, setRevealUpdatesState] = useState(0);
  const setRevealUpdates = useCallback(
    (fn: (c: number) => number) => setRevealUpdatesState(fn),
    [],
  );
  const [sidebarChoice, setSidebarChoice] = useState(readSidebarChoice);
  const narrowWindow = useMediaQuery(AUTO_COLLAPSE_QUERY);
  const sidebarCollapsed = sidebarChoice ?? narrowWindow;
  const updates = useUpdates();
  const breaks = useBreaks();
  const { navigation: pluginNavigation, ready: pluginsReady } = usePluginExtensions();

  const toggleSidebar = useCallback((): void => {
    const next = !sidebarCollapsed;
    setSidebarChoice(next);
    try {
      window.localStorage.setItem(SIDEBAR_COLLAPSED_KEY, next ? "1" : "0");
    } catch {}
  }, [sidebarCollapsed]);

  // Nav stack for back/forward availability
  const [navEntries, setNavEntries] = useState<string[]>(() => [pathname]);
  const [cursor, setCursor] = useState(0);
  // sync cursor when pathname changes externally
  useEffect(() => {
    setNavEntries((prev) => {
      // if pathname already at cursor, noop
      if (prev[cursor] === pathname) return prev;
      // new navigation: truncate and push
      const next = [...prev.slice(0, cursor + 1), pathname];
      setCursor(next.length - 1);
      return next;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pathname]);

  const currentRoute = urlToRoute(pathname, searchParams ?? new URLSearchParams());

  // lastCalendar handling: if current is calendar, store; on navigate to bare calendar use stored.
  const lastCalendar = useRef<Extract<Route, { name: "calendar" }>>({ name: "calendar" });
  if (currentRoute.name === "calendar") {
    lastCalendar.current = {
      name: "calendar",
      scale: currentRoute.scale ?? "day",
      date: currentRoute.date ?? localDateString(currentCalendarDay(new Date())),
    };
  }
  // patch navigate to apply lastCalendar merge
  const navigateWithMerge = useCallback(
    (next: Route): void => {
      const destination =
        next.name === "calendar" && Object.keys(next).length === 1 ? lastCalendar.current : next;
      const url = routeToUrl(destination, lang);
      router.push(url);
    },
    [lang, router],
  );

  const replace = useCallback(
    (next: Route): void => {
      const url = routeToUrl(next, lang);
      router.replace(url);
    },
    [lang, router],
  );

  const back = useCallback(() => router.back(), [router]);
  const forward = useCallback(() => router.forward(), [router]);

  // plugins redirect
  useEffect(() => {
    if (!pluginsReady || !isPluginRoute(currentRoute)) return;
    if (pluginNavigation.some((entry) => entry.route.name === currentRoute.name)) return;
    navigateWithMerge({ name: "settings", section: "pluginStore" });
  }, [currentRoute, navigateWithMerge, pluginNavigation, pluginsReady]);

  // keyboard
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (!hasPrimaryModifier(event) || event.altKey) return;
      if (event.key === ",") {
        event.preventDefault();
        navigateWithMerge({ name: "settings" });
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [navigateWithMerge]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (
        !hasPrimaryModifier(event) ||
        event.altKey ||
        event.key.toLowerCase() !== "z" ||
        isTyping(event.target)
      )
        return;
      event.preventDefault();
      if (undoBusy.current) return;
      undoBusy.current = true;
      const redo = event.shiftKey;
      void api
        .undo(redo)
        .then((label) => {
          setUndoError(null);
          setUndoNotice({
            id: Date.now(),
            text: label
              ? `${redo ? "Redid" : "Undid"} ${label.toLowerCase()}`
              : `Nothing to ${redo ? "redo" : "undo"}`,
          });
        })
        .catch((cause: unknown) => setUndoError(api.describeError(cause)))
        .finally(() => {
          undoBusy.current = false;
        });
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useTauriEvent<string>("open-timer", (id) => {
    setRevealTimer({ id });
    navigateWithMerge({ name: "timers" });
  });

  useEffect(() => {
    if (!isTauri()) return;
    api
      .fetchActivitySnapshot(0)
      .then((snapshot) => {
        setCaptureEnabledState(snapshot.captureEnabled);
        if (snapshot.trackingActive !== undefined) setTrackingActive(snapshot.trackingActive);
        if (snapshot.current) setCurrentApp(snapshot.current.app);
        else setCurrentApp(undefined);
        setCurrentKind(snapshot.current?.kind);
      })
      .catch((err) => console.error("Failed to load activity snapshot", err));
  }, []);

  const adoptCapture = (payload: ActivityTick | ActivitySnapshot): void => {
    setCaptureEnabledState(payload.captureEnabled);
    if (payload.trackingActive !== undefined) setTrackingActive(payload.trackingActive);
    if (payload.current) setCurrentApp(payload.current.app);
    else setCurrentApp(undefined);
    setCurrentKind(payload.current?.kind);
  };
  useTauriEvent<ActivityTick>(api.ACTIVITY_TICK, adoptCapture);
  useTauriEvent<ActivitySnapshot>(api.ACTIVITY_CHANGED, adoptCapture);

  const updatePending = useCallback(async (): Promise<void> => {
    if (!isTauri()) return;
    try {
      const startOfDay = new Date();
      startOfDay.setHours(0, 0, 0, 0);
      const entries = await api.listTimeEntries(startOfDay.getTime(), Date.now());
      const count = entries.filter((e) => e.status === "pending").length;
      setPendingCount(count);
    } catch (err) {
      console.error("Failed to load pending entries count", err);
    }
  }, []);
  useEffect(() => {
    void updatePending();
  }, [updatePending]);
  useTauriEvent(api.ENTRIES_CHANGED, updatePending);
  useTauriEvent(api.SUGGESTION_READY, updatePending);

  useTauriEvent(api.OPEN_REVIEW, () => {
    navigateWithMerge({
      name: "calendar",
      scale: "day",
      date: localDateString(currentCalendarDay(new Date())),
      review: true,
    });
    setCalendarMount((c) => c + 1);
  });

  useTauriEvent(api.OPEN_BREAK_SETTINGS, () => {
    navigateWithMerge({ name: "settings", section: "notifications" });
  });

  const handleToggleCapture = async () => {
    if (!isTauri()) return;
    try {
      const next = !trackingActive;
      await api.setCaptureEnabled(next);
      setTrackingActive(next);
      if (next) setCaptureEnabledState(true);
    } catch (err) {
      console.error("Failed to toggle capture", err);
    }
  };

  const value: AppShellContextValue = {
    lang,
    currentRoute,
    navigate: navigateWithMerge,
    replace,
    back,
    forward,
    canGoBack: cursor > 0,
    canGoForward: cursor < navEntries.length - 1,
    revealTimer,
    setRevealTimer,
    calendarMount,
    setCalendarMount,
    revealUpdates,
    setRevealUpdates,
    undoError,
    setUndoError,
    undoNotice,
    clearUndoNotice,
    trackingActive,
    captureEnabled,
    currentApp,
    currentKind,
    pendingCount,
    breaks,
    updates,
    pluginNavigation,
    pluginsReady,
    sidebarCollapsed,
    toggleSidebar,
    appVersion,
    revealTimerState: revealTimer,
  };

  return (
    <Ctx.Provider value={value}>
      <MotionConfig reducedMotion="user" transition={motionTransition}>
        <div className="flex h-full min-h-0 flex-col overflow-hidden">
          <TopBar
            canGoBack={cursor > 0}
            canGoForward={cursor < navEntries.length - 1}
            onBack={back}
            onForward={forward}
            sidebarCollapsed={sidebarCollapsed}
            onToggleSidebar={toggleSidebar}
            currentRoute={currentRoute}
            onNavigate={navigateWithMerge}
            updateVersion={updates?.available?.version}
            pendingCount={pendingCount}
            breakReminder={
              breaks.state.reminder
                ? { kind: "break", label: breaks.state.reminder.label }
                : breaks.state.stopwatch
                  ? { kind: "stopwatch", label: breaks.state.stopwatch.label }
                  : undefined
            }
            onOpenUpdate={() => {
              navigateWithMerge({ name: "settings", section: "updates" });
              setRevealUpdates((c) => c + 1);
            }}
            onOpenReview={() => navigateWithMerge({ name: "calendar", review: true })}
            onOpenReminder={(kind) =>
              navigateWithMerge({ name: kind === "break" ? "focus" : "timers" })
            }
            onOpenNotificationSettings={() =>
              navigateWithMerge({ name: "settings", section: "notifications" })
            }
          />
          {currentRoute.name !== "coach" && (
            <MorningOffer onOpen={() => navigateWithMerge({ name: "coach" })} />
          )}
          {undoError && (
            <div role="alert" className="bg-danger-soft px-4 py-2 text-[12px] text-danger">
              {undoError}
            </div>
          )}
          <AnimatePresence initial={false}>
            {undoNotice && (
              <UndoNotice key={undoNotice.id} text={undoNotice.text} onDone={clearUndoNotice} />
            )}
          </AnimatePresence>
          <div
            className={`grid min-h-0 flex-1 overflow-hidden transition-[grid-template-columns] duration-150 ${
              sidebarCollapsed
                ? "grid-cols-[56px_minmax(0,1fr)]"
                : "grid-cols-[224px_minmax(0,1fr)]"
            }`}
          >
            <Sidebar
              currentRoute={currentRoute}
              onNavigate={navigateWithMerge}
              pendingCount={pendingCount}
              currentApp={currentApp}
              collapsed={sidebarCollapsed}
              captureEnabled={captureEnabled}
              trackingActive={trackingActive}
              onToggleCapture={handleToggleCapture}
              onBreak={
                breaks.state.current && breaks.state.current.endedAt === null
                  ? {
                      label: breaks.state.current.label,
                      remainingMs:
                        breaks.state.current.startedAt +
                        breaks.state.current.plannedMs -
                        breaks.now,
                    }
                  : undefined
              }
              updateVersion={updates?.available?.version}
              onOpenUpdate={() => {
                navigateWithMerge({ name: "settings", section: "updates" });
                setRevealUpdates((c) => c + 1);
              }}
              breakReminder={
                breaks.state.reminder
                  ? { kind: "break", label: breaks.state.reminder.label }
                  : breaks.state.stopwatch
                    ? { kind: "stopwatch", label: breaks.state.stopwatch.label }
                    : undefined
              }
              onOpenReview={() => navigateWithMerge({ name: "calendar", review: true })}
              onOpenReminder={(kind) =>
                navigateWithMerge({ name: kind === "break" ? "focus" : "timers" })
              }
              onOpenNotificationSettings={() =>
                navigateWithMerge({ name: "settings", section: "notifications" })
              }
            />
            <div className="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
              <AnimatePresence mode="wait" initial={false}>
                <motion.div
                  key={currentRoute.name}
                  variants={pageVariants}
                  initial="initial"
                  animate="animate"
                  exit="exit"
                  transition={motionTransition}
                  className="flex h-full min-h-0 min-w-0 flex-col overflow-hidden"
                >
                  {children}
                </motion.div>
              </AnimatePresence>
            </div>
          </div>
          <Footer
            trackingActive={trackingActive}
            captureEnabled={captureEnabled}
            currentApp={currentApp}
            currentKind={currentKind}
            onToggleCapture={handleToggleCapture}
            onStartBreak={() => void api.startBreak()}
            onStartSession={(kind) =>
              void api.startSession(kind, kind[0].toUpperCase() + kind.slice(1))
            }
            pendingCount={pendingCount}
            version={appVersion}
          />
        </div>
      </MotionConfig>
    </Ctx.Provider>
  );
}

function UndoNotice({ text, onDone }: { text: string; onDone: () => void }) {
  useEffect(() => {
    const timer = window.setTimeout(onDone, 1800);
    return () => window.clearTimeout(timer);
  }, [onDone]);
  return (
    <motion.output
      initial={{ opacity: 0, y: 10, scale: 0.96 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: 6, scale: 0.98 }}
      transition={motionSpring}
      className="pointer-events-none fixed bottom-5 left-1/2 z-50 -translate-x-1/2 rounded-full border border-line-strong bg-surface-strong px-3.5 py-1.5 font-medium text-[12px] text-fg shadow-lg"
    >
      {text}
    </motion.output>
  );
}
