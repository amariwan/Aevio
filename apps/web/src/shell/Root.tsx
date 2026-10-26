"use client";

import { getCurrentWindow } from "@tauri-apps/api/window";
import Pulse from "@/features/pulse/Pulse";
import Reminder from "@/features/pulse/Reminder";
import Widget from "@/features/widget/Widget";
import ProductivitySummary from "@/features/summary/ProductivitySummary";
import BreakScreen from "@/features/pulse/BreakScreen";
import App from "./App";

// One bundle, four windows: the menu-bar panel is `pulse`, the break panel is
// `reminder`, and the always-visible timer display is `widget`.
// Guard against non-Tauri contexts (e.g. browser dev / SSR) where
// getCurrentWindow() would throw.
let label = "main";
try {
  label = getCurrentWindow().label;
} catch {
  label = "main";
}
if (typeof document !== "undefined") {
  if (label === "pulse") document.documentElement.classList.add("pulse-window");
  if (label === "reminder") document.documentElement.classList.add("reminder-window");
  if (label === "widget") document.documentElement.classList.add("widget-window");
  if (label === "summary") document.documentElement.classList.add("summary-window");
}

export default function Root() {
  if (label === "pulse") return <Pulse />;
  if (label === "reminder") return <Reminder />;
  if (label === "widget") return <Widget />;
  if (label === "summary") return <ProductivitySummary />;
  if (label === "break-screen") return <BreakScreen />;
  return <App />;
}
