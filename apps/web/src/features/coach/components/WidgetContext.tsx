import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useCoach } from "../useCoach";
import { useCoachFocus } from "../useCoachFocus";
import { localDay } from "../date";
/** A view of the same persisted focus context in the existing widget window. */
export function CoachWidgetContext() {
  const { snapshot } = useCoach(localDay(), Intl.DateTimeFormat().resolvedOptions().timeZone);
  const { focus, busy, error, action } = useCoachFocus();
  if (!snapshot?.preferences.enabled || !snapshot.preferences.widgetContext || !focus) return null;
  return (
    <Card className="rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
      <CardContent className="flex max-h-36 flex-col gap-2 overflow-auto p-3" data-widget-action>
        <p className="truncate text-sm font-medium text-fg">{focus.title}</p>
        <p className="text-xs text-fg-muted">{focus.goal}</p>
        <p className="text-xs text-fg">{focus.nextStep}</p>
        <p className="text-xs text-fg-muted">
          {snapshot.tasks.filter((task) => task.done).length}/{snapshot.tasks.length} Aufgaben
          bewusst abgeschlossen
        </p>
        <Button
          size="sm"
          variant="outline"
          disabled={busy}
          onClick={() =>
            void action(
              focus.state === "running" ? "coach_end_focus" : "coach_start_focus",
              focus.state === "running"
                ? { finish: false }
                : {
                    day: snapshot.day,
                    timeZone: snapshot.timeZone,
                    id: focus.taskId,
                    minutes: focus.plannedMinutes,
                    nextStep: focus.nextStep,
                    bodyDoubling: focus.bodyDoubling,
                  },
            )
          }
        >
          {focus.state === "running" ? "Fokus pausieren" : "Mit Kontext fortsetzen"}
        </Button>
        {error && (
          <p role="alert" className="text-xs text-danger">
            {error}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
