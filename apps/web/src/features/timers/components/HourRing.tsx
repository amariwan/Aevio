import { Tooltip } from "@/components/primitives/Tooltip";
import { motion, useReducedMotion } from "motion/react";

const SIZE = 96;
const STROKE = 7;
const RADIUS = (SIZE - STROKE) / 2;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
const CENTRE = SIZE / 2;

/** The dial's cycle: one full turn per tracked hour, then it wraps. */
const HOUR_MS = 3_600_000;

interface HourRingProps {
  /** This tracker's own elapsed time in ms — not the wall clock. */
  elapsed: number;
  running: boolean;
  /** Toggles start/pause — the ring's center doubles as that control. */
  onToggle: () => void;
}

/** Fraction of the current tracked hour that has passed, 0–1. Zero at 0:00,
 *  a quarter at 15:00, wrapping back to zero at 1:00:00. */
export function hourProgress(elapsed: number): number {
  return (elapsed % HOUR_MS) / HOUR_MS;
}

/** A dial that sweeps once per tracked hour and doubles as the play/pause
 *  button — its center shows a pause glyph while running, a play glyph
 *  while paused, so the icon always signals what tapping it will do. */
export function HourRing({ elapsed, running, onToggle }: HourRingProps) {
  const reduced = useReducedMotion();
  const progress = hourProgress(elapsed);
  const minutes = Math.floor((elapsed % HOUR_MS) / 60_000);
  const spring = reduced
    ? { duration: 0 }
    : { type: "spring" as const, stiffness: 180, damping: 22 };

  return (
    <Tooltip content={running ? "Pause" : "Start"}>
      <motion.button
        type="button"
        onClick={onToggle}
        aria-label={running ? "Pause tracking" : "Start tracking"}
        className="grid size-24 shrink-0 place-items-center rounded-full outline-none transition-transform hover:scale-[1.03] active:scale-95 focus-visible:ring-2 focus-visible:ring-accent/50"
        whileHover={reduced ? undefined : { scale: 1.03 }}
        whileTap={reduced ? undefined : { scale: 0.95 }}
      >
        <motion.svg
          viewBox={`0 0 ${SIZE} ${SIZE}`}
          className="size-24"
          role="img"
          aria-label={`${minutes} minutes into this tracker's current hour`}
          initial={reduced ? false : { opacity: 0, scale: 0.9 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={spring}
        >
          <circle
            cx={CENTRE}
            cy={CENTRE}
            r={RADIUS}
            fill="none"
            strokeWidth={STROKE}
            className="stroke-line"
          />
          <motion.circle
            cx={CENTRE}
            cy={CENTRE}
            r={RADIUS}
            fill="none"
            strokeWidth={STROKE}
            strokeLinecap="round"
            strokeDasharray={CIRCUMFERENCE}
            initial={reduced ? false : { strokeDashoffset: CIRCUMFERENCE }}
            animate={{ strokeDashoffset: CIRCUMFERENCE * (1 - progress) }}
            transition={spring}
            transform={`rotate(-90 ${CENTRE} ${CENTRE})`}
            className={running ? "stroke-accent" : "stroke-fg-faint"}
          />
          {running ? (
            <>
              <motion.rect
                x={CENTRE - 10}
                y={CENTRE - 14}
                width="6"
                height="28"
                rx="2"
                className="fill-accent"
                initial={reduced ? false : { opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={reduced ? { duration: 0 } : { duration: 0.2 }}
              />
              <motion.rect
                x={CENTRE + 4}
                y={CENTRE - 14}
                width="6"
                height="28"
                rx="2"
                className="fill-accent"
                initial={reduced ? false : { opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={reduced ? { duration: 0 } : { duration: 0.2, delay: 0.05 }}
              />
            </>
          ) : (
            <motion.path
              d={`M${CENTRE - 8},${CENTRE - 16} L${CENTRE - 8},${CENTRE + 16} L${CENTRE + 16},${CENTRE} Z`}
              className="fill-fg-muted"
              initial={reduced ? false : { opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={reduced ? { duration: 0 } : { duration: 0.2 }}
            />
          )}
        </motion.svg>
      </motion.button>
    </Tooltip>
  );
}
