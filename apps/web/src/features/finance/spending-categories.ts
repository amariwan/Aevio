import type { SpendingCategory } from "@/components/kokonutui/spending-ring";
import type { FinancePivotRow } from "@/lib/types";

const COLORS = [
  ["stroke-chart-1", "bg-chart-1"],
  ["stroke-chart-2", "bg-chart-2"],
  ["stroke-chart-3", "bg-chart-3"],
  ["stroke-chart-4", "bg-chart-4"],
  ["stroke-chart-5", "bg-chart-5"],
] as const;

const category = (name: string, amount: number, index: number): SpendingCategory => {
  const [stroke, dot] = COLORS[index % COLORS.length];
  return { name, amount, stroke, dot };
};

export function spendingCategories(rows: FinancePivotRow[]): SpendingCategory[] {
  const ranked = rows
    .filter((row) => row.costCents > 0)
    .sort((left, right) => right.costCents - left.costCents);
  const visible = ranked.slice(0, COLORS.length);
  const otherCents = ranked.slice(COLORS.length).reduce((total, row) => total + row.costCents, 0);

  return [
    ...visible.map((row, index) => category(row.projectName, row.costCents / 100, index)),
    ...(otherCents > 0 ? [category("Weitere", otherCents / 100, visible.length)] : []),
  ];
}
