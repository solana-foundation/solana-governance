"use client";

import { useState } from "react";
import { motion, AnimatePresence, type PanInfo } from "framer-motion";
import { cn } from "@/lib/utils";
import { calculateVotingEndsIn } from "@/helpers";
import { useMounted } from "@/hooks";
import { ProposalStatus } from "@/types";

interface TimeRemainingCarouselProps {
  lifecycleStage: ProposalStatus;
  supportToDiscussionEnd: Date;
  discussionToVotingEnd: Date;
  hasEnded: boolean;
}

interface CarouselCard {
  label: string;
  stage: "supporting" | "discussion";
  endDate: Date;
}

const carouselVariants = {
  enter: (dir: number) => ({
    x: dir > 0 ? "40%" : "-40%",
    scale: 0.92,
    opacity: 0,
  }),
  center: {
    x: 0,
    scale: 1,
    opacity: 1,
  },
  exit: (dir: number) => ({
    x: dir > 0 ? "-40%" : "40%",
    scale: 0.92,
    opacity: 0,
  }),
};

export function TimeRemainingCarousel({
  lifecycleStage,
  supportToDiscussionEnd,
  discussionToVotingEnd,
  hasEnded,
}: TimeRemainingCarouselProps) {
  const mounted = useMounted();

  const allCards: CarouselCard[] = [
    {
      label: "Time Remaining",
      stage: "supporting",
      endDate: supportToDiscussionEnd,
    },
    {
      label: "Time Remaining",
      stage: "discussion",
      endDate: discussionToVotingEnd,
    },
  ];

  const cards = lifecycleStage === "supporting" ? [allCards[0]] : allCards;

  const getInitialIndex = () =>
    lifecycleStage === "discussion" ? 1 : 0;

  const [activeIndex, setActiveIndex] = useState(getInitialIndex);
  const [direction, setDirection] = useState(0);
  const [prevStage, setPrevStage] = useState(lifecycleStage);

  if (prevStage !== lifecycleStage) {
    setPrevStage(lifecycleStage);
    setActiveIndex(getInitialIndex());
  }

  const showCarousel = cards.length > 1;

  const goTo = (newIndex: number) => {
    setActiveIndex((prev) => {
      const clamped = Math.max(0, Math.min(newIndex, cards.length - 1));
      if (clamped === prev) return prev;
      setDirection(clamped > prev ? 1 : -1);
      return clamped;
    });
  };

  const handleDragEnd = (
    _: MouseEvent | TouchEvent | PointerEvent,
    { offset }: PanInfo,
  ) => {
    if (offset.x < -20 && activeIndex < cards.length - 1) {
      goTo(activeIndex + 1);
    } else if (offset.x > 20 && activeIndex > 0) {
      goTo(activeIndex - 1);
    }
  };

  const safeActiveIndex = Math.max(0, Math.min(activeIndex, cards.length - 1));
  const currentCard = cards[safeActiveIndex];
  const now = new Date();

  const isCardEnded = (() => {
    if (hasEnded) return true;
    if (currentCard.endDate < now) return true;
    if (
      currentCard.stage === "supporting" &&
      lifecycleStage !== "supporting"
    ) {
      return true;
    }
    if (
      currentCard.stage === "discussion" &&
      (lifecycleStage === "voting" || lifecycleStage === "finalized")
    ) {
      return true;
    }
    return false;
  })();

  const timeRemaining = mounted
    ? calculateVotingEndsIn(currentCard.endDate.toISOString())
    : null;

  const formattedEndDate = currentCard.endDate.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    timeZone: "UTC",
    timeZoneName: "short",
  });

  const stageName =
    currentCard.stage === "supporting" ? "Support" : "Discussion";

  const cardContent = (
    <>
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs text-white/60">{currentCard.label}</span>
      </div>
      <span className="mt-3 text-xl font-semibold text-foreground">
        {isCardEnded ? "Ended" : timeRemaining || "--"}
      </span>
      <div className="mt-auto pt-2">
        <p className="m-0 text-xs leading-[1.2] text-white/40">
          {isCardEnded
            ? `${stageName} ended ${formattedEndDate}`
            : `${stageName} ends ${formattedEndDate}`}
        </p>
      </div>
    </>
  );

  return (
    <div className="relative flex h-full flex-col rounded-xl bg-white/3 p-4 overflow-hidden">
      {showCarousel ? (
        <motion.div
          className="relative flex flex-1 flex-col cursor-grab active:cursor-grabbing select-none"
          drag="x"
          dragConstraints={{ left: 0, right: 0 }}
          dragElastic={0.15}
          onDragEnd={handleDragEnd}
        >
          <AnimatePresence initial={false} custom={direction} mode="popLayout">
            <motion.div
              key={safeActiveIndex}
              custom={direction}
              variants={carouselVariants}
              initial="enter"
              animate="center"
              exit="exit"
              transition={{ type: "spring", stiffness: 500, damping: 35 }}
              className="flex flex-1 flex-col"
            >
              {cardContent}
            </motion.div>
          </AnimatePresence>
        </motion.div>
      ) : (
        <div className="flex flex-1 flex-col">{cardContent}</div>
      )}

      {showCarousel && (
        <div className="mt-3 flex items-center justify-center gap-1.5">
          {cards.map((_, index) => (
            <button
              key={index}
              type="button"
              onClick={() => goTo(index)}
              className="group cursor-pointer"
              aria-label={`Go to card ${index + 1}`}
            >
              <span
                className={cn(
                  "block h-1.5 rounded-full transition-all duration-200",
                  index === safeActiveIndex
                    ? "w-4 bg-primary"
                    : "w-1.5 bg-white/20 group-hover:bg-white/40",
                )}
              />
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
