import { useEffect, useRef, useState } from "react";
import { motion, useReducedMotion, useSpring, useTransform } from "motion/react";

/**
 * Animated counter that springs from the previous value to the new one.
 * Respects prefers-reduced-motion (snaps instantly). Use for KPI tiles,
 * dashboard headers and anywhere a number changes over time.
 */
export function AnimatedCounter({
  value,
  format = (n) => Math.round(n).toLocaleString(),
  className,
  duration = 0.6,
}: {
  value: number;
  format?: (n: number) => string;
  className?: string;
  duration?: number;
}) {
  const reduced = useReducedMotion();
  const spring = useSpring(value, {
    stiffness: 120,
    damping: 20,
    mass: 0.6,
    duration: reduced ? 0 : duration,
  });
  const display = useTransform(spring, (n) => format(n));
  const [text, setText] = useState(format(value));
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    const unsubscribe = display.on("change", (v) => {
      setText(v);
    });
    spring.set(value);
    return () => {
      unsubscribe();
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    };
  }, [value, spring, display]);

  return (
    <motion.span className={className} aria-live="polite">
      {text}
    </motion.span>
  );
}
