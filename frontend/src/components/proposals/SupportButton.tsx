import { useModal } from "@/contexts/ModalContext";
import { AppButton } from "../ui/AppButton";
import { useWallet } from "@solana/wallet-adapter-react";
import { useWalletRole } from "@/hooks";
import { ProposalStatus, WalletRole } from "@/types";
import { useHasUserSupported } from "@/hooks/useHasUserSupported";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import { toast } from "sonner";

interface Props {
  proposalId?: string;
  proposalStatus?: ProposalStatus;
  disabled?: boolean;
}

export const SupportButton = ({
  proposalId,
  proposalStatus,
  disabled: disabledProps,
}: Props) => {
  const { openModal } = useModal();
  const { connected, publicKey } = useWallet();
  const { walletRole } = useWalletRole(publicKey?.toBase58());

  const { data: hasUserSupported, isLoading: isLoadingHasUserSupported } =
    useHasUserSupported(proposalId);

  const isValidator = [WalletRole.VALIDATOR, WalletRole.BOTH].includes(
    walletRole
  );

  const disabled =
    disabledProps ||
    isLoadingHasUserSupported ||
    hasUserSupported ||
    !connected ||
    !isValidator;

  const isSupporting = proposalStatus === "supporting";

  let tooltipText =
    !isValidator && connected
      ? "You are not authorized to support this proposal, only validators can support proposals"
      : "Wallet not connected, please connect your wallet to be able to perform these actions";

  if (hasUserSupported) {
    tooltipText = "You have already supported this proposal";
  }

  const showTooltip =
    !(connected && publicKey && isValidator) || hasUserSupported;

  if (!isSupporting) {
    return null;
  }

  if (!proposalId) {
    return (
      <AppButton
        variant="gradient"
        text="Support"
        className="w-full justify-center text-sm font-semibold text-foreground"
        disabled={disabled}
        onClick={() => {
          openModal("support-proposal");
        }}
      />
    );
  }

  const handleDisabledClick = () => {
    if (disabled) {
      toast.error(tooltipText);
    }
  };

  if (showTooltip) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>
          <span onClick={handleDisabledClick}>
            <AppButton
              variant="gradient"
              text="Support"
              className="w-full justify-center text-sm font-semibold text-foreground"
              disabled={disabled}
              onClick={() => {
                openModal("support-proposal", { proposalId });
              }}
            />
          </span>
        </TooltipTrigger>
        <TooltipContent side="bottom">
          <p className="text-sm text-red-500/80">{tooltipText}</p>
        </TooltipContent>
      </Tooltip>
    );
  }

  return (
    <>
      <AppButton
        variant="gradient"
        text="Support"
        className="w-full justify-center text-sm font-semibold text-foreground"
        disabled={disabled}
        onClick={() => {
          openModal("support-proposal", { proposalId });
        }}
      />
    </>
  );
};
