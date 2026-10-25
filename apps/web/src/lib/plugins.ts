import type { PluginCatalogEntry, PluginContributionSnapshot, Route } from "./types";

export interface PluginNavigationEntry {
  id: string;
  label: string;
  pluginName: string;
  route: Route;
}

const PLUGIN_ROUTE_NAMES = new Set<Route["name"]>([
  "clients",
  "notes",
  "projects",
  "tasks",
  "timers",
]);

export function isPluginRoute(route: Route): boolean {
  return PLUGIN_ROUTE_NAMES.has(route.name);
}

/** Only host-owned routes are executable in the privileged React window. */
export function hostRouteForPluginPage(page: string): Route | undefined {
  const route = page.split("/").at(-1);
  const routes: Record<string, Route> = {
    clients: { name: "clients" },
    invoices: { name: "invoices" },
    notes: { name: "notes" },
    projects: { name: "projects" },
    tasks: { name: "tasks" },
    timers: { name: "timers" },
  };
  return route ? routes[route] : undefined;
}

export function activePluginNavigation(entries: PluginCatalogEntry[]): PluginNavigationEntry[] {
  return entries.flatMap((entry) => {
    if (entry.installed?.state !== "active") return [];
    return entry.manifest.contributes.navigation.flatMap((navigation) => {
      const route = hostRouteForPluginPage(navigation.page);
      return route
        ? [{ id: navigation.id, label: navigation.label, pluginName: entry.manifest.name, route }]
        : [];
    });
  });
}

export function activeContributionNavigation(
  entries: PluginContributionSnapshot[],
): PluginNavigationEntry[] {
  return entries.flatMap((entry) =>
    entry.contributions.navigation.flatMap((navigation) => {
      const route = hostRouteForPluginPage(navigation.page);
      return route
        ? [{ id: navigation.id, label: navigation.label, pluginName: entry.pluginName, route }]
        : [];
    }),
  );
}
