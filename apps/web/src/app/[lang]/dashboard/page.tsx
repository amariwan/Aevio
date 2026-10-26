"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { Dashboard } from "@/features/dashboard";

export default function Page() {
  const shell = useAppShellContext();
  return <Dashboard navigate={shell.navigate} />;
}
