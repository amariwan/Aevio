import { ExternalLink, PackageOpen } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import * as api from "@/lib/api";
import type { PluginCatalogEntry } from "@/lib/types";
import { PluginStoreActions } from "./PluginStoreActions";
import { PluginUpdatePolicyPanel } from "./PluginUpdatePolicyPanel";

type DetailsProps = {
  selected: PluginCatalogEntry;
  busyId: string | undefined;
  run: (pluginId: string, action: () => Promise<unknown>) => Promise<void>;
  registry?: { url: string; token: string };
};

export function PluginStoreDetails({ selected, busyId, run, registry }: DetailsProps) {
  const install = selected.manifest.id;
  const recordedInstallation = selected.installed;
  const installation =
    recordedInstallation?.state === "uninstalled" ? undefined : recordedInstallation;
  const missingPermissions = selected.manifest.permissions
    .map((permission) => permission.id)
    .filter((permission) => !installation?.grantedPermissions.includes(permission));
  const release = selected.releases.find((item) => !item.revoked);
  const recoverable =
    installation?.state === "failed" ||
    installation?.state === "quarantined" ||
    installation?.state === "recovery_required";

  return (
    <div className="space-y-4">
      <Card className="h-fit bg-surface">
        <CardHeader className="gap-3">
          <div className="flex items-start justify-between gap-3">
            <div className="flex min-w-0 items-center gap-2">
              <PackageOpen className="size-5 shrink-0 text-accent" />
              <CardTitle className="truncate">{selected.manifest.name}</CardTitle>
            </div>
            <Badge variant="secondary">{selected.manifest.category}</Badge>
          </div>
          <p className="text-sm text-fg-muted">
            {selected.manifest.publisher.displayName} · v{selected.manifest.version}
          </p>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-fg-muted">{selected.manifest.description}</p>
          <div className="rounded-lg bg-surface-strong p-3">
            <p className="mb-2 text-xs font-semibold text-fg">Requested permissions</p>
            <ul className="space-y-1.5 text-xs text-fg-muted">
              {selected.manifest.permissions.map((permission) => (
                <li key={permission.id} className="flex justify-between gap-2">
                  <span>{permission.id}</span>
                  <span>{permission.scope ?? "session"}</span>
                </li>
              ))}
            </ul>
          </div>
          <div className="flex flex-wrap gap-2 border-t border-line pt-3">
            {!installation ? (
              <Button
                disabled={busyId === install}
                onClick={() =>
                  void run(install, async () => {
                    if (registry && release) {
                      await api.pluginInstallRemote({
                        registryUrl: registry.url,
                        bearerToken: registry.token,
                        pluginId: install,
                        release,
                      });
                    } else {
                      const plan = await api.pluginPlanInstall(install);
                      await api.pluginInstall(install, plan);
                    }
                  })
                }
              >
                Install
              </Button>
            ) : (
              <PluginStoreActions
                install={install}
                installation={installation}
                busyId={busyId}
                missingPermissions={missingPermissions}
                release={release}
                permissions={selected.manifest.permissions.map((permission) => permission.id)}
                recoverable={recoverable}
                registry={registry}
                run={run}
              />
            )}
            {recordedInstallation?.state === "uninstalled" && (
              <Button
                variant="destructive"
                disabled={busyId === install}
                onClick={() => {
                  if (
                    window.confirm("Delete all stored data for this plugin? This cannot be undone.")
                  ) {
                    void run(install, () => api.pluginDeleteData(install));
                  }
                }}
              >
                Delete plugin data
              </Button>
            )}
            {installation?.state === "active" && selected.manifest.entrypoints.ui && (
              <Button
                variant="outline"
                disabled={busyId === install}
                onClick={() => void run(install, () => api.pluginOpenUi(install))}
                aria-label={`Open ${selected.manifest.name}`}
              >
                <ExternalLink className="size-4" /> Open plugin UI
              </Button>
            )}
          </div>
        </CardContent>
      </Card>
      {installation && <PluginUpdatePolicyPanel pluginId={install} releases={selected.releases} />}
    </div>
  );
}
