import { Activity, AlertTriangle, CheckCircle2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import * as api from "@/lib/api";
import type { PluginHealthSummary } from "@/lib/types";

function tone(state: string): "default" | "secondary" | "destructive" {
  if (state === "healthy") return "secondary";
  if (state === "failed" || state === "quarantined") return "destructive";
  return "default";
}

export function PluginHealthPanel() {
  const [items, setItems] = useState<PluginHealthSummary[]>([]);
  const [error, setError] = useState<string>();
  const load = useCallback(async () => {
    try {
      setItems(await api.pluginHealth());
      setError(undefined);
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    }
  }, []);
  useEffect(() => void load(), [load]);
  useTauriEvent(api.PLUGINS_CHANGED, () => void load());

  return (
    <Card className="bg-surface">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Activity className="size-4 text-accent" /> Plugin health
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {error && <p className="text-sm text-danger">{error}</p>}
        {items.length === 0 && !error && (
          <p className="text-sm text-fg-muted">No installed plugins to monitor.</p>
        )}
        {items.map((item) => {
          const unhealthy = item.state === "failed" || item.state === "quarantined";
          return (
            <div
              key={item.pluginId}
              className="flex items-start gap-3 rounded-lg border border-line px-3 py-2"
            >
              {unhealthy ? (
                <AlertTriangle className="mt-0.5 size-4 shrink-0 text-danger" />
              ) : (
                <CheckCircle2 className="mt-0.5 size-4 shrink-0 text-success" />
              )}
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium text-fg">{item.pluginId}</span>
                {item.lastError && (
                  <span className="block truncate text-xs text-danger">{item.lastError}</span>
                )}
              </span>
              <Badge variant={tone(item.state)}>{item.state}</Badge>
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}
