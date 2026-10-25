import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { Intervention } from "../types";
import * as api from "../api";

export function InterventionCard({
  intervention,
  day,
  onDismiss,
}: {
  intervention: Intervention;
  day: string;
  onDismiss: () => void;
}) {
  return (
    <Card className="rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
      <CardHeader>
        <CardTitle>{intervention.title}</CardTitle>
        <CardDescription>{intervention.message}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-wrap gap-2">
        {intervention.actions.map((action) => (
          <Button
            key={action.id}
            variant={action.kind === "dismiss" ? "ghost" : "outline"}
            onClick={() => {
              if (action.kind === "dismiss") {
                void api.dismissIntervention(day, intervention.kind).then(() => onDismiss());
              } else {
                onDismiss();
              }
            }}
          >
            {action.label}
          </Button>
        ))}
        <Button
          variant="ghost"
          onClick={() =>
            void api.snoozeIntervention(day, intervention.kind, 30).then(() => onDismiss())
          }
        >
          Später (30 Min.)
        </Button>
      </CardContent>
    </Card>
  );
}
