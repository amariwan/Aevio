"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { MyTimesheet } from "@/features/timesheets/MyTimesheet";
import type { Route } from "@/lib/types";
export default function Page() {
  const shell = useAppShellContext();
  const route = shell.currentRoute as Extract<Route, { name: "timesheet" }>;
  return <MyTimesheet route={route} navigate={shell.navigate} replace={shell.replace} />;
}
