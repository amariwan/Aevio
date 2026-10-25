import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Textarea } from "@/components/ui/textarea";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { COACH_DAY_CHANGED, describeError } from "@/lib/api";
import { formatDuration } from "@/lib/format";
import type { Apply, Day } from "../types";
import * as api from "../api";
interface WeekDay {
  day: string;
  completed: number;
  plannedMinutes: number;
  actualMs: number;
  outcome: string;
  focusSessions: number;
  postponed: number;
}
interface Memory {
  id: string;
  content: string;
  kind: "explicit" | "confirmed";
  updatedAt: number;
}
export interface Insights {
  weekly: { startsOn: string; days: WeekDay[]; previous: WeekDay[]; recommendations: string[] };
  memory: Memory[];
  estimates: { source: string; taskId: string; minutes: number; samples: number }[];
  planning: {
    selected: string[];
    deferred: string[];
    minutes: number;
    capacity: number;
    reason: string;
  };
}
export function WeeklyCoach({ day, apply, busy }: { day: Day; apply: Apply; busy: boolean }) {
  const [insights, setInsights] = useState<Insights | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [editing, setEditing] = useState<Memory | null>(null);
  const [working, setWorking] = useState(false);
  const load = useCallback(async () => {
    try {
      setInsights(await api.fetchInsights<Insights>(day.day, day.timeZone));
      setError(null);
    } catch (cause) {
      setError(describeError(cause));
    }
  }, [day.day, day.timeZone]);
  useEffect(() => {
    void load();
  }, [load, day.preferences.memoryEnabled]);
  useTauriEvent(COACH_DAY_CHANGED, () => void load());
  async function memory(
    command: { kind: "save"; memory: Memory } | { kind: "remove"; id: string } | { kind: "clear" },
  ) {
    setWorking(true);
    setError(null);
    try {
      await api.updateMemory(command);
      setContent("");
      setEditing(null);
      await load();
    } catch (cause) {
      setError(describeError(cause));
    } finally {
      setWorking(false);
    }
  }
  if (!insights)
    return (
      <Card className="rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
        <CardContent>
          <p role={error ? "alert" : "status"}>{error ?? "Wochenrückblick laden …"}</p>
          <Button variant="outline" onClick={() => void load()}>
            Neu laden
          </Button>
        </CardContent>
      </Card>
    );
  const current = insights.weekly.days.reduce((sum, row) => sum + row.actualMs, 0);
  const previous = insights.weekly.previous.reduce((sum, row) => sum + row.actualMs, 0);
  return (
    <div className="flex flex-col gap-5">
      {error && (
        <p role="alert" className="text-sm text-danger">
          {error}
        </p>
      )}
      <section>
        <div className="mb-4">
          <h2 className="text-base font-medium text-fg-strong">
            Woche ab {insights.weekly.startsOn}
          </h2>
          <p className="mt-1 text-sm text-fg-muted">
            Lokale Aggregate, keine Bewertung deiner Person. {formatDuration(current)} erfasst,
            Vorwoche {formatDuration(previous)}.
          </p>
        </div>
        <div className="mt-5 flex flex-col gap-2">
          {insights.weekly.days.map((row) => (
            <div
              key={row.day}
              className="flex flex-wrap gap-x-4 gap-y-1 border-b border-line py-2 text-sm"
            >
              <span className="font-medium text-fg">{row.day}</span>
              <span className="text-fg-muted">
                {row.completed} erledigt · {row.plannedMinutes} Min. geplant ·{" "}
                {formatDuration(row.actualMs)} erfasst · {row.focusSessions} Fokus-Sessions
              </span>
              <span className="text-fg-muted">
                Ziel:{" "}
                {
                  (
                    {
                      achieved: "erreicht",
                      partially_achieved: "teilweise erreicht",
                      not_achieved: "nicht erreicht",
                      changed: "geändert",
                      not_reviewed: "nicht bewertet",
                    } as Record<string, string>
                  )[row.outcome]
                }
              </span>
            </div>
          ))}
          {insights.weekly.recommendations.map((text) => (
            <p key={text} className="text-sm text-fg-muted">
              {text}
            </p>
          ))}
        </div>
      </section>
      <Card className="rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
        <CardHeader>
          <CardTitle>Memory & Schätzungen</CardTitle>
          <CardDescription>
            Explizite Präferenzen stehen in Einstellungen. Diese Notizen bestätigst du selbst.
            Berechnete Muster stehen getrennt darunter.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <Button
            variant="outline"
            disabled={busy}
            aria-pressed={day.preferences.memoryEnabled}
            onClick={() =>
              void apply({
                kind: "save_preferences",
                preferences: { ...day.preferences, memoryEnabled: !day.preferences.memoryEnabled },
              })
            }
          >
            {day.preferences.memoryEnabled
              ? "✓ Memory und Schätzmuster aktiv"
              : "Memory und Schätzmuster aktivieren"}
          </Button>
          <p className="text-xs text-fg-muted">
            Bei aktivem Memory können bis zu acht deiner bestätigten Notizen in einen ausdrücklich
            angeforderten On-Device-KI-Vorschlag einfließen. Sitzungskontext bleibt an deiner
            aktuellen Fokus-Session.
          </p>
          <form
            className="flex flex-col gap-3"
            onSubmit={(event) => {
              event.preventDefault();
              void memory({
                kind: "save",
                memory: {
                  id: editing?.id ?? "",
                  content,
                  kind: editing?.kind ?? "explicit",
                  updatedAt: 0,
                },
              });
            }}
          >
            <FieldGroup>
              <Field>
                <FieldLabel htmlFor="coach-memory">
                  {editing
                    ? "Notiz bearbeiten"
                    : "Was soll der Coach mit deiner Zustimmung berücksichtigen?"}
                </FieldLabel>
                <Textarea
                  id="coach-memory"
                  required
                  maxLength={500}
                  value={content}
                  onChange={(event) => setContent(event.target.value)}
                />
              </Field>
            </FieldGroup>
            <div className="flex gap-2">
              <Button type="submit" disabled={working || !content.trim()}>
                Memory-Notiz speichern
              </Button>
              {editing && (
                <Button
                  variant="ghost"
                  onClick={() => {
                    setEditing(null);
                    setContent("");
                  }}
                >
                  Abbrechen
                </Button>
              )}
            </div>
          </form>
          {insights.memory.length === 0 && (
            <p className="text-sm text-fg-muted">Keine gespeicherten Memory-Notizen.</p>
          )}
          {insights.memory.map((note) => (
            <div
              key={note.id}
              className="flex flex-wrap items-center gap-2 rounded-lg border border-line p-3"
            >
              <p className="flex-1 text-sm text-fg">{note.content}</p>
              <span className="text-xs text-fg-muted">
                {note.kind === "explicit" ? "Explizit" : "Bestätigt"}
              </span>
              <Button
                variant="ghost"
                disabled={working}
                onClick={() => {
                  setEditing(note);
                  setContent(note.content);
                }}
              >
                Bearbeiten
              </Button>
              <Button
                variant="ghost"
                disabled={working}
                onClick={() => void memory({ kind: "remove", id: note.id })}
              >
                Löschen
              </Button>
            </div>
          ))}
          <p className="text-sm text-fg-muted">
            Berechnete Dauerhinweise verwenden nur dieselbe Aufgabenidentität mit mindestens zwei
            zugeordneten Zeitbeispielen.
          </p>
          {insights.estimates.map((estimate) => {
            const task = day.catalog.find(
              (task) => task.task.source === estimate.source && task.task.id === estimate.taskId,
            );
            return task ? (
              <div
                key={`${estimate.source}:${estimate.taskId}`}
                className="flex flex-wrap items-center gap-3 text-sm"
              >
                <p className="flex-1">
                  {task.title}: etwa {estimate.minutes} Min. aus {estimate.samples} Beispielen.
                </p>
                <Button
                  variant="outline"
                  disabled={working}
                  onClick={() =>
                    void memory({
                      kind: "save",
                      memory: {
                        id: "",
                        content: `Für „${task.title}“ plane ich ungefähr ${estimate.minutes} Minuten ein.`,
                        kind: "confirmed",
                        updatedAt: 0,
                      },
                    })
                  }
                >
                  Als Erkenntnis bestätigen
                </Button>
              </div>
            ) : null;
          })}
          {insights.estimates.length === 0 && (
            <p className="text-xs text-fg-muted">
              Noch keine freigegebenen Muster mit ausreichend Beispielen.
            </p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
