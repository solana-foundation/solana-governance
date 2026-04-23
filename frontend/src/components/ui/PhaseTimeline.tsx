import { cn } from "@/lib/utils";
import { ProposalStatus } from "@/types";
import { Vote, PenLine, MessageSquareText } from "lucide-react";

interface Props {
  currentPhase: ProposalStatus | undefined;
}

export const PhaseTimeline = ({ currentPhase }: Props) => {
  const isSupporting = currentPhase === "supporting";
  const inPreVoting =
    currentPhase === "supporting" ||
    currentPhase === "discussion";
  const isVoting = currentPhase === "voting";
  const isFinished = currentPhase === "finalized";

  return (
    <>
      <div className="flex px-5">
        {/* phase 1 */}
        <div className="flex items-center relative w-[33%]">
          <div className="w-10.5 h-10.5 bg-green-dark rounded-full border-green-secondary border-[1px] z-1 flex items-center justify-center">
            <MessageSquareText
              size={18}
              className={
                inPreVoting && !isFinished
                  ? "text-green-icon-active"
                  : "text-green-icon"
              }
            />
          </div>
          <div
            className={cn(
              "absolute left-10 h-1.5 w-[calc(100%-theme(spacing.9))] border-[1px]",
              isVoting || isFinished
                ? "bg-green-dark border-green-secondary"
                : "bg-black border-gray-secondary"
            )}
          />
          {isSupporting && (
            <div className="absolute left-10 h-1.5 w-[calc(50%-theme(spacing.4))] bg-gradient-to-r from-green-dark to-green border-green-secondary border-[1px] rounded-full" />
          )}
        </div>
        {/* phase 2 */}
        <div className="flex items-center relative w-[33%]">
          <div className="w-10.5 h-10.5 bg-green-dark rounded-full border-green-secondary border-[1px] z-1 flex items-center justify-center">
            <PenLine
              size={18}
              className={
                isVoting ? "text-green-icon-active" : "text-green-icon"
              }
            />
          </div>
          <div
            className={cn(
              "absolute left-10 h-1.5 w-[calc(100%-theme(spacing.9))] border-[1px]",
              isFinished
                ? "bg-green-dark border-green-secondary"
                : "bg-black border-gray-secondary"
            )}
          />
          {isVoting && (
            <div className="absolute left-10 h-1.5 w-[calc(50%-theme(spacing.4))] bg-gradient-to-r from-green-dark to-green border-green-secondary border-[1px] rounded-full" />
          )}
          {/* border-gray-secondary */}
        </div>
        {/* phase 3 */}
        <div className="flex items-center relative">
          <div className="w-10.5 h-10.5 bg-green-dark rounded-full border-green-secondary border-[1px] z-1 flex items-center justify-center">
            <Vote
              size={18}
              className={
                isFinished ? "text-green-icon-active" : "text-green-icon"
              }
            />
          </div>
        </div>
      </div>

      <div className="flex relative mt-2 ml-2">
        <div className="flex flex-col items-center">
          <span>Support</span>
          {/* <span className="text-dao-text-secondary text-xs">Epoch: 750</span>
          <span className="text-dao-text-secondary text-xs">120 Days</span> */}
        </div>
        <div className="absolute flex flex-col items-center left-[36%] transform -translate-x-[36%]">
          <span>Voting</span>
          {/* <span className="text-dao-text-secondary text-xs">Epoch: 750</span>
          <span className="text-dao-text-secondary text-xs">60 Days</span> */}
        </div>
        <div className="absolute flex flex-col items-center left-[71%] transform -translate-x-[71%]">
          <span>Finished</span>
          {/* <span className="text-dao-text-secondary text-xs">Epoch: 750</span>
          <span className="text-dao-text-secondary text-xs">23 Days</span> */}
        </div>
      </div>
    </>
  );
};
