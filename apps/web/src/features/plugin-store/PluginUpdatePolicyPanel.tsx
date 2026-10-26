import { useCallback, useEffect, useState } from "react";
import { useI18n } from "@/hooks/useI18n";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import * as api from "@/lib/api";
import type { PluginRelease, PluginUpdatePolicy } from "@/lib/types";

type Props = {
  pluginId: string;
  releases: PluginRelease[];
};

export function PluginUpdatePolicyPanel({ pluginId, releases }: Props) {
  const { t } = useI18n();
  const [policy, setPolicy] = useState<PluginUpdatePolicy>();
  const [pinnedVersion, setPinnedVersion] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();

  const load = useCallback(async () => {
    try {
      const next = await api.pluginUpdatePolicy(pluginId);
      setPolicy(next);
      setPinnedVersion(next.pinnedVersion ?? "");
      setError(undefined);
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    }
  }, [pluginId]);

  useEffect(() => void load(), [load]);
  useTauriEvent(api.PLUGINS_CHANGED, () => void load());

  const save = async (patch: Partial<PluginUpdatePolicy>) => {
    if (!policy) return;
    try {
      setBusy(true);
      setError(undefined);
      const next = await api.pluginSetUpdatePolicy({ ...policy, ...patch });
      setPolicy(next);
      setPinnedVersion(next.pinnedVersion ?? "");
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };

  if (!policy) return null;
  const availableVersions = releases
    .filter((release) => !release.revoked)
    .map((release) => release.version);
  return (
    <Card className="bg-surface">
      <CardHeader>
        <CardTitle className="text-base">{t("pluginStore.updatePolicy")}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {error && <p className="text-sm text-danger">{error}</p>}
        <div className="grid gap-3 sm:grid-cols-2">
          <label className="space-y-1 text-xs text-fg-muted">
            {t("pluginStore.channel")}
            <NativeSelect
              value={policy.channel}
              disabled={busy}
              onChange={(event) =>
                void save({ channel: event.target.value as PluginUpdatePolicy["channel"] })
              }
            >
              <NativeSelectOption value="stable">{t("pluginStore.stable")}</NativeSelectOption>
              <NativeSelectOption value="beta">{t("pluginStore.beta")}</NativeSelectOption>
              <NativeSelectOption value="development">
                {t("pluginStore.development")}
              </NativeSelectOption>
            </NativeSelect>
          </label>
          <label className="space-y-1 text-xs text-fg-muted">
            {t("pluginStore.pinnedVersion")}
            <NativeSelect
              value={pinnedVersion}
              disabled={busy}
              onChange={(event) => {
                const value = event.target.value;
                setPinnedVersion(value);
                void save({ pinnedVersion: value || undefined });
              }}
            >
              <NativeSelectOption value="">{t("pluginStore.noPin")}</NativeSelectOption>
              {availableVersions.map((version) => (
                <NativeSelectOption key={version} value={version}>
                  {version}
                </NativeSelectOption>
              ))}
            </NativeSelect>
          </label>
        </div>
        <Button
          size="sm"
          variant={policy.automaticUpdates ? "secondary" : "outline"}
          disabled={busy}
          onClick={() => void save({ automaticUpdates: !policy.automaticUpdates })}
        >
          {policy.automaticUpdates
            ? t("pluginStore.automaticUpdatesOn")
            : t("pluginStore.automaticUpdatesOff")}
        </Button>
      </CardContent>
    </Card>
  );
}
