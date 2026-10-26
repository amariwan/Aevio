"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { Agent } from "@/features/agent";
export default function Page() {
  const shell = useAppShellContext();
  return <Agent navigate={shell.navigate} />;
}
