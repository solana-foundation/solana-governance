'use client';

import { useConnector } from '@solana/connector/react';
import { Wallet, ChevronDown } from 'lucide-react';

import { shortAddress } from '@/components/connector-kit/common/utils';
import { useWallet } from '@/contexts/WalletContext';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Spinner } from '@/components/ui/spinner';
import { cn } from '@/lib/utils';

import { WalletDropdownContent } from './WalletDropdownContent';

interface ConnectButtonProps {
  className?: string;
}

export function ConnectButton({
  className,
}: ConnectButtonProps) {
  const { openModal } = useWallet();
  const { isConnected, isConnecting, account, connector } = useConnector();

  if (isConnected && account && connector) {
    const walletIcon = connector.icon || undefined;

    return (
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm" className={cn('gap-2', className)}>
            <Avatar className="size-5">
              <AvatarImage src={walletIcon} alt={connector.name} />
              <AvatarFallback>
                <Wallet className="size-3" />
              </AvatarFallback>
            </Avatar>
            <span className="text-xs">{shortAddress(account)}</span>
            <ChevronDown className="h-4 w-4 opacity-50" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent
          align="end"
          sideOffset={8}
          className="rounded-[20px] border-border bg-[#15101b] p-0 opacity-100"
        >
          <WalletDropdownContent
            selectedAccount={account}
            walletIcon={walletIcon}
            walletName={connector.name}
          />
        </DropdownMenuContent>
      </DropdownMenu>
    );
  }

  const buttonContent = isConnecting ? (
    <>
      <Spinner />
      <span className="text-xs">Connecting...</span>
    </>
  ) : (
    <span className="!text-[14px]">Connect Wallet</span>
  );

  return (
    <Button size="sm" variant="outline" onClick={openModal} className={className}>
      {buttonContent}
    </Button>
  );
}
