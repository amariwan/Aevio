import type { CSSProperties } from "react";

import type { BlockState } from "@/lib/entries";

export type EntryBlockDensity = "full" | "compact" | "line" | "sliver";

export const DENSITY_CLASSES: Record<EntryBlockDensity, string> = {
  full: "rounded-lg p-2.5",
  compact: "rounded-lg px-2.5 py-1",
  line: "rounded-md px-2 py-0",
  sliver: "rounded-sm p-0",
};

export const NARROW_DENSITY_CLASSES: Record<EntryBlockDensity, string> = {
  full: "rounded-md px-1.5 py-1",
  compact: "rounded-md px-1.5 py-0.5",
  line: "rounded-sm px-1 py-0",
  sliver: "rounded-xs p-0",
};

const LINE_HEIGHTS = {
  title: 18,
  titleCompact: 17.25,
  time: 16.5,
  timeCompact: 15.75,
  chipText: 15,
  narrowTitle: 13.125,
  narrowSub: 12.5,
};
const CHIP_HEIGHT = LINE_HEIGHTS.chipText + 4;

const CHROME: Record<EntryBlockDensity, number> = {
  full: 22,
  compact: 10,
  line: 2,
  sliver: 0,
};
const NARROW_CHROME: Record<EntryBlockDensity, number> = {
  full: 10,
  compact: 6,
  line: 2,
  sliver: 0,
};

export interface LineBudget {
  title: number;
  chip: number;
  wrapRow: boolean;
}

interface BudgetInput {
  room: number;
  titleLine: number;
  base: number;
  chips: number;
  rowItems: number;
  rowLine: number;
  rowGap: number;
}

function budgetLines(input: BudgetInput): LineBudget {
  const budget: LineBudget = { title: 1, chip: 1, wrapRow: false };
  let spare = input.room - input.base;
  for (let turn = 0; turn < 32; turn++) {
    let grew = false;
    if (spare >= input.titleLine) {
      budget.title++;
      spare -= input.titleLine;
      grew = true;
    }
    if (input.chips > 0) {
      const cost =
        !budget.wrapRow && input.rowItems > 1
          ? (input.rowItems - 1) * (input.rowLine + input.rowGap)
          : input.chips * LINE_HEIGHTS.chipText;
      if (spare >= cost) {
        if (!budget.wrapRow && input.rowItems > 1) budget.wrapRow = true;
        else budget.chip++;
        spare -= cost;
        grew = true;
      }
    }
    if (!grew) break;
  }
  return budget;
}

export function clampStyle(lines: number): CSSProperties {
  return {
    display: "-webkit-box",
    WebkitBoxOrient: "vertical",
    WebkitLineClamp: lines,
    overflow: "hidden",
    textOverflow: "ellipsis",
  };
}

export function densityFor(height: number): EntryBlockDensity {
  if (height >= 72) return "full";
  if (height >= 26) return "compact";
  if (height >= 12) return "line";
  return "sliver";
}

export function wideBudget(
  density: EntryBlockDensity,
  height: number,
  chips: number,
  hasStatus: boolean,
): LineBudget {
  const room = height - CHROME[density];
  const chipRow = chips > 0 ? CHIP_HEIGHT : LINE_HEIGHTS.chipText;
  if (density === "full") {
    const rowItems = chips + (hasStatus ? 1 : 0);
    return budgetLines({
      room,
      titleLine: LINE_HEIGHTS.title,
      base: LINE_HEIGHTS.title + 2 + LINE_HEIGHTS.time + (rowItems > 0 ? 6 + chipRow : 0),
      chips,
      rowItems,
      rowLine: chipRow,
      rowGap: 6,
    });
  }
  if (density === "compact" && height >= 40) {
    return budgetLines({
      room,
      titleLine: LINE_HEIGHTS.titleCompact,
      base: LINE_HEIGHTS.titleCompact + 2 + Math.max(chipRow, LINE_HEIGHTS.timeCompact),
      chips,
      rowItems: 1 + chips + (hasStatus ? 1 : 0),
      rowLine: Math.max(chipRow, LINE_HEIGHTS.timeCompact),
      rowGap: 2,
    });
  }
  const titleLine = density === "compact" ? LINE_HEIGHTS.titleCompact : LINE_HEIGHTS.title;
  return budgetLines({
    room,
    titleLine,
    base: titleLine,
    chips: 0,
    rowItems: 0,
    rowLine: 0,
    rowGap: 0,
  });
}

export function narrowBudget(
  density: EntryBlockDensity,
  height: number,
): { first: number; second: number } {
  const lines = { first: 1, second: 1 };
  if (density === "line") return lines;
  let spare =
    height -
    NARROW_CHROME[density] -
    LINE_HEIGHTS.narrowTitle -
    (height >= 38 ? LINE_HEIGHTS.narrowSub : 0);
  for (let turn = 0; turn < 32; turn++) {
    let grew = false;
    if (spare >= LINE_HEIGHTS.narrowTitle) {
      lines.first++;
      spare -= LINE_HEIGHTS.narrowTitle;
      grew = true;
    }
    if (height >= 38 && spare >= LINE_HEIGHTS.narrowSub) {
      lines.second++;
      spare -= LINE_HEIGHTS.narrowSub;
      grew = true;
    }
    if (!grew) break;
  }
  return lines;
}

export function blockStyle(state: BlockState): CSSProperties {
  switch (state) {
    case "approved":
      return { backgroundColor: "color-mix(in srgb, var(--accent) 24%, var(--bg-panel))" };
    case "pending":
      return {
        backgroundImage:
          "repeating-linear-gradient(135deg, color-mix(in srgb, var(--accent) 14%, var(--bg-panel)) 0 6px, color-mix(in srgb, var(--accent) 7%, var(--bg-panel)) 6px 12px)",
      };
    case "needsYou":
    case "failed":
      return {
        backgroundImage:
          "repeating-linear-gradient(135deg, var(--bg-surface-1) 0 6px, transparent 6px 12px)",
      };
    default:
      return {};
  }
}

export const BLOCK_CLASSES: Record<BlockState, string> = {
  approved: "border-line shadow-xs",
  pending: "border-dashed border-line-strong",
  needsYou: "border-dashed border-line",
  failed: "border-dashed border-danger/50",
  processing: "entry-processing border-line",
  building: "border-dotted border-fg-soft/50 bg-transparent",
};
