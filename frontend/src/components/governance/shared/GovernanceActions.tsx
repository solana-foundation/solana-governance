"use client";

import { AppButton } from "@/components/ui/AppButton";
import { ViewType } from "@/types/governance";
import {
  PencilLine,
  PlusCircle,
  ThumbsUp,
  MapPinCheckInside,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useModal } from "@/contexts/ModalContext";
import { useWalletSession } from "@/contexts/WalletSessionContext";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface GovernanceActionsProps {
  variant: ViewType;
  title?: string;
  description?: string;
  gridClassName?: string;
  wrapperClassName?: string;
  className?: string;
  isLoading?: boolean;
}

interface ActionButtonConfig {
  label: string;
  variant: "gradient" | "outline";
  icon?: React.ReactNode;
  onClick: () => void;
  className: string;
  disabled?: boolean;
}

interface ActionGridProps {
  actions: ActionButtonConfig[];
  gridClassName: string;
  isLoading?: boolean;
}

function ActionGrid({ actions, gridClassName, isLoading }: ActionGridProps) {
  return (
    <div className={gridClassName}>
      {actions.map((action) => (
        <AppButton
          key={action.label}
          text={action.label}
          variant={action.variant}
          onClick={() => {
            action.onClick();
          }}
          className={action.className}
          icon={action.icon}
          size="lg"
          disabled={action.disabled || isLoading}
        />
      ))}
    </div>
  );
}

function getValidatorConfig(
  openModal: ReturnType<typeof useModal>["openModal"],
  disabled?: boolean
) {
  return {
    title: "Governance Actions",
    description:
      "Note: To create a proposal, you must use the validator's identity keypair when executing commands.",
    wrapperClassName: "glass-card p-6 space-y-4",
    descriptionClassName: "text-sm text-white/60",
    gridClassName: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4",
    actions: [
      {
        label: "Create Proposal",
        variant: "gradient",
        onClick: () => openModal("create-proposal"),
        className: "governance-action-primary",
        icon: <PlusCircle className="size-4" />,
        disabled,
      },
      {
        label: "Support Proposal",
        variant: "outline",
        onClick: () => openModal("support-proposal"),
        className: "governance-action-secondary",
        icon: <ThumbsUp className="size-4" />,
        disabled,
      },
      {
        label: "Cast Vote",
        variant: "outline",
        onClick: () => openModal("cast-vote"),
        className: "governance-action-secondary",
        icon: <MapPinCheckInside className="size-4" />,
        disabled,
      },
      {
        label: "Modify Vote",
        variant: "outline",
        onClick: () => openModal("modify-vote"),
        className: "governance-action-secondary",
        icon: <PencilLine className="size-4" />,
        disabled,
      },
    ] satisfies ActionButtonConfig[],
  };
}

function getStakerConfig(
  openModal: ReturnType<typeof useModal>["openModal"],
  disabled?: boolean
) {
  return {
    title: "Governance Actions",
    description:
      "As a staker, you can participate in governance by voting on proposals.",
    wrapperClassName: "glass-card p-6 space-y-4 h-full",
    descriptionClassName: "text-sm text-white/60",
    gridClassName: "grid grid-cols-1 sm:grid-cols-2 gap-4",
    actions: [
      {
        label: "Cast Vote",
        variant: "gradient",
        onClick: () => openModal("override-vote"),
        className: "governance-action-primary",
        icon: <MapPinCheckInside className="size-4" />,
        disabled,
      },
      {
        label: "Modify Vote",
        variant: "outline",
        onClick: () => openModal("modify-override-vote"),
        className: "governance-action-secondary",
        icon: <PencilLine className="size-4" />,
        disabled,
      },
    ] satisfies ActionButtonConfig[],
  };
}

export function GovernanceActions({
  variant,
  title,
  description,
  gridClassName,
  wrapperClassName,
  className,
  isLoading,
}: GovernanceActionsProps) {
  const { openModal } = useModal();
  const { connected } = useWalletSession();
  const config =
    variant === "validator"
      ? getValidatorConfig(openModal, !connected)
      : getStakerConfig(openModal, !connected);
  const resolvedTitle = title ?? config.title;
  const resolvedDescription =
    description === undefined ? config.description : description;
  const resolvedWrapperClassName = cn(
    wrapperClassName ?? config.wrapperClassName,
    className
  );
  const resolvedGridClassName = gridClassName ?? config.gridClassName;

  return (
    <div className={resolvedWrapperClassName}>
      <div className="space-y-2">
        <h3 className="h3 font-semibold">{resolvedTitle}</h3>
        {resolvedDescription && (
          <p className={config.descriptionClassName}>{resolvedDescription}</p>
        )}
      </div>
      {connected ? (
        <ActionGrid
          actions={config.actions}
          gridClassName={resolvedGridClassName}
          isLoading={isLoading}
        />
      ) : (
        <Tooltip>
          <TooltipTrigger asChild>
            <span>
              <ActionGrid
                actions={config.actions}
                gridClassName={resolvedGridClassName}
              />
            </span>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            <p className="text-sm text-red-500/80">
              Wallet not connected, please connect your wallet to be able to
              perform these actions
            </p>
          </TooltipContent>
        </Tooltip>
      )}
    </div>
  );
}
