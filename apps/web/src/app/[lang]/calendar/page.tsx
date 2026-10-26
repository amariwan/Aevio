"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { Calendar } from "@/features/calendar/Calendar";
import type { Route } from "@/lib/types";
export default function Page() {
  const shell = useAppShellContext();
  const route = shell.currentRoute as Extract<Route, { name: "calendar" }>;
  return <Calendar key={shell.calendarMount} route={route} navigate={shell.navigate} />;
}
