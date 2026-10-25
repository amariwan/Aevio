import { useCallback, useEffect, useState } from "react";
import { useTauriEvent } from "./useTauriEvent";
import * as api from "@/lib/api";
import { activeContributionNavigation, type PluginNavigationEntry } from "@/lib/plugins";

export interface PluginExtensionsState {
  navigation: PluginNavigationEntry[];
  ready: boolean;
}

export function usePluginExtensions(): PluginExtensionsState {
  const [navigation, setNavigation] = useState<PluginNavigationEntry[]>([]);
  const [ready, setReady] = useState(false);
  const load = useCallback(async () => {
    try {
      setNavigation(activeContributionNavigation(await api.pluginContributions()));
    } catch {
      setNavigation([]);
    } finally {
      setReady(true);
    }
  }, []);
  useEffect(() => void load(), [load]);
  useTauriEvent(api.PLUGINS_CHANGED, load);
  return { navigation, ready };
}
