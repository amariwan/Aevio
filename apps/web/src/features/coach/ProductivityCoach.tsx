import { useState } from "react";
import { CalendarDays, CheckCircle2, Clock3, Settings2, Target } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useI18n } from "@/hooks/useI18n";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { formatDuration } from "@/lib/format";
import { useCoach } from "./useCoach";
import { localDay } from "./date";
import { CoachFocusPanel } from "./components/Focus";
import { WeeklyCoach } from "./components/Weekly";
import { BrainDump } from "./components/BrainDump";
import { MyDay } from "./components/MyDay";
import { PlanSuggestion } from "./components/PlanSuggestion";
import { Planning } from "./components/Planning";
import { DailyReview } from "./components/Review";
import { CoachSettings } from "./components/Settings";

export function ProductivityCoach() {
  const { t } = useI18n();
  const [day, setDay] = useState(localDay);
  const [tab, setTab] = useState("today");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [zone] = useState(() => Intl.DateTimeFormat().resolvedOptions().timeZone);
  const { snapshot, error, busy, apply, refresh } = useCoach(day, zone);
  return (
    <main
      className="min-h-0 flex-1 overflow-auto bg-canvas p-5 text-fg"
      aria-label="Productivity Coach"
      aria-busy={busy}
    >
      <div className="mx-auto flex max-w-6xl flex-col gap-6">
        <header className="rounded-2xl border border-line p-5 sm:p-6">
          <div className="flex flex-col gap-5 lg:flex-row lg:items-end lg:justify-between">
            <div className="max-w-2xl">
              <div className="flex items-center gap-2 text-xs font-semibold tracking-[0.18em] text-accent uppercase">
                <span className="size-1.5 rounded-full bg-accent" aria-hidden="true" />
                Productivity Coach
              </div>
              <h1 className="mt-3 text-3xl font-semibold tracking-tight text-fg-strong sm:text-4xl">
                Ein ruhiger Blick auf deinen Tag
              </h1>
              <p className="mt-2 max-w-xl text-sm leading-6 text-fg-muted">
                Planen, anfangen und zurückblicken. Eine klare nächste Entscheidung reicht.
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <div className="flex items-center gap-2 rounded-xl border border-line bg-canvas p-1.5">
                <CalendarDays className="ml-2 size-4 text-fg-muted" aria-hidden="true" />
                <label htmlFor="coach-day" className="sr-only">
                  Tag auswählen
                </label>
                <Input
                  id="coach-day"
                  aria-label="Coach-Datum"
                  className="h-9 w-[9.5rem] border-0 bg-transparent px-2 shadow-none focus-visible:ring-0"
                  type="date"
                  required
                  disabled={busy}
                  value={day}
                  onChange={(event) => {
                    if (event.target.value) setDay(event.target.value);
                  }}
                />
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={busy}
                  onClick={() => setDay(localDay())}
                >
                  Heute
                </Button>
              </div>
              <Button
                type="button"
                size="icon-sm"
                variant="ghost"
                onClick={() => setSettingsOpen(true)}
                aria-label="Coach-Einstellungen öffnen"
                title="Coach-Einstellungen"
              >
                <Settings2 aria-hidden="true" />
              </Button>
            </div>
          </div>
        </header>
        {error && (
          <div
            role="alert"
            className="flex items-center justify-between gap-3 rounded-lg border border-line p-3 text-sm text-danger"
          >
            <p>{error}</p>
            <Button variant="outline" disabled={busy} onClick={() => void refresh()}>
              Neu laden
            </Button>
          </div>
        )}
        {!snapshot ? (
          <div role="status" className="flex flex-col gap-3">
            <p className="text-sm text-fg-muted">Tagesplan laden …</p>
            <Skeleton className="h-36" />
          </div>
        ) : (
          <>
            <div className="grid grid-cols-1 divide-y divide-line border-y border-line sm:grid-cols-3 sm:divide-x sm:divide-y-0">
              <div className="flex items-center gap-3 p-4">
                <span className="rounded-lg bg-accent-soft p-2 text-accent">
                  <Target className="size-4" aria-hidden="true" />
                </span>
                <div>
                  <p className="text-xs text-fg-muted">Tagesziel</p>
                  <p className="mt-0.5 truncate text-sm font-medium text-fg-strong">
                    {snapshot.plan?.goal || "Noch offen"}
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-3 p-4">
                <span className="rounded-lg bg-accent-soft p-2 text-accent">
                  <CheckCircle2 className="size-4" aria-hidden="true" />
                </span>
                <div>
                  <p className="text-xs text-fg-muted">Erledigt</p>
                  <p className="mt-0.5 text-sm font-medium text-fg-strong">
                    {snapshot.tasks.filter((task) => task.done).length} von {snapshot.tasks.length}
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-3 p-4">
                <span className="rounded-lg bg-accent-soft p-2 text-accent">
                  <Clock3 className="size-4" aria-hidden="true" />
                </span>
                <div>
                  <p className="text-xs text-fg-muted">Erfasst</p>
                  <p className="mt-0.5 text-sm font-medium text-fg-strong">
                    {formatDuration(snapshot.trackedMs)}
                  </p>
                </div>
              </div>
            </div>
            {!snapshot.preferences.enabled && (
              <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-line p-4">
                <p className="text-sm text-fg-muted">
                  Der proaktive Coach ist ausgeschaltet. Deine manuelle Planung funktioniert
                  weiterhin lokal.
                </p>
                <Button variant="outline" onClick={() => setSettingsOpen(true)}>
                  Coach einrichten
                </Button>
              </div>
            )}
            {snapshot.preferences.enabled &&
              snapshot.preferences.morningPlanning &&
              !snapshot.plan &&
              day === localDay() && (
                <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-line p-4">
                  <p className="text-sm">Möchtest du deinem Tag ein Hauptziel geben?</p>
                  <Button onClick={() => setTab("planning")}>{t("coach.planDay")}</Button>
                  <Button
                    variant="ghost"
                    disabled={busy}
                    onClick={() =>
                      void apply({
                        kind: "save_plan",
                        plan: {
                          day,
                          timeZone: zone,
                          goal: "",
                          notes: "",
                          availableMinutes: 0,
                          energy: null,
                          state: "skipped",
                          revision: 0,
                        },
                      })
                    }
                  >
                    Heute überspringen
                  </Button>
                </div>
              )}
            <Tabs value={tab} onValueChange={(value) => setTab(String(value))}>
              <TabsList
                variant="line"
                className="w-full max-w-full justify-start overflow-x-auto rounded-none border-b border-line p-0"
                aria-label="Coach-Bereiche"
              >
                <TabsTrigger value="today">{t("coach.myDay")}</TabsTrigger>
                <TabsTrigger value="planning">{t("coach.dayPlanning")}</TabsTrigger>
                <TabsTrigger value="focus">{t("coach.focus")}</TabsTrigger>
                <TabsTrigger value="review">{t("coach.dayClose")}</TabsTrigger>
                <TabsTrigger value="weekly">Woche & Memory</TabsTrigger>
                <TabsTrigger value="brain">{t("coach.brainDump")}</TabsTrigger>
              </TabsList>
              <TabsContent className="mt-5" value="today">
                <MyDay key={day} day={snapshot} apply={apply} busy={busy} error={error} />
              </TabsContent>
              <TabsContent className="mt-5" value="planning">
                <div className="flex flex-col gap-5">
                  <Planning key={day} day={snapshot} apply={apply} busy={busy} />
                  <PlanSuggestion
                    key={`${day}:${snapshot.plan?.revision}`}
                    day={snapshot}
                    apply={apply}
                    busy={busy}
                  />
                </div>
              </TabsContent>
              <TabsContent className="mt-5" value="focus">
                <CoachFocusPanel key={day} day={snapshot} />
              </TabsContent>
              <TabsContent className="mt-5" value="review">
                <DailyReview key={day} day={snapshot} apply={apply} busy={busy} />
              </TabsContent>
              <TabsContent className="mt-5" value="weekly">
                <WeeklyCoach key={day} day={snapshot} apply={apply} busy={busy} />
              </TabsContent>
              <TabsContent className="mt-5" value="brain">
                <BrainDump day={snapshot} />
              </TabsContent>
            </Tabs>
            <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
              <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-2xl">
                <DialogHeader>
                  <DialogTitle>Coach-Einstellungen</DialogTitle>
                  <DialogDescription>
                    Entscheide in Ruhe, wie viel Unterstützung du möchtest. Alles bleibt lokal.
                  </DialogDescription>
                </DialogHeader>
                <CoachSettings
                  key={JSON.stringify(snapshot.preferences)}
                  preferences={snapshot.preferences}
                  apply={apply}
                  busy={busy}
                  onSaved={() => setSettingsOpen(false)}
                />
              </DialogContent>
            </Dialog>
          </>
        )}
      </div>
    </main>
  );
}
