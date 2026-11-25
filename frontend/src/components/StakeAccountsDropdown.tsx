import { useWalletStakeAccounts } from "@/hooks";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui";
import {
  formatAddress,
  formatLamportsDisplay,
} from "@/lib/governance/formatters";
import { Loader2 } from "lucide-react";
import { useConnector } from "@solana/connector/react";
import { StakeAccountData } from "@/types";

interface VotingProposalsDropdownProps {
  value?: string;
  onValueChange: (value: string) => void;
  disabled?: boolean;
  disabledAccounts?: string[];
}

export const StakeAccountsDropdown = ({
  value,
  onValueChange,
  disabled,
  disabledAccounts,
}: VotingProposalsDropdownProps) => {
  const { account } = useConnector();
  const publicKey = account ?? undefined;

  const { data: stakeAccounts, isLoading } = useWalletStakeAccounts(
    publicKey
  );

  const isAccountValid = (stakeAcc: StakeAccountData) =>
    !disabledAccounts?.includes(stakeAcc.stakeAccount) &&
    stakeAcc.activeStake > 0n;

  const validStakeAccounts = stakeAccounts?.filter(isAccountValid);

  const hasNoValidAccounts = !isLoading && validStakeAccounts?.length === 0;

  return (
    <>
      <Select
        value={value}
        onValueChange={onValueChange}
        disabled={isLoading || disabled}
      >
        <SelectTrigger className="text-white w-full">
          <div className="flex gap-1 items-center">
            <span className="text-dao-text-secondary">Stake account:</span>
            {isLoading ? (
              <Loader2 className="size-4 animate-spin text-white/60" />
            ) : (
              <SelectValue placeholder="-" />
            )}
          </div>
        </SelectTrigger>
        <SelectContent className="text-white bg-background/40 backdrop-blur">
          {stakeAccounts?.map((stakeAcc) => (
            <SelectItem
              key={stakeAcc.stakeAccount}
              value={stakeAcc.stakeAccount}
              disabled={!isAccountValid(stakeAcc)}
            >
              {formatAddress(stakeAcc.stakeAccount)} -&nbsp;
              {formatLamportsDisplay(stakeAcc.activeStake).value}
              {stakeAcc.activeStake === 0n && (
                <span className="text-white/40">
                  &nbsp;(Insufficient balance)
                </span>
              )}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {hasNoValidAccounts && (
        <p className="text-xs text-destructive">
          No valid stake accounts available to vote
        </p>
      )}
    </>
  );
};
