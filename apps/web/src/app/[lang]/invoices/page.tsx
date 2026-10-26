"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { Invoices } from "@/features/invoices/Invoices";
import type { Route } from "@/lib/types";
export default function Page() {
  const shell = useAppShellContext();
  const route = shell.currentRoute as Extract<Route, { name: "invoices" }>;
  return <Invoices route={route} navigate={shell.navigate} replace={shell.replace} />;
}
