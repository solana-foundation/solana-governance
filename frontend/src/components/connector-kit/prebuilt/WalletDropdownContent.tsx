'use client';

import { DisconnectElement } from '@solana/connector/react';
import { Wallet, Copy, Check, LogOut } from 'lucide-react';
import { motion } from 'motion/react';
import { useState } from 'react';

import { shortAddress } from '@/components/connector-kit/common/utils';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';

export interface WalletDropdownContentProps {
  selectedAccount: string;
  walletIcon?: string;
  walletName: string;
}

export function WalletDropdownContent({
  selectedAccount,
  walletIcon,
  walletName,
}: WalletDropdownContentProps) {
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(selectedAccount);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      setCopied(false);
      console.error('Failed to copy to clipboard:', error);
    }
  }

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.15 }}
      className="w-[360px] space-y-4 p-4"
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Avatar className="size-12">
            <AvatarImage src={walletIcon} alt={walletName} />
            <AvatarFallback>
              <Wallet className="size-6" />
            </AvatarFallback>
          </Avatar>
          <div>
            <div className="text-lg font-semibold">{shortAddress(selectedAccount)}</div>
            <div className="text-xs text-muted-foreground">{walletName}</div>
          </div>
        </div>
        <Button
          type="button"
          onClick={handleCopy}
          variant="outline"
          size="icon"
          className="rounded-full"
          title={copied ? 'Copied!' : 'Copy address'}
        >
          {copied ? <Check className="h-4 w-4 text-green-500" /> : <Copy className="h-4 w-4" />}
        </Button>
      </div>

      <DisconnectElement
        render={({ disconnect, disconnecting }) => (
          <Button
            variant="default"
            className="h-11 w-full !rounded-[12px] text-[16px]"
            onClick={disconnect}
            disabled={disconnecting}
          >
            <LogOut className="mr-2 h-4 w-4" />
            {disconnecting ? 'Disconnecting...' : 'Disconnect'}
          </Button>
        )}
      />
    </motion.div>
  );
}
