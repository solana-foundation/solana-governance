import { useEndpoint } from "@/contexts/EndpointContext";
import {
  useGetValidators,
  useValidatorVotingPower,
  useWalletStakeAccounts,
} from "@/hooks";
import { useSnapshotMeta } from "@/hooks/useSnapshotMeta";
import {
  formatLamportsDisplay,
  formatOptionalCount,
  formatOptionalSlot,
} from "@/lib/governance/formatters";
import type { ViewType } from "@/types/governance";
import { useAnchorWallet } from "@solana/wallet-adapter-react";

interface DashboardStatsProps {
  currentView: ViewType;
  isLoading: boolean;
}

type StatEntry = {
  label: string;
  value: string | number;
  mobileValue?: string | number;
  rawValue?: string;
  showRaw: boolean;
  isLoading: boolean;
};

export function DashboardStats({
  currentView,
  isLoading: isLoadingProps,
}: DashboardStatsProps) {
  const { endpointType } = useEndpoint();
  const {
    data: snapshotMeta,
    isLoading: isLoadingSnapshotMeta,
    isError: isErrorSnapshotMeta,
  } = useSnapshotMeta();

  const { data: validators, isLoading: isLoadingValidators } =
    useGetValidators();

  const wallet = useAnchorWallet();

  const { data: stakeAccounts, isLoading: isLoadingStakeAccounts } =
    useWalletStakeAccounts(wallet?.publicKey?.toBase58());

  const { votingPower, isLoading: isLoadingVotingPower } =
    useValidatorVotingPower(wallet?.publicKey?.toBase58());

  const isLoading =
    isLoadingSnapshotMeta ||
    isLoadingValidators ||
    isLoadingStakeAccounts ||
    isLoadingProps;

  const snapshotSlot = snapshotMeta?.slot;
  // "-" reads as a legitimate value; say so explicitly when the NCN API could not be reached.
  const snapshotSlotValue = 
    isErrorSnapshotMeta || endpointType === "custom"
      ? "Unavailable"
      : formatOptionalSlot(snapshotSlot);
  const delegationsReceived = votingPower;
  // TODO: clarify what this number is exactly
  const voteAccountsCount = 321;

  const totalStaked =
    stakeAccounts?.reduce((acc, curr) => acc + curr.activeStake, 0) || 0;
  // Left undefined when the validator query has not resolved; formatOptionalCount
  // renders "-" rather than a "0" that reads as a real count.
  const activeValidators = validators?.length;

  // TODO: on mobile only show "custom" when the network is a custom URL, otherwise show the network name

  const stats: StatEntry[] =
    currentView === "validator"
      ? [
          {
            label: "Network",
            value: endpointType,
            mobileValue: endpointType,
            showRaw: false,
            isLoading: false,
          },
          {
            label: "Snapshot Slot",
            value: snapshotSlotValue,
            showRaw: false,
            isLoading: isLoadingSnapshotMeta,
          },
          {
            label: "Delegations Received",
            ...formatLamportsDisplay(delegationsReceived),
            isLoading: isLoadingVotingPower,
          },
          {
            label: "Vote Accounts",
            value: formatOptionalCount(voteAccountsCount),
            showRaw: false,
            isLoading,
          },
        ]
      : [
          {
            label: "Network",
            value: endpointType,
            mobileValue: endpointType,
            showRaw: false,
            isLoading: false,
          },
          {
            label: "Snapshot Slot",
            value: snapshotSlotValue,
            showRaw: false,
            isLoading: isLoadingSnapshotMeta,
          },
          {
            label: "Total Staked",
            ...formatLamportsDisplay(totalStaked),
            isLoading: isLoadingStakeAccounts,
          },
          {
            label: "Active Validators",
            value: formatOptionalCount(activeValidators),
            showRaw: false,
            isLoading: isLoadingValidators,
          },
        ];

  return (
    <div className="grid grid-cols-2 gap-3 sm:gap-4 lg:grid-cols-4 lg:gap-8">
      {stats.map((stat, index) => (
        <div key={index} className="min-w-0">
          <p className="text-(--color-dao-color-gray) text-[10px] sm:text-xs uppercase tracking-wider mb-1">
            {stat.label}
          </p>
          {stat.isLoading ? (
            <div className="h-6 w-20 bg-white/10 animate-pulse rounded-full mt-2" />
          ) : (
            <div>
              <p className="text-foreground text-base sm:text-lg lg:text-xl font-medium">
                <span className="sm:hidden">
                  {stat.mobileValue ?? stat.value}
                </span>
                <span className="hidden sm:inline">{stat.value}</span>
              </p>
              {stat.showRaw && stat.rawValue && (
                <p
                  className="text-white/50 text-[10px] mt-0.5 truncate"
                  title={stat.rawValue}
                >
                  {stat.rawValue}
                </p>
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
