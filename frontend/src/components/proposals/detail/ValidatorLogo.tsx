"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";

function ValidatorLogoSkeleton() {
  return (
    <div
      className="size-8 shrink-0 rounded-full bg-white/10 animate-pulse animate-pulse-glow ring-1 ring-white/10"
      aria-hidden
    />
  );
}

interface ValidatorLetterAvatarProps {
  validatorName: string;
  accentColor: string;
}

function ValidatorLetterAvatar({
  validatorName,
  accentColor,
}: ValidatorLetterAvatarProps) {
  return (
    <div
      className="flex size-8 shrink-0 items-center justify-center rounded-full text-[14px] font-semibold uppercase text-white shadow-lg"
      style={{ background: accentColor }}
      aria-hidden
    >
      {validatorName.slice(0, 1)}
    </div>
  );
}

interface ValidatorLogoImageProps {
  validatorImage: string;
  validatorName: string;
  accentColor: string;
}

function ValidatorLogoImage({
  validatorImage,
  validatorName,
  accentColor,
}: ValidatorLogoImageProps) {
  const [failed, setFailed] = useState(false);
  const [loaded, setLoaded] = useState(false);

  if (failed) {
    return (
      <ValidatorLetterAvatar
        validatorName={validatorName}
        accentColor={accentColor}
      />
    );
  }

  return (
    <div className="relative size-8 shrink-0">
      {!loaded && <ValidatorLogoSkeleton />}
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img
        src={validatorImage}
        alt={validatorName}
        className={cn(
          "size-8 rounded-full object-cover ring-1 ring-white/10 shadow-lg",
          !loaded && "absolute inset-0 opacity-0",
        )}
        onLoad={() => setLoaded(true)}
        onError={() => setFailed(true)}
      />
    </div>
  );
}

interface ValidatorLogoProps {
  validatorName: string;
  validatorImage?: string | null;
  accentColor: string;
}

export function ValidatorLogo({
  validatorName,
  validatorImage,
  accentColor,
}: ValidatorLogoProps) {
  if (!validatorImage) {
    return (
      <ValidatorLetterAvatar
        validatorName={validatorName}
        accentColor={accentColor}
      />
    );
  }

  return (
    <ValidatorLogoImage
      key={validatorImage}
      validatorImage={validatorImage}
      validatorName={validatorName}
      accentColor={accentColor}
    />
  );
}
