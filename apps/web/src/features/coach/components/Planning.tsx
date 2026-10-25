import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import type { Apply, Day, Plan } from "../types";

export function Planning({ day, apply, busy }: { day: Day; apply: Apply; busy: boolean }) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<Plan>(
    () =>
      day.plan ?? {
        day: day.day,
        timeZone: day.timeZone,
        goal: "",
        notes: "",
        availableMinutes: 0,
        energy: null,
        state: "planned",
        revision: 0,
      },
  );
  const [saved, setSaved] = useState(false);
  async function save(state: Plan["state"]) {
    setSaved(false);
    const next = await apply({ kind: "save_plan", plan: { ...draft, state } });
    if (next?.plan) {
      setSaved(true);
      setDraft(next.plan);
    }
  }
  return (
    <Card className="rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
      <CardHeader>
        <CardTitle>{t("planning.title")}</CardTitle>
        <CardDescription>
          Ein Hauptziel und bis zu drei wichtige Aufgaben können Orientierung geben. Du
          entscheidest, wie viel du planst.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form
          className="flex flex-col gap-5"
          onSubmit={(event) => {
            event.preventDefault();
            void save("planned");
          }}
        >
          {day.plan && day.plan.revision !== draft.revision && (
            <p role="status" className="text-sm text-fg-muted">
              Der gespeicherte Plan wurde in einer anderen Ansicht geändert.{" "}
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  if (day.plan) setDraft(day.plan);
                }}
              >
                Aktuellen Plan laden und Entwurf ersetzen
              </Button>
            </p>
          )}
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="coach-goal">Was möchtest du heute erreichen?</FieldLabel>
              <Input
                id="coach-goal"
                value={draft.goal}
                maxLength={2000}
                onChange={(event) => setDraft({ ...draft, goal: event.target.value })}
              />
            </Field>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field>
                <FieldLabel htmlFor="coach-capacity">Verfügbare Arbeitszeit in Minuten</FieldLabel>
                <Input
                  id="coach-capacity"
                  type="number"
                  min={0}
                  max={1440}
                  required
                  value={draft.availableMinutes}
                  onChange={(event) =>
                    setDraft({ ...draft, availableMinutes: Number(event.target.value) })
                  }
                />
                <p className="text-xs text-fg-muted">
                  Nach Terminen und Pausen. Es gibt noch keinen Kalenderabgleich.
                </p>
              </Field>
              <Field>
                <FieldLabel htmlFor="coach-energy">Energie, freiwillig</FieldLabel>
                <NativeSelect
                  id="coach-energy"
                  value={draft.energy ?? ""}
                  onChange={(event) =>
                    setDraft({
                      ...draft,
                      energy: event.target.value ? Number(event.target.value) : null,
                    })
                  }
                >
                  <NativeSelectOption value="">{t("planning.none")}</NativeSelectOption>
                  {[1, 2, 3, 4, 5].map((value) => (
                    <NativeSelectOption key={value} value={value}>
                      {value}
                    </NativeSelectOption>
                  ))}
                </NativeSelect>
              </Field>
            </div>
            <Field>
              <FieldLabel htmlFor="coach-plan-notes">Persönliche Notizen</FieldLabel>
              <Textarea
                id="coach-plan-notes"
                value={draft.notes}
                maxLength={10000}
                onChange={(event) => setDraft({ ...draft, notes: event.target.value })}
              />
            </Field>
          </FieldGroup>
          <p className="text-xs text-fg-muted">
            Tagesgrenzen: {draft.timeZone}. Aufgaben und Reihenfolge bearbeitest du unter „Mein
            Tag“.
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <Button type="submit" disabled={busy}>
              Plan speichern
            </Button>
            <Button
              type="button"
              variant="ghost"
              disabled={busy}
              onClick={() => void save("skipped")}
            >
              Planung überspringen
            </Button>
            {saved && (
              <p role="status" className="text-sm text-fg-muted">
                Gespeichert
              </p>
            )}
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
