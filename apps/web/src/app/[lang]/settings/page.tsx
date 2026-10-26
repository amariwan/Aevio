"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { Settings } from "@/features/settings/Settings";
import type { Route } from "@/lib/types";
export default function Page() {
  const shell = useAppShellContext();
  const route = shell.currentRoute as Extract<Route, { name: "settings" }>;
  return (
    <Settings
      route={route}
      updates={shell.updates}
      revealUpdates={shell.revealUpdates}
      navigate={shell.navigate}
    />
  );
}
