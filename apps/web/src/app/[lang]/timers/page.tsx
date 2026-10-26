"use client";
import { useAppShellContext } from "@/shell/AppContext";
import { Timers } from "@/features/timers/Timers";
export default function Page() {
  const shell = useAppShellContext();
  return <Timers revealTimer={shell.revealTimer} />;
}
