import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useCoach } from "../useCoach";
import { localDay } from "../date";

export function MorningOffer({ onOpen }: { onOpen: () => void }) {
  const day = localDay();
  const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const { snapshot, apply, busy, error } = useCoach(day, zone);
  if (
    !snapshot?.preferences.enabled ||
    !snapshot.preferences.morningPlanning ||
    snapshot.plan ||
    new Date().getHours() >= 12
  )
    return null;
  return (
    <Card className="mx-5 mt-3 rounded-none border-0 border-b border-line bg-transparent py-0 ring-0">
      <CardContent className="flex flex-wrap items-center gap-3 p-3">
        <p className="flex-1 text-sm text-fg">
          Möchtest du deinen Tag mit einem Hauptziel beginnen?
        </p>
        <Button onClick={onOpen}>Tagesplan öffnen</Button>
        <Button
          variant="ghost"
          disabled={busy}
          onClick={() =>
            void apply({
              kind: "save_plan",
              plan: {
                day,
                timeZone: zone,
                goal: "",
                notes: "",
                availableMinutes: 0,
                energy: null,
                state: "skipped",
                revision: 0,
              },
            })
          }
        >
          Heute überspringen
        </Button>
        {error && (
          <p role="alert" className="text-sm text-danger">
            {error}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
