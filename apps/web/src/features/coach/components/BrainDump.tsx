import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Textarea } from "@/components/ui/textarea";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { COACH_DAY_CHANGED, describeError } from "@/lib/api";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { Routines, type Routine, type RoutineCommand } from "./Routines";
import * as api from "../api";
import type { Day } from "../types";
type Category = "task" | "idea" | "reminder" | "note" | "question" | "later";
interface Entry {
  id: string;
  content: string;
  category: Category;
  convertedTaskId: string | null;
  updatedAt: number;
}
interface Brain {
  entries: Entry[];
  routines: Routine[];
}
const categories: Record<Category, string> = {
  task: "Aufgabe",
  idea: "Idee",
  reminder: "Erinnerung",
  note: "Notiz",
  question: "Frage",
  later: "Später / irgendwann",
};
type Command =
  | { kind: "save_entry"; entry: Entry }
  | { kind: "remove_entry"; id: string }
  | { kind: "convert"; id: string }
  | RoutineCommand;
export function BrainDump({ day }: { day: Day }) {
  const { t } = useI18n();
  const [brain, setBrain] = useState<Brain | null>(null);
  const [content, setContent] = useState("");
  const [category, setCategory] = useState<Category>("note");
  const [editing, setEditing] = useState<Entry | null>(null);
  const [suggestion, setSuggestion] = useState<Category | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const load = useCallback(async () => {
    try {
      setBrain(await api.fetchBrain<Brain>());
    } catch (cause) {
      setError(describeError(cause));
    }
  }, []);
  useEffect(() => {
    void load();
  }, [load]);
  useTauriEvent(COACH_DAY_CHANGED, () => void load());
  async function run(command: Command): Promise<boolean> {
    setBusy(true);
    setError(null);
    try {
      setBrain(await api.applyBrain<Brain, Command>(day.day, day.timeZone, command));
      setSaved(true);
      return true;
    } catch (cause) {
      setError(describeError(cause));
      return false;
    } finally {
      setBusy(false);
    }
  }
  async function classify() {
    setBusy(true);
    setError(null);
    try {
      setSuggestion(await api.classifyThought<Category>(content));
    } catch (cause) {
      setError(describeError(cause));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="flex flex-col gap-5">
      <Card className="rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
        <CardHeader>
          <CardTitle>{t("brainDump.title")}</CardTitle>
          <CardDescription>
            Du musst sie noch nicht ordnen. Kategorien sind bearbeitbar; eine Erinnerungskategorie
            plant noch keine Benachrichtigung.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-5">
          <form
            className="flex flex-col gap-4"
            onSubmit={(event) => {
              event.preventDefault();
              void run({
                kind: "save_entry",
                entry: {
                  id: editing?.id ?? "",
                  content,
                  category,
                  convertedTaskId: editing?.convertedTaskId ?? null,
                  updatedAt: 0,
                },
              }).then((ok) => {
                if (ok) {
                  setContent("");
                  setEditing(null);
                  setSuggestion(null);
                }
              });
            }}
          >
            <FieldGroup>
              <Field>
                <FieldLabel htmlFor="coach-brain-content">Gedanke, Idee oder Notiz</FieldLabel>
                <Textarea
                  id="coach-brain-content"
                  autoFocus
                  required
                  rows={4}
                  maxLength={10000}
                  value={content}
                  onChange={(event) => {
                    setContent(event.target.value);
                    setSuggestion(null);
                    setSaved(false);
                  }}
                />
              </Field>
              <Field>
                <FieldLabel htmlFor="coach-brain-category">{t("brainDump.category")}</FieldLabel>
                <NativeSelect
                  id="coach-brain-category"
                  value={category}
                  onChange={(event) => setCategory(event.target.value as Category)}
                >
                  {Object.entries(categories).map(([key, label]) => (
                    <NativeSelectOption key={key} value={key}>
                      {label}
                    </NativeSelectOption>
                  ))}
                </NativeSelect>
              </Field>
            </FieldGroup>
            <div className="flex flex-wrap gap-2">
              <Button type="submit" disabled={busy || !content.trim()}>
                Gedanken speichern
              </Button>
              <Button
                type="button"
                variant="outline"
                disabled={busy || !content.trim()}
                onClick={() => void classify()}
              >
                Kategorie vorschlagen
              </Button>
              {editing && (
                <Button
                  type="button"
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
          {suggestion && (
            <div className="flex flex-wrap items-center gap-3 text-sm text-fg-muted">
              <p>Lokale Regel schlägt „{categories[suggestion]}“ vor.</p>
              <Button
                variant="outline"
                onClick={() => {
                  setCategory(suggestion);
                  setSuggestion(null);
                }}
              >
                Kategorie für Entwurf übernehmen
              </Button>
            </div>
          )}
          {error && (
            <p role="alert" className="text-sm text-danger">
              {error}
            </p>
          )}
          {saved && (
            <p role="status" className="text-sm text-fg-muted">
              Gespeichert
            </p>
          )}
          {!brain ? (
            <p role="status">Gedanken laden …</p>
          ) : brain.entries.length === 0 ? (
            <p className="text-sm text-fg-muted">Hier ist Platz für deine ersten Gedanken.</p>
          ) : (
            brain.entries.map((entry) => (
              <div key={entry.id} className="flex flex-col gap-2 rounded-lg border border-line p-3">
                <p className="whitespace-pre-wrap text-sm text-fg">{entry.content}</p>
                <p className="text-xs text-fg-muted">
                  {categories[entry.category]}
                  {entry.convertedTaskId ? " · in Aufgabe umgewandelt" : ""}
                </p>
                <div className="flex flex-wrap gap-2">
                  <Button
                    variant="outline"
                    disabled={busy || !!entry.convertedTaskId}
                    onClick={() => void run({ kind: "convert", id: entry.id })}
                  >
                    Als Aufgabe am {day.day} einplanen
                  </Button>
                  <Button
                    variant="ghost"
                    disabled={busy}
                    onClick={() => {
                      setEditing(entry);
                      setContent(entry.content);
                      setCategory(entry.category);
                    }}
                  >
                    Bearbeiten
                  </Button>
                  <Button
                    variant="ghost"
                    disabled={busy}
                    onClick={() => void run({ kind: "remove_entry", id: entry.id })}
                  >
                    Löschen
                  </Button>
                </div>
              </div>
            ))
          )}
        </CardContent>
      </Card>
      {brain && <Routines routines={brain.routines} run={run} busy={busy} day={day.day} />}
    </div>
  );
}
