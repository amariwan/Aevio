import { useEffect, useState } from "react";
import { localDay } from "../date";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { describeError } from "@/lib/api";
import type { Apply, PersonalIntelligence, Preferences, Style } from "../types";
import * as api from "../api";
function time(value: number) {
  return `${String(Math.floor(value / 60)).padStart(2, "0")}:${String(value % 60).padStart(2, "0")}`;
}
function minute(value: string) {
  const [hours, minutes] = value.split(":").map(Number);
  return hours * 60 + minutes;
}
export function CoachSettings({
  preferences,
  apply,
  busy,
  onSaved,
}: {
  preferences: Preferences;
  apply: Apply;
  busy: boolean;
  onSaved?: () => void;
}) {
  const { t } = useI18n();
  const [draft, setDraft] = useState(preferences);
  const [error, setError] = useState<string | null>(null);
  const [erase, setErase] = useState(false);
  const [saved, setSaved] = useState(false);
  const [personal, setPersonal] = useState<PersonalIntelligence | null>(null);
  const day = localDay();
  const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  useEffect(() => {
    if (preferences.personalIntelligenceEnabled) {
      void api
        .fetchPersonalIntelligence(day, timeZone)
        .then(setPersonal)
        .catch(() => setPersonal(null));
    } else {
      setPersonal(null);
    }
  }, [preferences.personalIntelligenceEnabled, day, timeZone]);
  async function save() {
    setError(null);
    setSaved(false);
    try {
      if (
        draft.reminders &&
        !(await isPermissionGranted()) &&
        (await requestPermission()) !== "granted"
      ) {
        setError(
          "Benachrichtigungen sind im Betriebssystem nicht erlaubt. Du kannst die übrigen Einstellungen speichern, indem du Erinnerungen deaktivierst.",
        );
        return;
      }
      if (await apply({ kind: "save_preferences", preferences: draft })) {
        setSaved(true);
        onSaved?.();
      }
    } catch (cause) {
      setError(describeError(cause));
    }
  }
  return (
    <div className="flex flex-col gap-4">
      <form
        className="flex flex-col gap-5"
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <FieldGroup>
          <div className="flex flex-wrap gap-2">
            {(
              [
                ["enabled", "Coach aktiv"],
                ["morningPlanning", "Planung morgens anbieten"],
                ["reminders", "Native Erinnerungen erlauben"],
                ["aiEnabled", "On-Device-KI für Aufgaben erlauben"],
                ["visualTimer", "Visueller Fokus-Timer"],
                ["automaticHints", "Appwechsel-Hinweise erlauben"],
                ["widgetContext", "Fokuskontext im Widget zeigen"],
                ["widgetVisible", "Floating Widget anzeigen"],
                ["personalIntelligenceEnabled", "Persönliche Intelligenz (opt-in)"],
              ] as const
            ).map(([key, label]) => (
              <Button
                key={key}
                type="button"
                variant="outline"
                aria-pressed={draft[key]}
                onClick={() => setDraft({ ...draft, [key]: !draft[key] })}
              >
                {draft[key] ? "✓ " : ""}
                {label}
              </Button>
            ))}
          </div>
          <p className="text-xs text-fg-muted">
            Die KI erhält nur den von dir ausgewählten Aufgabentitel, deine bisherigen Schritte und
            den gewählten Unterstützungsbedarf. Auf nicht unterstützten Plattformen erhältst du
            ausdrücklich gekennzeichnete lokale Vorschläge.
          </p>
          <Field>
            <FieldLabel htmlFor="coach-style">Coaching-Stil</FieldLabel>
            <NativeSelect
              id="coach-style"
              value={draft.style}
              onChange={(event) => setDraft({ ...draft, style: event.target.value as Style })}
            >
              {["minimal", "calm", "supportive", "structured", "direct"].map((style) => (
                <NativeSelectOption key={style} value={style}>
                  {style}
                </NativeSelectOption>
              ))}
            </NativeSelect>
          </Field>
          <div className="grid gap-3 sm:grid-cols-2">
            <Field>
              <FieldLabel htmlFor="coach-quiet-start">{t("coachSettings.quietStart")}</FieldLabel>
              <Input
                id="coach-quiet-start"
                type="time"
                required
                value={time(draft.quietStart)}
                onChange={(event) => setDraft({ ...draft, quietStart: minute(event.target.value) })}
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="coach-quiet-end">{t("coachSettings.quietEnd")}</FieldLabel>
              <Input
                id="coach-quiet-end"
                type="time"
                required
                value={time(draft.quietEnd)}
                onChange={(event) => setDraft({ ...draft, quietEnd: minute(event.target.value) })}
              />
            </Field>
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <Field>
              <FieldLabel htmlFor="coach-cooldown">
                Mindestens Minuten zwischen Hinweisen
              </FieldLabel>
              <Input
                id="coach-cooldown"
                type="number"
                min={15}
                max={1440}
                required
                value={draft.cooldownMinutes}
                onChange={(event) =>
                  setDraft({ ...draft, cooldownMinutes: Number(event.target.value) })
                }
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="coach-review-time">{t("coachSettings.reviewTime")}</FieldLabel>
              <Input
                id="coach-review-time"
                type="time"
                required
                value={time(draft.reviewMinute)}
                onChange={(event) =>
                  setDraft({ ...draft, reviewMinute: minute(event.target.value) })
                }
              />
            </Field>
          </div>
          <Field>
            <FieldLabel htmlFor="coach-buffer">Reservierter Planungspuffer (Minuten)</FieldLabel>
            <Input
              id="coach-buffer"
              type="number"
              min={0}
              max={1440}
              required
              value={draft.planningBufferMinutes}
              onChange={(event) =>
                setDraft({ ...draft, planningBufferMinutes: Number(event.target.value) })
              }
            />
            <p className="text-xs text-fg-muted">
              Wird von der verfügbaren Zeit abgezogen, bevor Aufgaben vorgeschlagen werden.
            </p>
          </Field>
        </FieldGroup>
        {preferences.personalIntelligenceEnabled && personal && (
          <div className="flex flex-col gap-2">
            <p className="text-sm font-medium text-fg">{t("coachSettings.learnedObservations")}</p>
            {personal.observations.map((obs) => (
              <div
                key={obs.id}
                className="flex items-center justify-between gap-2 rounded border border-line p-2"
              >
                <p className="text-xs text-fg-muted">
                  {obs.explanation} · {obs.samples} Belege
                </p>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() =>
                    void api.suppressObservation(day, obs.id).then(() =>
                      setPersonal((prev) =>
                        prev
                          ? {
                              ...prev,
                              observations: prev.observations.filter((o) => o.id !== obs.id),
                            }
                          : prev,
                      ),
                    )
                  }
                >
                  Nicht mehr lernen
                </Button>
              </div>
            ))}
            <Button
              type="button"
              variant="destructive"
              size="sm"
              onClick={() => void api.resetPersonalIntelligence().then(() => setPersonal(null))}
            >
              Personal Intelligence zurücksetzen
            </Button>
          </div>
        )}
        {error && (
          <p role="alert" className="text-sm text-danger">
            {error}
          </p>
        )}
        <div className="flex flex-wrap items-center gap-3">
          <Button type="submit" disabled={busy}>
            Coach-Einstellungen speichern
          </Button>
          {saved && (
            <p role="status" className="text-sm text-fg-muted">
              Gespeichert
            </p>
          )}
          <Button
            type="button"
            variant="destructive"
            disabled={busy}
            onClick={() => setErase(true)}
          >
            Coach-Daten löschen …
          </Button>
        </div>
      </form>
      <Dialog open={erase} onOpenChange={setErase}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Coach-Daten endgültig löschen?</DialogTitle>
            <DialogDescription>
              Tagespläne, persönliche Coach-Aufgaben, Schritte, Reviews, Memory, Brain Dump,
              Routinen und Coach-Einstellungen werden gelöscht. Projektaufgaben und Zeitbuchungen
              bleiben erhalten.
            </DialogDescription>
          </DialogHeader>
          <div className="flex gap-3">
            <Button
              variant="destructive"
              disabled={busy}
              onClick={() =>
                void apply({ kind: "erase_coach" }).then((ok) => {
                  if (ok) setErase(false);
                })
              }
            >
              Coach-Daten endgültig löschen
            </Button>
            <Button variant="outline" onClick={() => setErase(false)}>
              Abbrechen
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
