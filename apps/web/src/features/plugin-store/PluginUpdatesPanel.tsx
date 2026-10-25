import { ArrowUpCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import * as api from "@/lib/api";
import type { PluginCatalogEntry, PluginUpdateCandidate } from "@/lib/types";

type Props = {
  candidates: PluginUpdateCandidate[];
  entries: PluginCatalogEntry[];
  registry: { url: string; token: string };
  busyId: string | undefined;
  run: (pluginId: string, action: () => Promise<unknown>) => Promise<void>;
};

export function PluginUpdatesPanel({ candidates, entries, registry, busyId, run }: Props) {
  if (candidates.length === 0) return null;

  return (
    <Card className="bg-surface">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <ArrowUpCircle className="size-4 text-accent" /> Available updates
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {candidates.map((candidate) => {
          const entry = entries.find((item) => item.manifest.id === candidate.pluginId);
          const wasActive = entry?.installed?.state === "active";
          const busy = busyId === candidate.pluginId;
          return (
            <div key={candidate.pluginId} className="rounded-lg border border-line p-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div>
                  <p className="text-sm font-medium text-fg">{candidate.pluginId}</p>
                  <p className="text-xs text-fg-muted">
                    v{candidate.currentVersion} → v{candidate.release.version}
                  </p>
                </div>
                <Button
                  size="sm"
                  disabled={busy}
                  onClick={() =>
                    void run(candidate.pluginId, async () => {
                      if (wasActive) await api.pluginSetEnabled(candidate.pluginId, false);
                      try {
                        await api.pluginInstallRemote({
                          registryUrl: registry.url,
                          bearerToken: registry.token,
                          pluginId: candidate.pluginId,
                          release: candidate.release,
                          permissions: candidate.addedPermissions,
                          enable: wasActive,
                        });
                      } catch (error: unknown) {
                        if (wasActive) await api.pluginSetEnabled(candidate.pluginId, true);
                        throw error;
                      }
                    })
                  }
                >
                  {candidate.requiresPermissionApproval ? "Approve & update" : "Update"}
                </Button>
              </div>
              {candidate.addedPermissions.length > 0 && (
                <p className="mt-2 text-xs text-fg-muted">
                  New permissions: {candidate.addedPermissions.join(", ")}
                </p>
              )}
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}
