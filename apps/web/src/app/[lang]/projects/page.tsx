"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { Projects } from "@/features/projects/Projects";
import type { Route } from "@/lib/types";
export default function Page() {
  const shell = useAppShellContext();
  const route = shell.currentRoute as Extract<Route, { name: "projects" }>;
  return <Projects route={route} navigate={shell.navigate} replace={shell.replace} />;
}
