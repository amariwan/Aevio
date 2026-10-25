import { LayoutGrid, Settings2, ShieldCheck } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useI18n } from "@/hooks/useI18n";
import * as api from "@/lib/api";
import type {
  PluginCatalogEntry,
  PluginUpdateCandidate,
  PluginWorkspaceDesiredState,
  Route,
} from "@/lib/types";
import { PluginStoreDetails } from "./PluginStoreDetails";
import { PluginGlobalSearch } from "./PluginGlobalSearch";
import { PluginLayoutControls } from "./PluginLayoutControls";
import { PluginHealthPanel } from "./PluginHealthPanel";
import { PluginRegistryConnection } from "./PluginRegistryConnection";
import { PluginStoreCatalog } from "./PluginStorePanels";
import { PluginUpdatesPanel } from "./PluginUpdatesPanel";
import { PluginWorkspaceStatePanel } from "./PluginWorkspaceStatePanel";

async function applyAutomaticUpdates(
  candidates: PluginUpdateCandidate[],
  entries: PluginCatalogEntry[],
  registry: { url: string; token: string },
): Promise<string[]> {
  const failures: string[] = [];
  for (const candidate of candidates.filter(
    (item) => item.automaticUpdates && !item.requiresPermissionApproval,
  )) {
    const installed = entries.find((entry) => entry.manifest.id === candidate.pluginId)?.installed;
    const wasActive = installed?.state === "active";
    try {
      if (wasActive) await api.pluginSetEnabled(candidate.pluginId, false);
      await api.pluginInstallRemote({
        registryUrl: registry.url,
        bearerToken: registry.token,
        pluginId: candidate.pluginId,
        release: candidate.release,
        enable: wasActive,
      });
    } catch (cause: unknown) {
      if (wasActive) {
        try {
          await api.pluginSetEnabled(candidate.pluginId, true);
        } catch {
          // Keep the original update error; the health panel exposes the state.
        }
      }
      failures.push(`${candidate.pluginId}: ${api.describeError(cause)}`);
    }
  }
  return failures;
}

export function PluginStore({ navigate }: { navigate: (route: Route) => void }) {
  const { t } = useI18n();
  const [entries, setEntries] = useState<PluginCatalogEntry[]>([]);
  const [selectedId, setSelectedId] = useState<string>();
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("all");
  const [busyId, setBusyId] = useState<string>();
  const [error, setError] = useState<string>();
  const [registryUrl, setRegistryUrl] = useState("");
  const [bearerToken, setBearerToken] = useState("");
  const [workspaceId, setWorkspaceId] = useState("");
  const [registryConnected, setRegistryConnected] = useState(false);
  const [registryBusy, setRegistryBusy] = useState(false);
  const [updates, setUpdates] = useState<PluginUpdateCandidate[]>([]);
  const [workspaceStates, setWorkspaceStates] = useState<PluginWorkspaceDesiredState[]>([]);

  const load = useCallback(async () => {
    try {
      setError(undefined);
      const next = await api.pluginCatalog();
      setEntries(next);
      setSelectedId((current) => current ?? next[0]?.manifest.id);
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    }
  }, []);
  useEffect(() => void load(), [load]);
  const loadRemote = async (url: string, token: string): Promise<PluginCatalogEntry[]> => {
    const [remote, local] = await Promise.all([
      api.pluginRegistryCatalog(url, token),
      api.pluginCatalog(),
    ]);
    const installed = new Map(local.map((entry) => [entry.manifest.id, entry.installed]));
    setEntries(remote.map((entry) => ({ ...entry, installed: installed.get(entry.manifest.id) })));
    setSelectedId((current) => current ?? remote[0]?.manifest.id);
    return remote.map((entry) => ({ ...entry, installed: installed.get(entry.manifest.id) }));
  };
  const connectRegistry = async () => {
    try {
      setRegistryBusy(true);
      setError(undefined);
      const remoteEntries = await loadRemote(registryUrl.trim(), bearerToken);
      const nextUpdates = await api.pluginRegistryUpdates(registryUrl.trim(), bearerToken);
      const failures = await applyAutomaticUpdates(nextUpdates, remoteEntries, {
        url: registryUrl.trim(),
        token: bearerToken,
      });
      const refreshedEntries = nextUpdates.some(
        (candidate) => candidate.automaticUpdates && !candidate.requiresPermissionApproval,
      )
        ? await loadRemote(registryUrl.trim(), bearerToken)
        : remoteEntries;
      setUpdates(
        refreshedEntries === remoteEntries
          ? nextUpdates
          : await api.pluginRegistryUpdates(registryUrl.trim(), bearerToken),
      );
      if (failures.length > 0) setError(`Automatic plugin updates failed: ${failures.join("; ")}`);
      setWorkspaceStates(
        workspaceId.trim()
          ? await api.pluginWorkspacePluginStates({
              registryUrl: registryUrl.trim(),
              bearerToken,
              workspaceId: workspaceId.trim(),
            })
          : [],
      );
      setRegistryConnected(true);
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    } finally {
      setRegistryBusy(false);
    }
  };
  const disconnectRegistry = async () => {
    setRegistryConnected(false);
    setBearerToken("");
    setUpdates([]);
    setWorkspaceStates([]);
    await load();
  };

  const categories = useMemo(
    () => ["all", ...new Set(entries.map((entry) => entry.manifest.category))],
    [entries],
  );
  const visible = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return entries.filter((entry) => {
      const matchesCategory = category === "all" || entry.manifest.category === category;
      const matchesQuery =
        normalized.length === 0 ||
        `${entry.manifest.name} ${entry.manifest.description}`.toLowerCase().includes(normalized);
      return matchesCategory && matchesQuery;
    });
  }, [category, entries, query]);
  const selected = entries.find((entry) => entry.manifest.id === selectedId) ?? visible[0];
  const run = async (pluginId: string, action: () => Promise<unknown>) => {
    try {
      setBusyId(pluginId);
      setError(undefined);
      await action();
      if (registryConnected) {
        await loadRemote(registryUrl.trim(), bearerToken);
        setUpdates(await api.pluginRegistryUpdates(registryUrl.trim(), bearerToken));
        if (workspaceId.trim()) {
          setWorkspaceStates(
            await api.pluginWorkspacePluginStates({
              registryUrl: registryUrl.trim(),
              bearerToken,
              workspaceId: workspaceId.trim(),
            }),
          );
        }
      } else await load();
    } catch (cause: unknown) {
      setError(api.describeError(cause));
    } finally {
      setBusyId(undefined);
    }
  };
  return (
    <main className="flex min-h-0 flex-1 flex-col overflow-y-auto bg-canvas p-5 lg:p-8">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-5">
        <header className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <p className="text-xs font-semibold tracking-[0.16em] text-fg-faint uppercase">Aevio</p>
            <h1 className="mt-1 text-2xl font-semibold text-fg-strong">Plugin Store</h1>
            <p className="mt-1 max-w-2xl text-sm text-fg-muted">{t("pluginStore.subtitle")}</p>
          </div>
          <div className="flex items-center gap-2 text-xs text-fg-muted">
            <ShieldCheck className="size-4 text-success" /> First-party, host-mediated modules
          </div>
        </header>
        {error && (
          <div className="rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-sm text-danger">
            {error}
          </div>
        )}
        <Tabs defaultValue="discover" className="min-h-0">
          <TabsList variant="line" className="w-full justify-start border-b border-line">
            <TabsTrigger value="discover">
              <LayoutGrid className="size-4" /> Discover
              <span className="text-xs text-fg-faint">{visible.length}</span>
            </TabsTrigger>
            <TabsTrigger value="manage">
              <Settings2 className="size-4" /> Manage
              {registryConnected && <span className="size-1.5 rounded-full bg-success" />}
            </TabsTrigger>
          </TabsList>

          <TabsContent value="discover" className="pt-5">
            <div className="grid min-h-0 gap-5 lg:grid-cols-[minmax(0,1fr)_22rem]">
              <PluginStoreCatalog
                entries={visible}
                categories={categories}
                category={category}
                query={query}
                selectedId={selected?.manifest.id}
                onCategoryChange={setCategory}
                onQueryChange={setQuery}
                onSelect={setSelectedId}
              />
              {selected && (
                <PluginStoreDetails
                  selected={selected}
                  busyId={busyId}
                  run={run}
                  registry={
                    registryConnected ? { url: registryUrl.trim(), token: bearerToken } : undefined
                  }
                />
              )}
            </div>
          </TabsContent>

          <TabsContent value="manage" className="space-y-4 pt-5">
            <PluginRegistryConnection
              registryUrl={registryUrl}
              bearerToken={bearerToken}
              connected={registryConnected}
              busy={registryBusy}
              onRegistryUrlChange={setRegistryUrl}
              onBearerTokenChange={setBearerToken}
              workspaceId={workspaceId}
              onWorkspaceIdChange={setWorkspaceId}
              onConnect={() => void connectRegistry()}
              onDisconnect={() => void disconnectRegistry()}
            />
            {registryConnected && workspaceId.trim() && (
              <PluginWorkspaceStatePanel
                workspaceId={workspaceId.trim()}
                registry={{ url: registryUrl.trim(), token: bearerToken }}
                entries={entries}
                states={workspaceStates}
                onSaved={(state) =>
                  setWorkspaceStates((current) => [
                    ...current.filter((item) => item.pluginId !== state.pluginId),
                    state,
                  ])
                }
              />
            )}
            <div className="grid gap-4 lg:grid-cols-2">
              <PluginGlobalSearch onNavigate={navigate} />
              <PluginHealthPanel />
              <PluginLayoutControls entries={entries} />
              {registryConnected && (
                <PluginUpdatesPanel
                  candidates={updates}
                  entries={entries}
                  registry={{ url: registryUrl.trim(), token: bearerToken }}
                  busyId={busyId}
                  run={run}
                />
              )}
            </div>
          </TabsContent>
        </Tabs>
      </div>
    </main>
  );
}
