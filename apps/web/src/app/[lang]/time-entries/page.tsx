"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { TimeEntries } from "@/features/entries/TimeEntries";
import type { Route } from "@/lib/types";
export default function Page() {
  const shell = useAppShellContext();
  const route = shell.currentRoute as Extract<Route, { name: "entries" }>;
  return <TimeEntries route={route} navigate={shell.navigate} replace={shell.replace} />;
}
