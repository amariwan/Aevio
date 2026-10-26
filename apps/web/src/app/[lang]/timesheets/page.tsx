"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { Timesheets } from "@/features/timesheets/Timesheets";
import type { Route } from "@/lib/types";
export default function Page() {
  const shell = useAppShellContext();
  const route = shell.currentRoute as Extract<Route, { name: "timesheets" }>;
  return <Timesheets route={route} navigate={shell.navigate} replace={shell.replace} />;
}
