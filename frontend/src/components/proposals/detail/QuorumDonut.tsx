"use client";

import React, { useMemo } from "react";
import { useSpring, animated, to } from "@react-spring/web";
import { formatSOL } from "@/lib/governance/formatters";
import { formatBigintPercentage } from "@/helpers/bigint";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

const defaultColors = {
  for: { start: "#c8e6da", end: "#11C67D" },
  against: { start: "#fcb3c1", end: "#F43F5F" },
  abstain: { start: "#000000", end: "#9ca3af" },
};

interface QuorumDonutProps {
  forLamports: bigint | number;
  againstLamports: bigint | number;
  abstainLamports: bigint | number;
  /** `undefined` when total network stake could not be loaded. */
  totalLamports: bigint | number | undefined;
  quorumPercentage?: number;
}

/** Shown in the donut centre when total network stake could not be loaded. */
const UNKNOWN_TOTAL = "—";

export function QuorumDonutSkeleton() {
  return (
    <div className="relative w-full max-w-[220px] sm:max-w-[240px]">
      <svg viewBox="0 0 120 120" className="w-full h-full">
        <circle
          cx="60"
          cy="60"
          r="50"
          fill="none"
          stroke="#374151"
          strokeWidth="7"
          className="animate-pulse"
        />
      </svg>
      <div className="absolute inset-0 mt-3 sm:mt-2 flex flex-col items-center justify-center text-center">
        <div className="h-7 w-20 bg-white/10 animate-pulse rounded" />
        <div className="h-5 w-16 bg-white/10 animate-pulse rounded mt-1" />
      </div>
    </div>
  );
}

export default function QuorumDonut({
  forLamports,
  againstLamports,
  abstainLamports,
  totalLamports,
  quorumPercentage,
}: QuorumDonutProps) {
  const colors = defaultColors;

  // Memoize calculations to avoid re-computing on every render
  const { chartData, totalSol } = useMemo(() => {
    // Unknown must not render as "0.00 Total SOL" — that reads as a real
    // figure, and would contradict the "—" the breakdown beside it shows.
    const totalSol =
      totalLamports === undefined ? UNKNOWN_TOTAL : formatSOL(totalLamports);

    const chartData = [
      { name: "For", value: forLamports },
      { name: "Against", value: againstLamports },
      { name: "Abstain", value: abstainLamports },
    ];

    return { chartData, totalLamports, totalSol };
  }, [totalLamports, forLamports, againstLamports, abstainLamports]);

  // Calculate the rotation for the quorum marker in degrees
  const quorumAngleDegrees = quorumPercentage ? quorumPercentage * 360 : 0;

  // SVG Donut Chart calculations
  const radius = 50;
  const strokeWidth = 7;
  const circumference = 2 * Math.PI * radius;

  // No arcs when the denominator is unknown: an empty ring is the honest
  // rendering, and it matches the "—" in the centre.
  const hasTotal = totalLamports !== undefined && totalLamports > 0;
  // SVG needs bounded floating-point ratios; derive them from a decimal string,
  // never by coercing a lamport bigint itself.
  const ratio = (value: bigint | number) => {
    if (!hasTotal || totalLamports === undefined || typeof value !== typeof totalLamports) return 0;
    return typeof value === "bigint" && typeof totalLamports === "bigint"
      ? Number(formatBigintPercentage(value, totalLamports, 6)) / 100
      : (value as number) / (totalLamports as number);
  };
  const forPercent = ratio(chartData[0].value);
  const againstPercent = ratio(chartData[1].value);
  const abstainPercent = ratio(chartData[2].value);

  // Animation spring for the percentages
  const { forP, againstP, abstainP } = useSpring({
    to: {
      forP: forPercent,
      againstP: againstPercent,
      abstainP: abstainPercent,
    },
    from: {
      forP: 0,
      againstP: 0,
      abstainP: 0,
    },
    config: { mass: 1, tension: 170, friction: 26 },
  });

  // Animated lengths
  const forLength = to([forP], (p) => p * circumference);
  const againstLength = to([againstP], (p) => p * circumference);
  const abstainLength = to([abstainP], (p) => p * circumference);

  // Animated offsets
  const againstOffset = to([forP], (fp) => -fp * circumference);
  const abstainOffset = to(
    [forP, againstP],
    (fp, ap) => -(fp + ap) * circumference
  );

  // Static angles for gradients
  const forAngle = forPercent * 360;
  const againstAngle = againstPercent * 360;

  // Rotation for each gradient to align with its arc
  const forGradientRotate = forAngle / 2;
  const againstGradientRotate = forAngle + againstAngle / 2;
  const abstainGradientRotate =
    forAngle + againstAngle + (abstainPercent * 360) / 2;

  // Calculate marker position using trigonometry so it's independent of segment rotation
  const quorumAngleForCalc = quorumAngleDegrees - 90;
  const quorumAngleRadians = quorumAngleForCalc * (Math.PI / 180);
  const markerX = 60 + radius * Math.cos(quorumAngleRadians);
  const markerY = 60 + radius * Math.sin(quorumAngleRadians);

  return (
    <div className="relative w-full max-w-[220px] sm:max-w-[240px]">
      <svg viewBox="0 0 120 120" className="w-full h-full">
        <defs>
          <linearGradient
            id="forGradient"
            gradientUnits="userSpaceOnUse"
            x1="60"
            y1="10"
            x2="60"
            y2="110"
            gradientTransform={`rotate(${forGradientRotate} 60 60)`}
          >
            <stop offset="0%" stopColor={colors.for.start} />
            <stop offset="100%" stopColor={colors.for.end} />
          </linearGradient>
          <linearGradient
            id="againstGradient"
            gradientUnits="userSpaceOnUse"
            x1="60"
            y1="10"
            x2="60"
            y2="110"
            gradientTransform={`rotate(${againstGradientRotate} 60 60)`}
          >
            <stop offset="0%" stopColor={colors.against.start} />
            <stop offset="100%" stopColor={colors.against.end} />
          </linearGradient>
          <linearGradient
            id="abstainGradient"
            gradientUnits="userSpaceOnUse"
            x1="60"
            y1="10"
            x2="60"
            y2="110"
            gradientTransform={`rotate(${abstainGradientRotate} 60 60)`}
          >
            <stop offset="0%" stopColor={colors.abstain.start} />
            <stop offset="100%" stopColor={colors.abstain.end} />
          </linearGradient>
        </defs>
        {/* Chart Background */}
        <circle
          cx="60"
          cy="60"
          r={radius}
          fill="none"
          stroke="#374151"
          strokeWidth={strokeWidth}
        ></circle>

        <g transform="rotate(-90 60 60)">
          {/* Draw three separate arcs with smooth react-spring animation */}
          {/* Abstain Arc */}
          <animated.circle
            cx="60"
            cy="60"
            r={radius}
            fill="none"
            stroke="url(#abstainGradient)"
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeDasharray={to(
              [abstainLength],
              (l) => `${l} ${circumference}`
            )}
            style={{ strokeDashoffset: abstainOffset }}
          />
          {/* Against Arc */}
          <animated.circle
            cx="60"
            cy="60"
            r={radius}
            fill="none"
            stroke="url(#againstGradient)"
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeDasharray={to(
              [againstLength],
              (l) => `${l} ${circumference}`
            )}
            style={{ strokeDashoffset: againstOffset }}
          />
          {/* For Arc */}
          <animated.circle
            cx="60"
            cy="60"
            r={radius}
            fill="none"
            stroke="url(#forGradient)"
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeDasharray={to([forLength], (l) => `${l} ${circumference}`)}
          />
        </g>

        {/* Quorum Marker - Placed with calculated coordinates */}
        <Tooltip>
          <TooltipTrigger asChild>
            <circle
              cx={markerX}
              cy={markerY}
              r={strokeWidth / 2}
              fill="white"
              stroke="#141D2D"
              strokeWidth="1.5"
              className="cursor-pointer"
            />
          </TooltipTrigger>
          <TooltipContent
            side="top"
            className="rounded-lg bg-background/50 px-3 py-2 backdrop-blur-xs"
            sideOffset={8}
          >
            <p className="text-xs font-medium text-foreground">
              Quorum:{" "}
              {(quorumPercentage ? quorumPercentage * 100 : 0).toFixed(0)}%
            </p>
          </TooltipContent>
        </Tooltip>
      </svg>

      {/* Center Text */}
      <div className="absolute inset-0 mt-3 sm:mt-2 flex flex-col items-center justify-center text-center pointer-events-none">
        <span className="text-lg md:text-lg font-bold text-foreground leading-tight">
          {totalSol}
        </span>
        <span className="text-sm text-gray-400 -mt-0.5 md:mt-0">Total SOL</span>
      </div>
    </div>
  );
}
