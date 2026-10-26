"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { Teams } from "@/features/teams";
import type { Route } from "@/lib/types";
export default function Page() {
  const shell = useAppShellContext();
  const route = shell.currentRoute as Extract<Route, { name: "teams" }>;
  return <Teams tab={route.tab} />;
}
