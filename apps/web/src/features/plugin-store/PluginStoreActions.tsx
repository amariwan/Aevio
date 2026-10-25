import { PackageOpen, Power, RotateCcw, ShieldOff, Trash2, Wrench } from "lucide-react";
import { Button } from "@/components/ui/button";
import * as api from "@/lib/api";
import type { PluginCatalogEntry } from "@/lib/types";

type Props = {
  install: string;
  installation: NonNullable<PluginCatalogEntry["installed"]>;
  busyId: string | undefined;
  missingPermissions: string[];
  release: PluginCatalogEntry["releases"][number] | undefined;
  permissions: string[];
  recoverable: boolean;
  registry?: { url: string; token: string };
  run: (pluginId: string, action: () => Promise<unknown>) => Promise<void>;
};

export function PluginStoreActions({
  install,
  installation,
  busyId,
  missingPermissions,
  release,
  permissions,
  recoverable,
  registry,
  run,
}: Props) {
  const disabled = busyId === install;
  return (
    <>
      <Button
        disabled={disabled}
        onClick={() =>
          void run(install, async () => {
            if (missingPermissions.length > 0)
              await api.pluginGrantPermissions(install, missingPermissions);
            await api.pluginSetEnabled(install, installation.state !== "active");
          })
        }
      >
        <Power className="size-4" />
        {installation.state === "active"
          ? "Disable"
          : missingPermissions.length > 0
            ? "Grant & enable"
            : "Enable"}
      </Button>
      {permissions.length > 0 && (
        <Button
          variant="outline"
          disabled={disabled}
          onClick={() =>
            void run(install, () =>
              api.pluginRevokePermissions(install, permissions).then(() => undefined),
            )
          }
        >
          <ShieldOff className="size-4" /> Revoke permissions
        </Button>
      )}
      <Button
        variant="destructive"
        disabled={disabled || installation.state === "active"}
        onClick={() => void run(install, () => api.pluginUninstall(install))}
      >
        <Trash2 className="size-4" /> Uninstall
      </Button>
      <Button
        variant="destructive"
        disabled={disabled || installation.state === "active"}
        onClick={() => {
          if (window.confirm("Delete all stored data for this plugin? This cannot be undone.")) {
            void run(install, () => api.pluginDeleteData(install));
          }
        }}
      >
        <Trash2 className="size-4" /> Delete plugin data
      </Button>
      {recoverable && (
        <Button
          variant="outline"
          disabled={disabled}
          onClick={() => void run(install, () => api.pluginRecover(install))}
        >
          <Wrench className="size-4" /> Recover
        </Button>
      )}
      {release && installation.state !== "active" && (
        <Button
          variant="outline"
          disabled={disabled}
          onClick={() =>
            void run(install, async () => {
              if (registry) {
                await api.pluginInstallRemote({
                  registryUrl: registry.url,
                  bearerToken: registry.token,
                  pluginId: install,
                  release,
                });
              } else {
                const packagePath = await api.pluginPickPackage();
                if (packagePath)
                  await api.pluginInstallPackage(packagePath, release, missingPermissions, false);
              }
            })
          }
        >
          <PackageOpen className="size-4" /> {registry ? "Download update" : "Install package"}
        </Button>
      )}
      {installation.state !== "active" && (
        <Button
          variant="outline"
          disabled={disabled}
          onClick={() => void run(install, () => api.pluginRollback(install))}
        >
          <RotateCcw className="size-4" /> Roll back
        </Button>
      )}
    </>
  );
}
