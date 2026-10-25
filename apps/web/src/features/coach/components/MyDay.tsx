import { useEffect, useState } from "react";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { COACH_DAY_CHANGED } from "@/lib/api";
import { ArrowDown, ArrowUp, Check, Pencil, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import { CardDescription, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { formatDuration } from "@/lib/format";
import type { Apply, Day, Intervention, PersonalIntelligence, PlannedTask } from "../types";
import { nextDay } from "../date";
import * as api from "../api";
import { InterventionCard } from "./InterventionCard";
import { TaskEditor } from "./TaskEditor";
import { Assistance } from "./Assistance";

export function MyDay({
  day,
  apply,
  busy,
  error,
}: {
  day: Day;
  apply: Apply;
  busy: boolean;
  error: string | null;
}) {
  const { t } = useI18n();
  const [editing, setEditing] = useState<PlannedTask | "new" | null>(null);
  const [help, setHelp] = useState<PlannedTask | null>(null);
  const [targetDay, setTargetDay] = useState(nextDay(day.day));
  const [existing, setExisting] = useState("");
  const [intervention, setIntervention] = useState<Intervention | null>(null);
  const [personal, setPersonal] = useState<PersonalIntelligence | null>(null);
  useEffect(() => {
    void api
      .fetchIntervention(day.day, day.timeZone)
      .then(setIntervention)
      .catch(() => setIntervention(null));
    void api
      .fetchPersonalIntelligence(day.day, day.timeZone)
      .then(setPersonal)
      .catch(() => setPersonal(null));
  }, [day.day, day.timeZone]);
  useTauriEvent(COACH_DAY_CHANGED, () => {
    void api
      .fetchIntervention(day.day, day.timeZone)
      .then(setIntervention)
      .catch(() => setIntervention(null));
  });
  function reorder(index: number, offset: number) {
    const ids = day.tasks.map((task) => task.id);
    [ids[index], ids[index + offset]] = [ids[index + offset], ids[index]];
    void apply({ kind: "reorder", ids });
  }
  return (
    <div className="flex flex-col gap-5">
      {intervention && (
        <InterventionCard
          intervention={intervention}
          day={day.day}
          onDismiss={() => setIntervention(null)}
        />
      )}
      {personal?.recommendations?.[0] && (
        <section className="border-b border-line pb-6">
          <div className="mb-3">
            <CardTitle>{personal.recommendations[0].title}</CardTitle>
            <CardDescription>{personal.recommendations[0].message}</CardDescription>
          </div>
          <div className="flex flex-col gap-2">
            <p className="text-xs text-fg-muted">{personal.recommendations[0].evidence}</p>
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                onClick={() =>
                  void api
                    .applyRecommendation(
                      day.day,
                      personal.recommendations[0].id,
                      personal.recommendations[0].kind,
                    )
                    .then(() => setPersonal(null))
                }
              >
                Übernehmen
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() =>
                  void api
                    .suppressRecommendation(day.day, personal.recommendations[0].id)
                    .then(() => setPersonal(null))
                }
              >
                Nicht mehr vorschlagen
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setPersonal(null)}>
                Verwerfen
              </Button>
            </div>
          </div>
        </section>
      )}
      <section className="border-b border-line pb-6">
        <div className="mb-3">
          <CardTitle>
            {day.plan?.goal
              ? `Heute geht es um: ${day.plan.goal}`
              : "Womit möchtest du heute beginnen?"}
          </CardTitle>
          <CardDescription>
            {day.tasks.filter((task) => task.done).length} von {day.tasks.length} Aufgaben erledigt
            · {day.plannedMinutes} Min. geplant · {formatDuration(day.trackedMs)} erfasst
          </CardDescription>
        </div>
        <div className="flex flex-col gap-3">
          <p className="text-sm text-fg-muted">
            {day.plan?.availableMinutes ?? 0} Min. verfügbare Arbeitszeit ·{" "}
            {formatDuration(day.unassignedMs)} noch keiner Tagesaufgabe zugeordnet.
          </p>
          {day.overbookedMinutes > 0 && (
            <p role="status" className="rounded-lg border border-line p-3 text-sm text-fg">
              Dein Plan liegt {day.overbookedMinutes} Minuten über der verfügbaren Zeit. Du kannst
              Aufgaben verschieben oder den Umfang anpassen. Deine Entscheidung bleibt frei.
            </p>
          )}
          {day.tasks.filter((task) => task.important).length > 3 && (
            <p className="text-xs text-fg-muted">
              Mehr als drei wichtige Aufgaben sind möglich. Falls es sich voll anfühlt, kannst du
              eine kleinere Auswahl treffen.
            </p>
          )}
          <div className="flex flex-wrap gap-2">
            <Button onClick={() => setEditing("new")} disabled={busy}>
              Aufgabe hinzufügen
            </Button>
            <NativeSelect
              aria-label="Bestehende Aufgabe auswählen"
              value={existing}
              onChange={(event) => setExisting(event.target.value)}
            >
              <NativeSelectOption value="">Bestehende Aufgabe auswählen</NativeSelectOption>
              {day.catalog
                .filter(
                  (task) =>
                    !task.done &&
                    !day.tasks.some(
                      (planned) =>
                        planned.task.source === task.task.source &&
                        planned.task.id === task.task.id,
                    ),
                )
                .map((task) => (
                  <NativeSelectOption
                    key={`${task.task.source}:${task.task.id}`}
                    value={`${task.task.source}:${task.task.id}`}
                  >
                    {task.title} · {task.task.source === "project" ? "Projekt" : "Persönlich"}
                  </NativeSelectOption>
                ))}
            </NativeSelect>
            <Button
              variant="outline"
              disabled={busy || !existing}
              onClick={() => {
                const task = day.catalog.find(
                  (item) => `${item.task.source}:${item.task.id}` === existing,
                );
                if (task)
                  void apply({
                    kind: "add_task",
                    task: task.task,
                    estimateMinutes: task.estimateMinutes ?? 0,
                    important: false,
                  }).then((ok) => {
                    if (ok) setExisting("");
                  });
              }}
            >
              Einplanen
            </Button>
          </div>
        </div>
      </section>
      <section className="border-b border-line pb-6">
        <div className="mb-3">
          <CardTitle>Was ist dein nächster kleiner Schritt?</CardTitle>
          <CardDescription>
            Reihenfolge, Schätzungen und Abschluss verwaltest du bewusst.
          </CardDescription>
        </div>
        <div className="flex flex-col gap-4">
          {day.tasks.length === 0 && (
            <p className="py-5 text-sm text-fg-muted">
              Dein Tag ist noch offen. Halte eine Aufgabe fest oder wähle eine bestehende aus.
            </p>
          )}
          {day.tasks.map((task, index) => (
            <div key={task.id} className="flex flex-col gap-3 rounded-lg border border-line p-4">
              <div className="flex items-start gap-3">
                <Button
                  variant="outline"
                  size="icon"
                  disabled={busy}
                  aria-label={`${task.done ? "Wieder öffnen" : "Erledigen"}: ${task.title}`}
                  aria-pressed={task.done}
                  onClick={() =>
                    void apply({ kind: "complete_task", id: task.id, done: !task.done })
                  }
                >
                  {task.done && <Check data-icon="inline-start" />}
                </Button>
                <div className="min-w-0 flex-1">
                  <p className={`font-medium text-fg ${task.done ? "line-through" : ""}`}>
                    {task.title}
                  </p>
                  <p className="text-xs text-fg-muted">
                    {task.estimateMinutes} Min. geschätzt · {formatDuration(task.actualMs)}{" "}
                    zugeordnet · P{task.priority}
                    {task.postponements > 0 ? ` · ${task.postponements}× verschoben` : ""}
                  </p>
                  {task.notes && (
                    <p className="mt-1 whitespace-pre-wrap text-sm text-fg-muted">{task.notes}</p>
                  )}
                </div>
                {task.important && <Badge variant="secondary">{t("myDay.important")}</Badge>}
              </div>
              {task.steps.length > 0 && (
                <ol className="flex flex-col gap-2">
                  {task.steps.map((step, position) => (
                    <li key={position} className="flex items-center gap-2">
                      <Button
                        variant="ghost"
                        className="h-auto justify-start whitespace-normal text-left"
                        size="sm"
                        aria-pressed={step.done}
                        disabled={busy}
                        onClick={() =>
                          void apply({
                            kind: "save_steps",
                            id: task.id,
                            steps: task.steps.map((item, i) =>
                              i === position ? { ...item, done: !item.done } : item,
                            ),
                          })
                        }
                      >
                        {step.done ? "✓" : position + 1}. {step.title}
                      </Button>
                    </li>
                  ))}
                </ol>
              )}
              <div className="flex flex-wrap gap-2">
                <Button variant="outline" size="sm" disabled={busy} onClick={() => setHelp(task)}>
                  <Sparkles data-icon="inline-start" />
                  Just Start / Schritte
                </Button>
                <Button variant="ghost" size="sm" disabled={busy} onClick={() => setEditing(task)}>
                  <Pencil data-icon="inline-start" />
                  Bearbeiten
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={`Nach oben: ${task.title}`}
                  disabled={busy || index === 0}
                  onClick={() => reorder(index, -1)}
                >
                  <ArrowUp />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={`Nach unten: ${task.title}`}
                  disabled={busy || index === day.tasks.length - 1}
                  onClick={() => reorder(index, 1)}
                >
                  <ArrowDown />
                </Button>
                {!task.done && (
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={busy || !targetDay}
                    onClick={() => void apply({ kind: "move_task", id: task.id, targetDay })}
                  >
                    Verschieben
                  </Button>
                )}
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={busy}
                  onClick={() => void apply({ kind: "remove_task", id: task.id })}
                >
                  Aus Tagesplan nehmen
                </Button>
              </div>
            </div>
          ))}
          <div className="flex flex-wrap items-center gap-3">
            <label htmlFor="coach-move-date" className="text-sm text-fg-muted">
              Verschieben nach
            </label>
            <Input
              id="coach-move-date"
              type="date"
              className="w-auto"
              value={targetDay}
              onChange={(event) => setTargetDay(event.target.value)}
            />
          </div>
        </div>
      </section>
      {day.carryover.length > 0 && (
        <section className="border-b border-line pb-6">
          <div className="mb-3">
            <CardTitle>Noch offen aus früheren Tagen</CardTitle>
          </div>
          <div className="flex flex-col gap-2">
            {day.carryover.map((task) => (
              <div key={task.id} className="flex items-center justify-between gap-3">
                <p className="text-sm text-fg">{task.title}</p>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={busy}
                  onClick={() => void apply({ kind: "move_task", id: task.id, targetDay: day.day })}
                >
                  In diesen Tag übertragen
                </Button>
              </div>
            ))}
          </div>
        </section>
      )}
      {day.timeEntries.length > 0 && (
        <section className="border-b border-line pb-6">
          <div className="mb-3">
            <CardTitle>{t("myDay.assignTime")}</CardTitle>
            <CardDescription>
              Eine Zeitbuchung wird genau einer Aufgabe zugeordnet. Projektzuordnungen werden
              automatisch berücksichtigt.
            </CardDescription>
          </div>
          <div className="flex flex-col gap-3">
            {day.timeEntries.map((entry) => (
              <div key={entry.id} className="flex flex-wrap items-center gap-3">
                <span className="min-w-0 flex-1 text-sm text-fg-muted">
                  {entry.description || "Zeitbuchung"} · {formatDuration(entry.durationMs)}
                </span>
                <NativeSelect
                  aria-label={`Aufgabe für ${entry.description || "Zeitbuchung"}`}
                  value={
                    day.tasks.some((task) => task.id === entry.assignedTo)
                      ? (entry.assignedTo ?? "")
                      : ""
                  }
                  disabled={busy}
                  onChange={(event) =>
                    void apply(
                      event.target.value
                        ? { kind: "link_time", id: event.target.value, entryId: entry.id }
                        : { kind: "unlink_time", entryId: entry.id },
                    )
                  }
                >
                  <NativeSelectOption value="">{t("myDay.noManualAssignment")}</NativeSelectOption>
                  {day.tasks.map((task) => (
                    <NativeSelectOption key={task.id} value={task.id}>
                      {task.title}
                    </NativeSelectOption>
                  ))}
                </NativeSelect>
              </div>
            ))}
          </div>
        </section>
      )}
      {editing && (
        <TaskEditor
          key={editing === "new" ? "new" : editing.id}
          open
          error={error}
          task={editing === "new" ? undefined : editing}
          apply={apply}
          busy={busy}
          onClose={() => setEditing(null)}
        />
      )}
      {help && (
        <Assistance
          key={help.id}
          task={help}
          day={day}
          timeZone={day.timeZone}
          mutationError={error}
          aiEnabled={day.preferences.aiEnabled && day.preferences.enabled}
          onClose={() => setHelp(null)}
          apply={apply}
          busy={busy}
        />
      )}
    </div>
  );
}
