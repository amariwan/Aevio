import { useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import * as api from "@/lib/api";
import type {
  PluginCatalogEntry,
  PluginUpdateChannel,
  PluginWorkspaceDesiredState,
} from "@/lib/types";

type Props = {
  workspaceId: string;
  registry: { url: string; token: string };
  entries: PluginCatalogEntry[];
  states: PluginWorkspaceDesiredState[];
  onSaved: (state: PluginWorkspaceDesiredState) => void;
};

export function PluginWorkspaceStatePanel({
  workspaceId,
  registry,
  entries,
  states,
  onSaved,
}: Props) {
  const [selectedPlugin, setSelectedPlugin] = useState("");
  const [busyId, setBusyId] = useState<string>();
  const [error, setError] = useState<string>();
  const available = useMemo(
    () => entries.filter((entry) => !states.some((state) => state.pluginId === entry.manifest.id)),
    [entries, states],
  );

  const save = async (
    pluginId: string,
    input: {
      requestedVersion?: string | null;
      channel: PluginUpdateChannel;
      enabled: boolean;
      requestedPermissions: string[];
    },
  ) => {
    try {
      setBusyId(pluginId);
      setError(undefined);
      const state = await api.pluginSetWorkspacePluginState({
        registryUrl: registry.url,
        bearerToken: registry.token,
        workspaceId,
        pluginId,
        input,
      });
      onSaved(state);
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    } finally {
      setBusyId(undefined);
    }
  };

  return (
    <Card className="bg-surface">
      <CardHeader>
        <CardTitle className="text-base">Workspace plugin intent</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <p className="text-xs text-fg-muted">
          Server intent only. Each device still verifies artifacts and grants local permissions.
        </p>
        {error && <p className="text-sm text-danger">{error}</p>}
        {available.length > 0 && (
          <div className="flex gap-2">
            <NativeSelect
              value={selectedPlugin}
              onChange={(event) => setSelectedPlugin(event.target.value)}
              aria-label="Plugin to add to workspace"
            >
              <NativeSelectOption value="">Add a plugin…</NativeSelectOption>
              {available.map((entry) => (
                <NativeSelectOption key={entry.manifest.id} value={entry.manifest.id}>
                  {entry.manifest.name}
                </NativeSelectOption>
              ))}
            </NativeSelect>
            <Button
              size="sm"
              disabled={!selectedPlugin || busyId !== undefined}
              onClick={() => {
                const entry = entries.find((item) => item.manifest.id === selectedPlugin);
                if (!entry) return;
                void save(selectedPlugin, {
                  requestedVersion: entry.manifest.version,
                  channel: "stable",
                  enabled: false,
                  requestedPermissions: [],
                });
                setSelectedPlugin("");
              }}
            >
              Add
            </Button>
          </div>
        )}
        {states.length === 0 ? (
          <p className="text-sm text-fg-muted">No workspace plugin intent has been published.</p>
        ) : (
          states.map((state) => {
            const entry = entries.find((item) => item.manifest.id === state.pluginId);
            return (
              <div
                key={state.pluginId}
                className="grid gap-2 rounded-lg border border-line p-3 sm:grid-cols-[minmax(0,1fr)_8rem_auto]"
              >
                <div>
                  <p className="text-sm font-medium text-fg">
                    {entry?.manifest.name ?? state.pluginId}
                  </p>
                  <p className="text-xs text-fg-muted">
                    {state.requestedVersion ?? "latest"} · revision {state.revision}
                  </p>
                </div>
                <NativeSelect
                  value={state.channel}
                  disabled={busyId === state.pluginId}
                  aria-label={`Update channel for ${state.pluginId}`}
                  onChange={(event) =>
                    void save(state.pluginId, {
                      requestedVersion: state.requestedVersion,
                      channel: event.target.value as PluginUpdateChannel,
                      enabled: state.enabled,
                      requestedPermissions: state.requestedPermissions,
                    })
                  }
                >
                  <NativeSelectOption value="stable">Stable</NativeSelectOption>
                  <NativeSelectOption value="beta">Beta</NativeSelectOption>
                  <NativeSelectOption value="development">Development</NativeSelectOption>
                </NativeSelect>
                <Button
                  size="sm"
                  variant={state.enabled ? "secondary" : "outline"}
                  disabled={busyId === state.pluginId}
                  onClick={() =>
                    void save(state.pluginId, {
                      requestedVersion: state.requestedVersion,
                      channel: state.channel,
                      enabled: !state.enabled,
                      requestedPermissions: state.requestedPermissions,
                    })
                  }
                >
                  {state.enabled ? "Enabled" : "Disabled"}
                </Button>
              </div>
            );
          })
        )}
      </CardContent>
    </Card>
  );
}
