import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Textarea } from "@/components/ui/textarea";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { formatDuration } from "@/lib/format";
import type { Apply, Day, Outcome } from "../types";
const outcomes: Record<Outcome, string> = {
  achieved: "Erreicht",
  partially_achieved: "Teilweise erreicht",
  not_achieved: "Nicht erreicht",
  changed: "Geändert / nicht mehr relevant",
  not_reviewed: "Noch nicht bewertet",
};
export function DailyReview({ day, apply, busy }: { day: Day; apply: Apply; busy: boolean }) {
  const { t } = useI18n();
  const [review, setReview] = useState(day.review);
  const [saved, setSaved] = useState(false);
  return (
    <Card className="rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
      <CardHeader>
        <CardTitle>{t("review.title")}</CardTitle>
        <CardDescription>
          Eine Reflexion ohne Bewertung von außen. Du kannst sie später über das Datum wieder öffnen
          und bearbeiten.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="mb-5 flex flex-col gap-2 text-sm text-fg-muted">
          <p>Ziel: {day.plan?.goal || "Kein Tagesziel festgelegt"}</p>
          <p>
            {day.tasks.filter((task) => task.done).length} von {day.tasks.length} Aufgaben
            abgeschlossen · {day.plannedMinutes} Min. geplant · {formatDuration(day.trackedMs)}{" "}
            erfasst · {day.changes} Planänderungen
          </p>
          <p>Arbeitszeit zeigt Aufwand. Ob dein Ziel erreicht ist, entscheidest du selbst.</p>
        </div>
        <form
          className="flex flex-col gap-5"
          onSubmit={(event) => {
            event.preventDefault();
            setSaved(false);
            void apply({ kind: "save_review", review, revision: review.revision }).then((ok) => {
              if (ok) {
                setSaved(true);
                setReview(ok.review);
              }
            });
          }}
        >
          {day.review.revision !== review.revision && (
            <p role="status" className="text-sm text-fg-muted">
              Eine neuere Reflexion ist verfügbar.{" "}
              <Button type="button" variant="outline" onClick={() => setReview(day.review)}>
                Aktuelle Reflexion laden und Entwurf ersetzen
              </Button>
            </p>
          )}
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="coach-outcome">Hast du dein Hauptziel erreicht?</FieldLabel>
              <NativeSelect
                id="coach-outcome"
                value={review.outcome}
                onChange={(event) =>
                  setReview({ ...review, outcome: event.target.value as Outcome })
                }
              >
                {Object.entries(outcomes).map(([value, title]) => (
                  <NativeSelectOption key={value} value={value}>
                    {title}
                  </NativeSelectOption>
                ))}
              </NativeSelect>
            </Field>
            {(
              [
                ["worked", "Was hat heute gut funktioniert?"],
                ["obstacles", "Was hat dich aufgehalten?"],
                ["tomorrow", "Was möchtest du morgen anders machen?"],
              ] as const
            ).map(([key, label]) => (
              <Field key={key}>
                <FieldLabel htmlFor={`coach-review-${key}`}>{label}</FieldLabel>
                <Textarea
                  id={`coach-review-${key}`}
                  value={review[key]}
                  maxLength={10000}
                  onChange={(event) => setReview({ ...review, [key]: event.target.value })}
                />
              </Field>
            ))}
          </FieldGroup>
          <div className="flex items-center gap-3">
            <Button type="submit" disabled={busy}>
              Reflexion speichern
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
