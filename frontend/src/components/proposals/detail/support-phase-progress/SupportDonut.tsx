"use client";

import { useMemo } from "react";
import { useSpring, animated, to } from "@react-spring/web";
import { CircleCheck } from "lucide-react";
import { formatSOL } from "@/lib/governance/formatters";
import { DONUT_CONFIG, DonutChartBase } from "../shared/DonutChartBase";

/**
 * Presentational only. These figures come from `computeSupportStats` so the
 * donut and its parent cannot disagree — the donut used to recompute
 * `isThresholdMet` itself and reported success while stake was still loading.
 */
interface SupportDonutProps {
  progressPercent: number;
  /**
   * Incorporates the on-chain crossing verdict (via `computeSupportStats`),
   * not just live math. When set, the percentage is not shown at all: it is
   * an estimate from live stake, which differs from the epoch-stakes total
   * the program measured against, so it can read "99.9%" for a proposal the
   * chain already advanced.
   */
  isThresholdMet: boolean;
  remainingLamports: bigint;
}

export function SupportDonut({
  progressPercent,
  isThresholdMet,
  remainingLamports,
}: SupportDonutProps) {
  const remainingSol = useMemo(
    () => formatSOL(remainingLamports),
    [remainingLamports]
  );

  // Clamp display to 100% for the arc (even if progress > 100%)
  const displayPercent = Math.min(progressPercent, 100) / 100;

  // Animation spring
  const { progress } = useSpring({
    progress: displayPercent,
    from: { progress: 0 },
    config: { mass: 1, tension: 170, friction: 26 },
  });

  const arcLength = to([progress], (p) => p * DONUT_CONFIG.circumference);

  const gradients = (
    <linearGradient
      id="supportGradient"
      gradientUnits="userSpaceOnUse"
      x1="60"
      y1="10"
      x2="60"
      y2="110"
    >
      <stop offset="0%" stopColor="#004CC7" />
      <stop offset="100%" stopColor="#11C67D" />
    </linearGradient>
  );

  const centerContent = isThresholdMet ? (
    <>
      <CircleCheck className="size-8 text-emerald-400" aria-hidden="true" />
      <span className="mt-2 text-sm font-medium text-emerald-400">
        Threshold reached
      </span>
    </>
  ) : (
    <>
      <span className="text-2xl font-semibold text-foreground md:text-3xl">
        {progressPercent.toFixed(1)}%
      </span>
      <span className="text-sm text-gray-400">reached</span>
      <span className="mt-1 text-xs font-medium text-primary">
        {remainingSol} SOL needed
      </span>
    </>
  );

  return (
    <DonutChartBase gradients={gradients} centerContent={centerContent}>
      <animated.circle
        cx={DONUT_CONFIG.center.x}
        cy={DONUT_CONFIG.center.y}
        r={DONUT_CONFIG.radius}
        fill="none"
        stroke="url(#supportGradient)"
        strokeWidth={DONUT_CONFIG.strokeWidth}
        strokeLinecap="round"
        strokeDasharray={to(
          [arcLength],
          (l) => `${l} ${DONUT_CONFIG.circumference}`
        )}
      />
    </DonutChartBase>
  );
}
