import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { describeError } from "@/lib/api";
import * as api from "../api";
import type { Apply, Day } from "../types";
interface Suggestion {
  selected: string[];
  deferred: string[];
  minutes: number;
  capacity: number;
  bufferMinutes: number;
  effectiveCapacity: number;
  reason: string;
  source: string;
}
export function PlanSuggestion({ day, apply, busy }: { day: Day; apply: Apply; busy: boolean }) {
  const { t } = useI18n();
  const [proposal, setProposal] = useState<Suggestion | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function generate(useAi: boolean) {
    setWorking(true);
    setError(null);
    try {
      setProposal(await api.fetchPlanProposal<Suggestion>(day.day, day.timeZone, useAi));
    } catch (cause) {
      setError(describeError(cause));
    } finally {
      setWorking(false);
    }
  }
  return (
    <Card className="rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
      <CardHeader>
        <CardTitle>{t("planSuggestion.title")}</CardTitle>
        <CardDescription>
          Berücksichtigt gespeicherte verfügbare Zeit und bestehende Aufgaben. Termine musst du
          bereits von deiner verfügbaren Zeit abziehen.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            disabled={working || busy || !day.plan}
            onClick={() => void generate(false)}
          >
            Lokal vorschlagen
          </Button>
          {day.preferences.enabled && day.preferences.aiEnabled && (
            <Button
              variant="outline"
              disabled={working || busy || !day.plan}
              onClick={() => void generate(true)}
            >
              Mit On-Device-KI vorschlagen
            </Button>
          )}
        </div>
        {working && (
          <p role="status" className="text-sm text-fg-muted">
            Planungsvorschlag wird erstellt …
          </p>
        )}
        {error && (
          <p role="alert" className="text-sm text-danger">
            {error}
          </p>
        )}
        {proposal && (
          <>
            <p className="text-sm text-fg-muted">
              {proposal.source === "on_device" ? "On-Device-KI" : "Lokale Regeln"}:{" "}
              {proposal.reason}
            </p>
            <p className="text-sm">
              {proposal.minutes} von {proposal.effectiveCapacity} wirksamen Minuten empfohlen
              (verfügbar {proposal.capacity}, Puffer {proposal.bufferMinutes}).
            </p>
            <ol className="list-inside list-decimal text-sm">
              {proposal.selected.map((id) => (
                <li key={id}>{day.tasks.find((task) => task.id === id)?.title}</li>
              ))}
            </ol>
            <p className="text-sm text-fg-muted">
              Zur späteren Entscheidung:{" "}
              {proposal.deferred
                .map((id) => day.tasks.find((task) => task.id === id)?.title)
                .join(", ") || "keine weiteren Aufgaben"}
              . Verschieben kannst du einzelne Aufgaben unter „Mein Tag“.
            </p>
            <Button
              disabled={busy || working}
              onClick={() => {
                const ids = [
                  ...proposal.selected,
                  ...proposal.deferred,
                  ...day.tasks.filter((task) => task.done).map((task) => task.id),
                ];
                void apply({ kind: "reorder", ids });
              }}
            >
              Vorgeschlagene Reihenfolge übernehmen
            </Button>
            <p className="text-xs text-fg-muted">
              Legt keine Aufgaben an und verschiebt keine Aufgabe ohne deinen Klick.
            </p>
          </>
        )}
      </CardContent>
    </Card>
  );
}
