/**
 * Central chart design tokens. Single source of truth for spacing, radii,
 * stroke widths, and palette slots used across every chart in Aevio. Tokens
 * are CSS-variable backed so they follow the active theme and dark/light mode
 * automatically.
 */
export const CHART_TOKENS = {
  // Spacing
  gap: 2,
  padding: { x: 8, y: 8 },
  // Radii
  radius: { sm: 4, md: 6, lg: 8, pill: 9999 },
  // Stroke widths
  stroke: { thin: 1, base: 1.5, thick: 2, heavy: 3 },
  // Donut
  donut: { stroke: 14, gap: 2, corner: 2 },
  // Bar
  bar: { radius: 4, gap: 2, min: 2 },
  // Axis
  axis: { tickSize: 10, fontSize: 10 },
  // Animation
  motion: {
    spring: { stiffness: 260, damping: 24, mass: 0.8 },
    springSoft: { stiffness: 180, damping: 22, mass: 1 },
    ease: [0.22, 1, 0.36, 1] as const,
    duration: { fast: 0.2, base: 0.4, slow: 0.7 },
  },
} as const;

/** Resolve a CSS variable color reference. */
export function cssVar(name: string): string {
  return `var(--${name})`;
}

/** Standard chart palette slots (theme-aware). */
export const CHART_PALETTE = [
  "var(--chart-1)",
  "var(--chart-2)",
  "var(--chart-3)",
  "var(--chart-4)",
  "var(--chart-5)",
] as const;

/** Reduced-motion aware: returns true if user prefers reduced motion. */
export function prefersReducedMotion(): boolean {
  if (typeof window === "undefined") return false;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}
