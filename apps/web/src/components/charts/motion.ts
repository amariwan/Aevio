import { CHART_TOKENS } from "./tokens";

/**
 * Shared animation presets for charts. Centralises spring physics, durations
 * and easing so every chart feels consistent. All presets respect
 * `prefers-reduced-motion` via the `useReducedMotion` hook from motion/react.
 */
export const CHART_MOTION = {
  /** Spring for value transitions (bars growing, donut arcs). */
  spring: CHART_TOKENS.motion.spring,
  /** Softer spring for hover/tooltip micro-interactions. */
  springSoft: CHART_TOKENS.motion.springSoft,
  /** Cubic-bezier ease for CSS transitions. */
  ease: CHART_TOKENS.motion.ease,
  /** Durations in seconds. */
  duration: CHART_TOKENS.motion.duration,
  /** Common variants. */
  variants: {
    fadeIn: {
      initial: { opacity: 0 },
      animate: { opacity: 1 },
      transition: { duration: CHART_TOKENS.motion.duration.base, ease: CHART_TOKENS.motion.ease },
    },
    slideUp: {
      initial: { opacity: 0, y: 8 },
      animate: { opacity: 1, y: 0 },
      transition: { duration: CHART_TOKENS.motion.duration.base, ease: CHART_TOKENS.motion.ease },
    },
    scaleIn: {
      initial: { opacity: 0, scale: 0.92 },
      animate: { opacity: 1, scale: 1 },
      transition: { type: "spring", ...CHART_TOKENS.motion.spring },
    },
  },
} as const;

/** Hook-friendly reduced-motion check (re-export for convenience). */
export function motionTransition(reduced: boolean, base: Record<string, unknown>) {
  if (reduced) return { duration: 0 };
  return base;
}
