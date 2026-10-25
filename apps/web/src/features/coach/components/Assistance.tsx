import { useState } from "react";
import { useI18n } from "@/hooks/useI18n";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { Textarea } from "@/components/ui/textarea";
import { describeError } from "@/lib/api";
import * as api from "../api";
import type { Apply, Day, PlannedTask, Proposal, StartingReason } from "../types";
const reasons: Record<StartingReason, string> = {
  cannot_start: "Ich kann nicht anfangen",
  unsure: "Ich weiß nicht, womit ich anfangen soll",
  too_large: "Die Aufgabe ist zu groß",
  low_energy: "Ich habe wenig Energie",
  distracted: "Ich bin abgelenkt",
  first_step: "Ich brauche einen kleinen ersten Schritt",
};
export function Assistance({
  task,
  day,
  timeZone,
  aiEnabled,
  onClose,
  apply,
  busy,
  mutationError,
}: {
  task: PlannedTask;
  day: Day;
  timeZone: string;
  aiEnabled: boolean;
  onClose: () => void;
  apply: Apply;
  busy: boolean;
  mutationError: string | null;
}) {
  const { t } = useI18n();
  const [reason, setReason] = useState<StartingReason>("first_step");
  const [granularity, setGranularity] = useState(2);
  const [proposal, setProposal] = useState<Proposal | null>(null);
  const [steps, setSteps] = useState(task.steps.map((step) => step.title).join("\n"));
  const [error, setError] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);
  async function generate(useAi: boolean) {
    setGenerating(true);
    setError(null);
    try {
      const next = await api.propose({
        title: task.title,
        reason,
        granularity,
        estimateMinutes: task.estimateMinutes,
        previousSteps: steps
          .split("\n")
          .map((line) => line.trim())
          .filter(Boolean),
        useAi,
      });
      setProposal(next);
      setSteps(next.steps.join("\n"));
    } catch (cause) {
      setError(describeError(cause));
    } finally {
      setGenerating(false);
    }
  }
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !generating && !busy) onClose();
      }}
    >
      <DialogContent className="max-h-[85vh] overflow-auto">
        <DialogHeader>
          <DialogTitle>{t("assistance.title")}</DialogTitle>
          <DialogDescription>
            {task.title} · Vorschläge werden erst durch „Schritte übernehmen“ gespeichert.
          </DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="coach-start-reason">Was würde dir gerade helfen?</FieldLabel>
            <NativeSelect
              id="coach-start-reason"
              value={reason}
              onChange={(event) => setReason(event.target.value as StartingReason)}
            >
              {Object.entries(reasons).map(([value, label]) => (
                <NativeSelectOption key={value} value={value}>
                  {label}
                </NativeSelectOption>
              ))}
            </NativeSelect>
          </Field>
          <Field>
            <FieldLabel htmlFor="coach-granularity">Wie klein sollen die Schritte sein?</FieldLabel>
            <NativeSelect
              id="coach-granularity"
              value={granularity}
              onChange={(event) => setGranularity(Number(event.target.value))}
            >
              <NativeSelectOption value={1}>Grob · 3 Schritte</NativeSelectOption>
              <NativeSelectOption value={2}>Konkret · etwa 5 Schritte</NativeSelectOption>
              <NativeSelectOption value={3}>Sehr klein · etwa 7 Schritte</NativeSelectOption>
            </NativeSelect>
          </Field>
        </FieldGroup>
        <div className="flex flex-wrap gap-2">
          <Button disabled={generating || busy} onClick={() => void generate(false)}>
            Lokalen Vorschlag erstellen
          </Button>
          {aiEnabled && (
            <Button
              variant="outline"
              disabled={generating || busy}
              onClick={() => void generate(true)}
            >
              Mit On-Device-KI verfeinern
            </Button>
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            disabled={generating || busy}
            onClick={() => {
              const minutes = Math.min(180, Math.max(2, task.estimateMinutes || 25));
              const nextStep =
                proposal?.nextStep || task.steps.find((step) => !step.done)?.title || task.title;
              void api
                .startFocus({
                  day: day.day,
                  timeZone,
                  id: task.id,
                  minutes,
                  nextStep,
                  bodyDoubling: false,
                })
                .then(() => onClose())
                .catch((cause) => setError(describeError(cause)));
            }}
          >
            Jetzt starten
          </Button>
          <Button variant="ghost" disabled={generating || busy} onClick={onClose}>
            Verwerfen
          </Button>
        </div>
        {mutationError && (
          <p role="alert" className="text-sm text-danger">
            {mutationError}
          </p>
        )}
        {generating && (
          <p role="status" className="text-sm text-fg-muted">
            Vorschlag wird erstellt …
          </p>
        )}
        {error && (
          <p role="alert" className="text-sm text-danger">
            {error}
          </p>
        )}
        {proposal && (
          <div className="flex flex-col gap-2 text-sm">
            <p className="text-accent">{proposal.nextStep}</p>
            <p className="text-fg-muted">
              {proposal.source === "on_device" ? "On-Device-KI" : "Regelbasierter Vorschlag"}.{" "}
              {proposal.note} Du kannst nach zwei Minuten aufhören.
            </p>
          </div>
        )}
        <Field>
          <FieldLabel htmlFor="coach-step-proposal">
            Schritte bearbeiten · eine Zeile pro Schritt
          </FieldLabel>
          <Textarea
            id="coach-step-proposal"
            rows={7}
            maxLength={15000}
            value={steps}
            onChange={(event) => setSteps(event.target.value)}
          />
          <p className="text-xs text-fg-muted">
            Zeilen ändern, entfernen oder ergänzen. Erneutes Generieren ersetzt nur diesen Entwurf.
          </p>
        </Field>
        <Button
          disabled={generating || busy}
          onClick={() =>
            void apply({
              kind: "save_steps",
              id: task.id,
              steps: steps
                .split("\n")
                .map((line) => line.trim())
                .filter(Boolean)
                .map((title) => ({
                  title,
                  done: task.steps.find((step) => step.title === title)?.done ?? false,
                })),
            }).then((ok) => {
              if (ok) onClose();
            })
          }
        >
          Schritte übernehmen
        </Button>
      </DialogContent>
    </Dialog>
  );
}
