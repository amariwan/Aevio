"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { Clients } from "@/features/clients/Clients";
import type { Route } from "@/lib/types";
export default function Page() {
  const shell = useAppShellContext();
  const route = shell.currentRoute as Extract<Route, { name: "clients" }>;
  return <Clients route={route} navigate={shell.navigate} replace={shell.replace} />;
}
