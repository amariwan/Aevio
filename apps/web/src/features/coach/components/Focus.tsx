import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Textarea } from "@/components/ui/textarea";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { formatDuration } from "@/lib/format";
import { localDay } from "../date";
import { useCoachFocus } from "../useCoachFocus";
import type { Day } from "../types";
export function CoachFocusPanel({ day }: { day: Day }) {
  const { t } = useI18n();
  const { focus, error, busy, now, action } = useCoachFocus();
  const [id, setId] = useState(day.tasks.find((task) => !task.done)?.id ?? "");
  const [minutes, setMinutes] = useState(25);
  const [body, setBody] = useState(false);
  const [next, setNext] = useState("");
  const task = day.tasks.find((task) => task.id === id);
  const elapsed = focus
    ? focus.state === "running"
      ? Math.max(0, now - focus.startedAt)
      : focus.elapsedMs
    : 0;
  return (
    <Card className="rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
      <CardHeader>
        <CardTitle>Fokus & Rückkehr</CardTitle>
        <CardDescription>
          Eine Aufgabe, ein nächster Schritt. Die Session nutzt Aevios bestehende
          Fokus-Zeiterfassung.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-5">
        {error && (
          <p role="alert" className="text-sm text-danger">
            {error}
          </p>
        )}
        {focus && (
          <div className="flex flex-col gap-3 rounded-lg border border-line p-4">
            <p className="font-medium text-fg">
              {focus.state === "interrupted" ? "Dein Rückkehrkontext" : "Gerade im Fokus"}:{" "}
              {focus.title}
            </p>
            <p className="text-sm text-fg-muted">
              Ziel: {focus.goal || "Kein Tagesziel festgelegt"}
            </p>
            <p className="text-sm text-fg">
              {focus.nextStep || "Wähle einen kleinen nächsten Schritt."}
            </p>
            {day.preferences.visualTimer && (
              <p className="font-mono text-2xl tabular-nums text-accent">
                {formatDuration(elapsed)} / {focus.plannedMinutes} Min.
              </p>
            )}
            {focus.bodyDoubling && (
              <p className="text-sm text-fg-muted">
                Ruhiges Co-Working. Nur ein Check-in am gewählten Ende, falls native Erinnerungen
                erlaubt sind.
              </p>
            )}
            <p className="text-xs text-fg-muted">
              Du kannst jederzeit aufhören. Eine Session schließt keine Aufgabe automatisch ab.
            </p>
            <div className="flex flex-wrap gap-2">
              {focus.state === "running" ? (
                <Button
                  disabled={busy}
                  onClick={() => void action("coach_end_focus", { finish: false })}
                >
                  Fokus pausieren
                </Button>
              ) : (
                <Button
                  disabled={busy || day.day !== localDay()}
                  onClick={() =>
                    void action("coach_start_focus", {
                      day: day.day,
                      timeZone: day.timeZone,
                      id: focus.taskId,
                      minutes: focus.plannedMinutes,
                      nextStep: focus.nextStep,
                      bodyDoubling: focus.bodyDoubling,
                    })
                  }
                >
                  Mit Kontext fortsetzen
                </Button>
              )}
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => void action("coach_end_focus", { finish: true })}
              >
                Fokus abschließen
              </Button>
            </div>
          </div>
        )}
        {(!focus || focus.state !== "running") && (
          <form
            className="flex flex-col gap-4"
            onSubmit={(event) => {
              event.preventDefault();
              void action("coach_start_focus", {
                day: day.day,
                timeZone: day.timeZone,
                id,
                minutes,
                nextStep: next || task?.steps.find((step) => !step.done)?.title || "",
                bodyDoubling: body,
              });
            }}
          >
            <FieldGroup>
              <Field>
                <FieldLabel htmlFor="coach-focus-task">{t("focus.task")}</FieldLabel>
                <NativeSelect
                  id="coach-focus-task"
                  value={id}
                  onChange={(event) => setId(event.target.value)}
                >
                  <NativeSelectOption value="">Aufgabe auswählen</NativeSelectOption>
                  {day.tasks
                    .filter((task) => !task.done)
                    .map((task) => (
                      <NativeSelectOption key={task.id} value={task.id}>
                        {task.title}
                      </NativeSelectOption>
                    ))}
                </NativeSelect>
              </Field>
              <Field>
                <FieldLabel htmlFor="coach-focus-step">Dein nächster Schritt</FieldLabel>
                <Textarea
                  id="coach-focus-step"
                  maxLength={1000}
                  value={next}
                  placeholder={
                    task?.steps.find((step) => !step.done)?.title ??
                    "Was möchtest du als Erstes tun?"
                  }
                  onChange={(event) => setNext(event.target.value)}
                />
              </Field>
              <Field>
                <FieldLabel htmlFor="coach-focus-minutes">
                  Minuten · auch 2 oder 5 sind möglich
                </FieldLabel>
                <Input
                  id="coach-focus-minutes"
                  type="number"
                  min={2}
                  max={180}
                  required
                  value={minutes}
                  onChange={(event) => setMinutes(Number(event.target.value))}
                />
              </Field>
              <Button
                type="button"
                variant="outline"
                aria-pressed={body}
                onClick={() => setBody(!body)}
              >
                {body ? "✓ " : ""}Ruhiges Body Doubling
              </Button>
            </FieldGroup>
            <Button type="submit" disabled={busy || !task || day.day !== localDay()}>
              Fokus starten
            </Button>
            {day.day !== localDay() && (
              <p className="text-sm text-fg-muted">Fokus-Sessions starten im heutigen Tagesplan.</p>
            )}
          </form>
        )}
      </CardContent>
    </Card>
  );
}
