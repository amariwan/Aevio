import { type ReactElement, useCallback, useEffect, useRef, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { Footer } from "@/components/layout/Footer";
import { Sidebar } from "@/components/layout/Sidebar";
import { TopBar } from "@/components/layout/TopBar";
import { Apps } from "@/features/apps/Apps";
import { Agent } from "@/features/agent";
import { Calendar } from "@/features/calendar/Calendar";
import { Dashboard } from "@/features/dashboard";

import { Documents } from "@/features/documents";
import { Focus } from "@/features/focus";
import { Finance } from "@/features/finance/Finance";

import { Clients } from "@/features/clients/Clients";
import { TimeEntries } from "@/features/entries/TimeEntries";
import { Invoices } from "@/features/invoices/Invoices";
import { Initiatives } from "@/features/initiatives";

import { Issues } from "@/features/issues";
import { MorningOffer } from "@/features/coach/components/MorningOffer";
import { ProductivityCoach } from "@/features/coach";
import { Projects } from "@/features/projects/Projects";
import { Settings } from "@/features/settings/Settings";
import { Staffing } from "@/features/staffing/Staffing";
import { Timers } from "@/features/timers/Timers";
import { MyTimesheet } from "@/features/timesheets/MyTimesheet";
import { Notes } from "@/features/notes";
import { Timesheets } from "@/features/timesheets/Timesheets";
import { Tasks } from "@/features/tasks";
import { Teams } from "@/features/teams";
import { useBreaks } from "@/hooks/useBreaks";
import { useMediaQuery } from "@/hooks/useMediaQuery";
import { SettingsProvider } from "@/hooks/useSettings";
import { I18nProvider } from "@/hooks/useI18n";
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

const SIDEBAR_COLLAPSED_KEY = "aevio.sidebarCollapsed";

/** Below this window width the sidebar starts collapsed to leave the page room. */
const AUTO_COLLAPSE_QUERY = "(max-width: 959px)";

/**
 * Sidebar width is a per-viewer convenience, so it lives in localStorage.
 * `undefined` means the user never chose, so the window width decides.
 */
function readSidebarChoice(): boolean | undefined {
  try {
    const raw = window.localStorage.getItem(SIDEBAR_COLLAPSED_KEY);
    return raw === null ? undefined : raw === "1";
  } catch {
    return undefined;
  }
}

/** Browser-style history: a stack plus where the user is standing in it. */
interface NavState {
  entries: Route[];
  cursor: number;
}

export default function App() {
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
  const [nav, setNav] = useState<NavState>({
    entries: [{ name: "dashboard" }],
    cursor: 0,
  });
  const currentRoute = nav.entries[nav.cursor] || { name: "calendar" };
  const lastCalendar = useRef<Extract<Route, { name: "calendar" }>>({
    name: "calendar",
  });
  if (currentRoute.name === "calendar") {
    lastCalendar.current = {
      name: "calendar",
      scale: currentRoute.scale ?? "day",
      date: currentRoute.date ?? localDateString(currentCalendarDay(new Date())),
    };
  }
  const [revealTimer, setRevealTimer] = useState<{ id: string }>();
  const [undoError, setUndoError] = useState<string | null>(null);
  // A fresh id per ⌘Z, so repeating the same undo restarts the notice.
  const [undoNotice, setUndoNotice] = useState<{
    id: number;
    text: string;
  } | null>(null);
  const clearUndoNotice = useCallback(() => setUndoNotice(null), []);
  const undoBusy = useRef(false);

  const [captureEnabled, setCaptureEnabledState] = useState(true);
  const [trackingActive, setTrackingActive] = useState(true);
  const [currentApp, setCurrentApp] = useState<string | undefined>(undefined);
  const [currentKind, setCurrentKind] = useState<ActivitySegment["kind"]>();
  const [pendingCount, setPendingCount] = useState(0);
  // Bumped to remount the Calendar, so a review request always starts fresh
  // on today, even when that exact route is already showing.
  const [calendarMount, setCalendarMount] = useState(0);
  // Bumped by the sidebar's "Update available", so Settings scrolls to its
  // Updates section even when Settings is already open.
  const [revealUpdates, setRevealUpdates] = useState(0);
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
    } catch {
      // Storage can be unavailable; the choice still holds for this session.
    }
  }, [sidebarCollapsed]);

  const navigate = useCallback((next: Route): void => {
    const destination =
      next.name === "calendar" && Object.keys(next).length === 1 ? lastCalendar.current : next;
    setNav((previous) => {
      const current = previous.entries[previous.cursor];
      if (current && JSON.stringify(current) === JSON.stringify(destination)) {
        return previous;
      }
      const entries = [...previous.entries.slice(0, previous.cursor + 1), destination];
      return { entries, cursor: entries.length - 1 };
    });
  }, []);

  useEffect(() => {
    if (!pluginsReady || !isPluginRoute(currentRoute)) return;
    if (pluginNavigation.some((entry) => entry.route.name === currentRoute.name)) return;
    navigate({ name: "settings", section: "pluginStore" });
  }, [currentRoute, navigate, pluginNavigation, pluginsReady]);

  /** Updates the current route in place: filters and tabs, not new pages. */
  const replace = useCallback((next: Route): void => {
    setNav((previous) => {
      const entries = [...previous.entries];
      entries[previous.cursor] = next;
      return { ...previous, entries };
    });
  }, []);

  const back = useCallback((): void => {
    setNav((previous) => ({
      ...previous,
      cursor: Math.max(0, previous.cursor - 1),
    }));
  }, []);

  const forward = useCallback((): void => {
    setNav((previous) => ({
      ...previous,
      cursor: Math.min(previous.entries.length - 1, previous.cursor + 1),
    }));
  }, []);

  /* ⌘, (Ctrl+, elsewhere) - the macOS Settings convention. */
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (!hasPrimaryModifier(event) || event.altKey) return;
      if (event.key === ",") {
        event.preventDefault();
        navigate({ name: "settings" });
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [navigate]);

  // Leave native text undo alone. Entry history belongs to the app, so it
  // remains available after navigating away from the page that made it.
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
    navigate({ name: "timers" });
  });

  // Initial load & capture events
  useEffect(() => {
    if (!isTauri()) return;
    api
      .fetchActivitySnapshot(0)
      .then((snapshot) => {
        setCaptureEnabledState(snapshot.captureEnabled);
        if (snapshot.trackingActive !== undefined) {
          setTrackingActive(snapshot.trackingActive);
        }
        if (snapshot.current) {
          setCurrentApp(snapshot.current.app);
        } else {
          setCurrentApp(undefined);
        }
        setCurrentKind(snapshot.current?.kind);
      })
      .catch((err) => {
        console.error("Failed to load activity snapshot", err);
      });
  }, []);

  const adoptCapture = (payload: ActivityTick | ActivitySnapshot): void => {
    setCaptureEnabledState(payload.captureEnabled);
    if (payload.trackingActive !== undefined) {
      setTrackingActive(payload.trackingActive);
    }
    if (payload.current) {
      setCurrentApp(payload.current.app);
    } else {
      setCurrentApp(undefined);
    }
    setCurrentKind(payload.current?.kind);
  };
  useTauriEvent<ActivityTick>(api.ACTIVITY_TICK, adoptCapture);
  useTauriEvent<ActivitySnapshot>(api.ACTIVITY_CHANGED, adoptCapture);

  // Entries waiting for review today (the Calendar's pending badge).
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

  // The menu-bar panel's "N to review" button.
  useTauriEvent(api.OPEN_REVIEW, () => {
    navigate({
      name: "calendar",
      scale: "day",
      date: localDateString(currentCalendarDay(new Date())),
      review: true,
    });
    setCalendarMount((count) => count + 1);
  });

  // The break reminder's "Reminder settings…".
  useTauriEvent(api.OPEN_BREAK_SETTINGS, () => {
    navigate({ name: "settings", section: "notifications" });
  });

  const handleToggleCapture = async () => {
    if (!isTauri()) return;
    try {
      const next = !trackingActive;
      await api.setCaptureEnabled(next);
      setTrackingActive(next);
      if (next) {
        setCaptureEnabledState(true);
      }
    } catch (err) {
      console.error("Failed to toggle capture", err);
    }
  };

  const renderView = (): ReactElement => {
    switch (currentRoute.name) {
      case "dashboard":
        return <Dashboard navigate={navigate} />;
      case "initiatives":
        return <Initiatives />;
      case "notes":
        return <Notes />;
      case "documents":
        return <Documents />;
      case "focus":
        return <Focus />;
      case "agent":
        return <Agent navigate={navigate} />;
      case "finance":
        return <Finance />;
      case "staffing":
        return <Staffing />;
      case "coach":
        return <ProductivityCoach />;
      case "tasks":
        return <Tasks />;
      case "issues":
        return <Issues />;
      case "teams":
        return <Teams tab={currentRoute.tab} />;
      case "calendar":
        return <Calendar key={calendarMount} route={currentRoute} navigate={navigate} />;
      case "timesheet":
        return <MyTimesheet route={currentRoute} navigate={navigate} replace={replace} />;
      case "timers":
        return <Timers revealTimer={revealTimer} />;
      case "apps":
        return <Apps />;
      case "entries":
        return <TimeEntries route={currentRoute} navigate={navigate} replace={replace} />;
      case "timesheets":
        return <Timesheets route={currentRoute} navigate={navigate} replace={replace} />;
      case "projects":
        return <Projects route={currentRoute} navigate={navigate} replace={replace} />;
      case "clients":
        return <Clients route={currentRoute} navigate={navigate} replace={replace} />;
      case "invoices":
        return <Invoices route={currentRoute} navigate={navigate} replace={replace} />;
      case "settings":
        return (
          <Settings
            route={currentRoute}
            updates={updates}
            revealUpdates={revealUpdates}
            navigate={navigate}
          />
        );
      default:
        return <Dashboard navigate={navigate} />;
    }
  };

  return (
    <MotionConfig reducedMotion="user" transition={motionTransition}>
      <SettingsProvider>
        <I18nProvider>
          <div className="flex h-full min-h-0 flex-col overflow-hidden">
            <TopBar
              canGoBack={nav.cursor > 0}
              canGoForward={nav.cursor < nav.entries.length - 1}
              onBack={back}
              onForward={forward}
              sidebarCollapsed={sidebarCollapsed}
              onToggleSidebar={toggleSidebar}
              currentRoute={currentRoute}
              onNavigate={navigate}
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
                navigate({ name: "settings", section: "updates" });
                setRevealUpdates((count) => count + 1);
              }}
              onOpenReview={() => navigate({ name: "calendar", review: true })}
              onOpenReminder={(kind) => navigate({ name: kind === "break" ? "focus" : "timers" })}
              onOpenNotificationSettings={() =>
                navigate({ name: "settings", section: "notifications" })
              }
            />
            {currentRoute.name !== "coach" && (
              <MorningOffer onOpen={() => navigate({ name: "coach" })} />
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
                onNavigate={navigate}
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
                  navigate({ name: "settings", section: "updates" });
                  setRevealUpdates((count) => count + 1);
                }}
                breakReminder={
                  breaks.state.reminder
                    ? { kind: "break", label: breaks.state.reminder.label }
                    : breaks.state.stopwatch
                      ? { kind: "stopwatch", label: breaks.state.stopwatch.label }
                      : undefined
                }
                onOpenReview={() => navigate({ name: "calendar", review: true })}
                onOpenReminder={(kind) => navigate({ name: kind === "break" ? "focus" : "timers" })}
                onOpenNotificationSettings={() =>
                  navigate({ name: "settings", section: "notifications" })
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
                    {renderView()}
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
        </I18nProvider>
      </SettingsProvider>
    </MotionConfig>
  );
}

/** "Undid split": a brief pill confirming what ⌘Z did, then gone. */
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
