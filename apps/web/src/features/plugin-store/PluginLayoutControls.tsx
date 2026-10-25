import { Pin, RotateCcw, Eye, EyeOff } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import * as api from "@/lib/api";
import type { PluginCatalogEntry, PluginContributionPreference } from "@/lib/types";

type Props = { entries: PluginCatalogEntry[] };

export function PluginLayoutControls({ entries }: Props) {
  const [preferences, setPreferences] = useState<PluginContributionPreference[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const activeNavigation = useMemo(
    () =>
      entries.flatMap((entry) =>
        entry.installed?.state === "active"
          ? entry.manifest.contributes.navigation.map((item, index) => ({
              pluginName: entry.manifest.name,
              item,
              defaultOrder: index,
            }))
          : [],
      ),
    [entries],
  );

  const load = useCallback(async () => {
    try {
      setPreferences(await api.pluginLayout());
      setError(undefined);
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    }
  }, []);
  useEffect(() => void load(), [load]);

  const preferenceFor = (pluginId: string, contributionId: string, orderIndex: number) =>
    preferences.find(
      (preference) =>
        preference.pluginId === pluginId && preference.contributionId === contributionId,
    ) ?? {
      pluginId,
      contributionId,
      visible: true,
      pinned: false,
      orderIndex,
    };

  const update = async (preference: PluginContributionPreference) => {
    try {
      setBusy(true);
      setError(undefined);
      const saved = await api.pluginSetContributionPreference(preference);
      setPreferences((current) => [
        ...current.filter(
          (item) =>
            item.pluginId !== saved.pluginId || item.contributionId !== saved.contributionId,
        ),
        saved,
      ]);
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };

  const restore = async () => {
    try {
      setBusy(true);
      await api.pluginRestoreLayout();
      await load();
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };

  if (activeNavigation.length === 0) return null;
  return (
    <Card className="bg-surface">
      <CardHeader className="flex-row items-center justify-between space-y-0">
        <CardTitle className="text-base">Workspace composition</CardTitle>
        <Button size="sm" variant="outline" disabled={busy} onClick={() => void restore()}>
          <RotateCcw className="size-3.5" /> Restore defaults
        </Button>
      </CardHeader>
      <CardContent className="space-y-2">
        {error && <p className="text-sm text-danger">{error}</p>}
        {activeNavigation.map(({ pluginName, item, defaultOrder }) => {
          const entry = entries.find((candidate) =>
            candidate.manifest.contributes.navigation.some(
              (navigation) => navigation.id === item.id,
            ),
          );
          if (!entry) return null;
          const preference = preferenceFor(entry.manifest.id, item.id, defaultOrder);
          return (
            <div
              key={item.id}
              className="flex items-center gap-2 rounded-lg border border-line px-3 py-2"
            >
              <span className="min-w-0 flex-1 truncate text-sm text-fg">
                {item.label} <span className="text-xs text-fg-muted">· {pluginName}</span>
              </span>
              <Button
                size="sm"
                variant={preference.pinned ? "secondary" : "ghost"}
                disabled={busy}
                aria-label={preference.pinned ? `Unpin ${item.label}` : `Pin ${item.label}`}
                onClick={() => void update({ ...preference, pinned: !preference.pinned })}
              >
                <Pin className="size-3.5" />
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={busy}
                aria-label={preference.visible ? `Hide ${item.label}` : `Show ${item.label}`}
                onClick={() => void update({ ...preference, visible: !preference.visible })}
              >
                {preference.visible ? (
                  <Eye className="size-3.5" />
                ) : (
                  <EyeOff className="size-3.5" />
                )}
              </Button>
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}
