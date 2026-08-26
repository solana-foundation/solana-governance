import { useProposals } from "@/hooks";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui";
import { formatAddress } from "@/lib/governance/formatters";
import { Loader2 } from "lucide-react";
import type { Address } from "@solana/kit";
import { toast } from "sonner";

interface VotingProposalsDropdownProps {
  value?: string;
  onValueChange: (proposalid: string, consensusResult: Address) => void;
  disabled?: boolean;
}

export const VotingProposalsDropdown = ({
  value,
  onValueChange,
  disabled,
}: VotingProposalsDropdownProps) => {
  const { data: proposals, isLoading } = useProposals({
    voting: true,
    finalized: false,
  });

  const handleChange = (value: string) => {
    if (proposals) {
      const consensusResult = proposals.find(
        (p) => p.publicKey === value
      )?.consensusResult;

      if (consensusResult) {
        onValueChange(value, consensusResult);
      } else {
        toast.error("Couldn't obtain consensus result");
      }
    }
  };

  return (
    <div className="space-y-2">
      <label
        htmlFor="proposal-id"
        className="text-sm font-medium text-white/80"
      >
        Proposal ID
      </label>
      <Select
        value={value}
        onValueChange={handleChange}
        disabled={isLoading || disabled}
      >
        <SelectTrigger className="text-white w-full">
          <div className="flex gap-1 items-center">
            {isLoading ? (
              <Loader2 className="size-4 animate-spin text-white/60" />
            ) : (
              <SelectValue placeholder="-" />
            )}
          </div>
        </SelectTrigger>
        <SelectContent className="text-white bg-background/40 backdrop-blur">
          {proposals?.map((proposal) => (
            <SelectItem
              key={proposal.publicKey}
              value={proposal.publicKey}
            >
              {proposal.title} -&nbsp;{" "}
              {formatAddress(proposal.publicKey)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
};
