import { useRef, useState } from "react";
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
import { Textarea } from "@/components/ui/textarea";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import type { Apply, PlannedTask } from "../types";
import { parseLocalDatetime, toLocalDatetimeInput } from "../date";
export function TaskEditor({
  task,
  open,
  onClose,
  apply,
  busy,
  error,
}: {
  task?: PlannedTask;
  open: boolean;
  onClose: () => void;
  apply: Apply;
  busy: boolean;
  error: string | null;
}) {
  const { t } = useI18n();
  const reminderInputRef = useRef<HTMLInputElement>(null);
  const [reminder, setReminder] = useState(() => {
    if (!task?.reminderAt) return "";
    return toLocalDatetimeInput(task.reminderAt);
  });
  const [reminderError, setReminderError] = useState<string | null>(null);
  const [title, setTitle] = useState(task?.title ?? "");
  const [notes, setNotes] = useState(task?.notes ?? "");
  const [priority, setPriority] = useState(task?.priority ?? 3);
  const [estimateMinutes, setEstimate] = useState(task?.estimateMinutes ?? 0);
  const [important, setImportant] = useState(task?.important ?? false);
  return (
    <Dialog
      open={open}
      onOpenChange={(value) => {
        if (!value && !busy) onClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{task ? "Aufgabe bearbeiten" : "Eine Aufgabe festhalten"}</DialogTitle>
          <DialogDescription>
            {task?.task.source === "project"
              ? "Titel, Notizen und Priorität ändern die bestehende Projektaufgabe."
              : "Ein nächster Schritt, den du später jederzeit ändern kannst."}
          </DialogDescription>
        </DialogHeader>
        <form
          className="flex flex-col gap-5"
          noValidate
          onSubmit={(event) => {
            event.preventDefault();
            // Read the reminder directly from the DOM input: WebKit's native
            // date spinner (AXIncrementor) updates the input's `.value` but
            // does not always emit a React `change` event, so the controlled
            // `reminder` state can lag behind. Prefer the live DOM value.
            const domInput = document.getElementById(
              "coach-task-reminder",
            ) as HTMLInputElement | null;
            const liveReminder = (
              domInput?.value ||
              reminderInputRef.current?.value ||
              reminder
            ).trim();
            if (liveReminder) {
              const parsed = parseLocalDatetime(liveReminder);
              if (parsed === null) {
                setReminderError("Ungültiges Datum.");
                return;
              }
              if (parsed < Date.now()) {
                setReminderError("Erinnerung muss in der Zukunft liegen.");
                return;
              }
              setReminder(liveReminder);
            } else {
              setReminderError(null);
            }
            const reminderAt = liveReminder ? parseLocalDatetime(liveReminder) : null;
            void apply(
              task
                ? {
                    kind: "edit_task",
                    reminderAt,
                    id: task.id,
                    title,
                    notes,
                    priority,
                    estimateMinutes,
                    important,
                  }
                : {
                    kind: "create_task",
                    reminderAt,
                    title,
                    notes,
                    priority,
                    estimateMinutes,
                    important,
                  },
            ).then((ok) => {
              if (ok) onClose();
            });
          }}
        >
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="coach-task-title">{t("taskEditor.task")}</FieldLabel>
              <Input
                id="coach-task-title"
                autoFocus
                required
                maxLength={500}
                value={title}
                onChange={(event) => setTitle(event.target.value)}
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="coach-task-notes">Kontext / Notizen</FieldLabel>
              <Textarea
                id="coach-task-notes"
                maxLength={10000}
                value={notes}
                onChange={(event) => setNotes(event.target.value)}
              />
            </Field>
            <div className="grid grid-cols-2 gap-3">
              <Field>
                <FieldLabel htmlFor="coach-estimate">Geschätzte Minuten</FieldLabel>
                <Input
                  id="coach-estimate"
                  type="number"
                  min={0}
                  max={1440}
                  required
                  value={estimateMinutes}
                  onChange={(event) => setEstimate(Number(event.target.value))}
                />
              </Field>
              <Field>
                <FieldLabel htmlFor="coach-priority">Priorität</FieldLabel>
                <NativeSelect
                  id="coach-priority"
                  value={priority}
                  onChange={(event) => setPriority(Number(event.target.value))}
                >
                  {[1, 2, 3, 4, 5].map((value) => (
                    <NativeSelectOption key={value} value={value}>
                      {value}
                      {value === 5 ? " · hoch" : ""}
                    </NativeSelectOption>
                  ))}
                </NativeSelect>
              </Field>
            </div>
            {(!task || task.task.source === "personal") && (
              <Field>
                <FieldLabel htmlFor="coach-task-reminder">Native Erinnerung, freiwillig</FieldLabel>
                <Input
                  id="coach-task-reminder"
                  ref={reminderInputRef}
                  type="datetime-local"
                  step={60}
                  defaultValue={task?.reminderAt ? toLocalDatetimeInput(task.reminderAt) : ""}
                  onChange={(event) => {
                    setReminder(event.target.value);
                    setReminderError(null);
                  }}
                />
                {reminderError && (
                  <p role="alert" className="text-sm text-danger">
                    {reminderError}
                  </p>
                )}
                <p className="text-xs text-fg-muted">
                  Benötigt aktivierte Erinnerungen in den Coach-Einstellungen. Leer lassen, um keine
                  Erinnerung zu planen.
                </p>
              </Field>
            )}
            <Button
              type="button"
              variant="outline"
              aria-pressed={important}
              onClick={() => setImportant(!important)}
            >
              {important ? "Als besonders wichtig markiert" : "Als besonders wichtig markieren"}
            </Button>
          </FieldGroup>
          {error && (
            <p role="alert" className="text-sm text-danger">
              {error}
            </p>
          )}
          <Button type="submit" disabled={busy || !title.trim()}>
            Aufgabe speichern
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
