import type { Transition, Variants } from "motion/react";

/** Shared motion language for the desktop shell and feature surfaces. */
export const motionEase = [0.22, 1, 0.36, 1] as const;

export const motionTransition: Transition = {
  duration: 0.28,
  ease: motionEase,
};

export const motionSpring: Transition = {
  type: "spring",
  stiffness: 400,
  damping: 28,
};

export const pageVariants: Variants = {
  initial: { opacity: 0, y: 12 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -8 },
};

export const surfaceVariants: Variants = {
  initial: { opacity: 0, scale: 0.98, y: 8 },
  animate: { opacity: 1, scale: 1, y: 0 },
  exit: { opacity: 0, scale: 0.985, y: 4 },
};

export const staggerVariants: Variants = {
  initial: {},
  animate: { transition: { staggerChildren: 0.05 } },
};

export const staggerItemVariants: Variants = {
  initial: { opacity: 0, y: 8 },
  animate: { opacity: 1, y: 0 },
};
