import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
export interface Routine {
  id: string;
  title: string;
  tasks: { title: string; estimateMinutes: number }[];
}
export type RoutineCommand =
  | { kind: "save_routine"; routine: Routine }
  | { kind: "remove_routine"; id: string }
  | { kind: "apply_routine"; id: string };
export function Routines({
  routines,
  run,
  busy,
  day,
}: {
  routines: Routine[];
  run: (command: RoutineCommand) => Promise<boolean>;
  busy: boolean;
  day: string;
}) {
  const { t } = useI18n();
  const [editing, setEditing] = useState<Routine | null>(null);
  const [title, setTitle] = useState("");
  const [lines, setLines] = useState("");
  const [error, setError] = useState<string | null>(null);
  async function save() {
    const tasks = lines
      .split("\n")
      .filter((line) => line.trim())
      .map((line) => {
        const divider = line.lastIndexOf("|");
        return {
          title: (divider < 0 ? line : line.slice(0, divider)).trim(),
          estimateMinutes: divider < 0 ? 0 : Number(line.slice(divider + 1).trim()),
        };
      });
    if (
      tasks.some(
        (task) =>
          !Number.isInteger(task.estimateMinutes) ||
          task.estimateMinutes < 0 ||
          task.estimateMinutes > 1440,
      )
    ) {
      setError("Bitte ganze Minuten zwischen 0 und 1440 hinter | eintragen.");
      return;
    }
    if (await run({ kind: "save_routine", routine: { id: editing?.id ?? "", title, tasks } })) {
      setTitle("");
      setLines("");
      setEditing(null);
      setError(null);
    }
  }
  return (
    <Card className="rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
      <CardHeader>
        <CardTitle>{t("routines.title")}</CardTitle>
        <CardDescription>
          Vorlagen werden nur durch „In Tagesplan übernehmen“ angewendet, höchstens einmal pro
          Routine und Tag.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <form
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault();
            void save();
          }}
        >
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="coach-routine-title">{t("routines.name")}</FieldLabel>
              <Input
                id="coach-routine-title"
                required
                maxLength={500}
                value={title}
                onChange={(event) => setTitle(event.target.value)}
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="coach-routine-tasks">
                Aufgaben · je eine Zeile: Titel | Minuten
              </FieldLabel>
              <Textarea
                id="coach-routine-tasks"
                required
                rows={4}
                value={lines}
                maxLength={15000}
                placeholder={t("routines.examplePrompt")}
                onChange={(event) => setLines(event.target.value)}
              />
            </Field>
          </FieldGroup>
          {error && (
            <p role="alert" className="text-sm text-danger">
              {error}
            </p>
          )}
          <div className="flex gap-2">
            <Button type="submit" disabled={busy}>
              Routine speichern
            </Button>
            {editing && (
              <Button
                variant="ghost"
                onClick={() => {
                  setEditing(null);
                  setTitle("");
                  setLines("");
                }}
              >
                Abbrechen
              </Button>
            )}
          </div>
        </form>
        {routines.map((routine) => (
          <div key={routine.id} className="flex flex-col gap-2 rounded-lg border border-line p-3">
            <p className="font-medium">{routine.title}</p>
            <ol className="list-inside list-decimal text-sm text-fg-muted">
              {routine.tasks.map((task, i) => (
                <li key={i}>
                  {task.title} · {task.estimateMinutes} Min.
                </li>
              ))}
            </ol>
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => void run({ kind: "apply_routine", id: routine.id })}
              >
                In Tagesplan {day} übernehmen
              </Button>
              <Button
                variant="ghost"
                disabled={busy}
                onClick={() => {
                  setEditing(routine);
                  setTitle(routine.title);
                  setLines(
                    routine.tasks
                      .map((task) => `${task.title} | ${task.estimateMinutes}`)
                      .join("\n"),
                  );
                }}
              >
                Bearbeiten
              </Button>
              <Button
                variant="ghost"
                disabled={busy}
                onClick={() => void run({ kind: "remove_routine", id: routine.id })}
              >
                Löschen
              </Button>
            </div>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
