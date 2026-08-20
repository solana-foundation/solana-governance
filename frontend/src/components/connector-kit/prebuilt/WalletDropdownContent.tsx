'use client';

import {
  BalanceElement,
  ClusterElement,
  TokenListElement,
  TransactionHistoryElement,
  DisconnectElement,
} from '@solana/connector/react';
import {
  Wallet,
  Copy,
  Globe,
  ChevronLeft,
  ChevronDown,
  Check,
  RefreshCw,
  Coins,
  History,
  ExternalLink,
  ArrowUpRight,
  ArrowDownLeft,
  LogOut,
} from 'lucide-react';
import { motion } from 'motion/react';
import { useState } from 'react';

import { SwapTokenIcon } from '@/components/connector-kit/common/SwapTokenIcon';
import {
  clusterColors,
  getTransactionSubtitle,
  getTransactionTitle,
  shortAddress,
} from '@/components/connector-kit/common/utils';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '@/components/ui/collapsible';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';

export interface WalletDropdownContentProps {
  selectedAccount: string;
  walletIcon?: string;
  walletName: string;
  showCluster?: boolean;
  showBalance?: boolean;
  showTokenList?: boolean;
  showTransactionHistory?: boolean;
}

type DropdownView = 'wallet' | 'network';

export function WalletDropdownContent({
  selectedAccount,
  walletIcon,
  walletName,
  showCluster = true,
  showBalance = true,
  showTokenList = true,
  showTransactionHistory = true,
}: WalletDropdownContentProps) {
  const [view, setView] = useState<DropdownView>('wallet');
  const [copied, setCopied] = useState(false);
  const [isTokensOpen, setIsTokensOpen] = useState(false);
  const [isTransactionsOpen, setIsTransactionsOpen] = useState(false);

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

  if (view === 'wallet' || !showCluster) {
    return (
      <motion.div
        key="wallet"
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
          <div className="flex items-center gap-2">
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

            {showCluster ? (
              <ClusterElement
                render={({ cluster }) => (
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    className="relative rounded-full"
                    onClick={() => setView('network')}
                    title={`Network: ${cluster?.label || 'Unknown'}`}
                  >
                    <Globe className="h-4 w-4" />
                    <span
                      className={cn(
                        'absolute -right-0.5 -bottom-0.5 h-3.5 w-3.5 rounded-full border-2 border-background',
                        clusterColors[cluster?.id || ''] || 'bg-emerald-500',
                      )}
                    />
                  </Button>
                )}
              />
            ) : null}
          </div>
        </div>

        {showBalance ? (
          <BalanceElement
            render={({ solBalance, isLoading, refetch }) => (
              <div className="rounded-[12px] border bg-muted/50 p-4">
                <div className="mb-1 flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">Balance</span>
                  <button
                    onClick={() => refetch()}
                    disabled={isLoading}
                    className="cursor-pointer rounded p-1 transition-colors hover:bg-accent disabled:opacity-50"
                  >
                    <RefreshCw className={cn('h-3.5 w-3.5', isLoading && 'animate-spin')} />
                  </button>
                </div>
                <div className="text-2xl font-bold">
                  {isLoading ? (
                    <div className="h-8 w-32 animate-pulse rounded bg-muted" />
                  ) : solBalance !== null ? (
                    `${solBalance.toFixed(4)} SOL`
                  ) : (
                    '-- SOL'
                  )}
                </div>
              </div>
            )}
          />
        ) : null}

        {showTokenList || showTransactionHistory ? <Separator className="scale-x-110" /> : null}

        {showTokenList || showTransactionHistory ? (
          <div className="space-y-2">
            {showTokenList ? (
              <Collapsible
                open={isTokensOpen}
                onOpenChange={setIsTokensOpen}
                className="rounded-[12px] border px-3"
              >
                <CollapsibleTrigger className="flex w-full items-center justify-between py-3 hover:cursor-pointer hover:no-underline">
                  <div className="flex items-center gap-2">
                    <Coins className="h-4 w-4" />
                    <span className="text-sm font-medium">Tokens</span>
                  </div>
                  <ChevronDown
                    className={cn(
                      'h-4 w-4 shrink-0 transition-transform duration-200',
                      isTokensOpen && 'rotate-180',
                    )}
                  />
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <TokenListElement
                    limit={5}
                    render={({ tokens, isLoading }) => (
                      <div className="space-y-2 pb-2">
                        {isLoading ? (
                          <div className="space-y-2">
                            {[1, 2, 3].map(i => (
                              <div key={i} className="flex items-center gap-3">
                                <div className="h-8 w-8 animate-pulse rounded-full bg-muted" />
                                <div className="flex-1">
                                  <div className="mb-1 h-4 w-16 animate-pulse rounded bg-muted" />
                                  <div className="h-3 w-24 animate-pulse rounded bg-muted" />
                                </div>
                              </div>
                            ))}
                          </div>
                        ) : tokens.length > 0 ? (
                          tokens.map(token => (
                            <div key={token.mint} className="flex items-center gap-3 py-1">
                              {token.logo ? (
                                <img
                                  src={token.logo}
                                  className="h-8 w-8 rounded-full"
                                  alt={token.symbol}
                                />
                              ) : (
                                <div className="flex h-8 w-8 items-center justify-center rounded-full bg-muted">
                                  <Coins className="h-4 w-4" />
                                </div>
                              )}
                              <div className="min-w-0 flex-1">
                                <p className="truncate text-sm font-medium">{token.symbol}</p>
                                <p className="truncate text-xs text-muted-foreground">
                                  {token.name}
                                </p>
                              </div>
                              <p className="font-mono text-sm">{token.formatted}</p>
                            </div>
                          ))
                        ) : (
                          <p className="py-2 text-center text-sm text-muted-foreground">
                            No tokens found
                          </p>
                        )}
                      </div>
                    )}
                  />
                </CollapsibleContent>
              </Collapsible>
            ) : null}

            {showTransactionHistory ? (
              <Collapsible
                open={isTransactionsOpen}
                onOpenChange={setIsTransactionsOpen}
                className="rounded-[12px] border px-3"
              >
                <CollapsibleTrigger className="flex w-full items-center justify-between py-3 hover:cursor-pointer hover:no-underline">
                  <div className="flex items-center gap-2">
                    <History className="h-4 w-4" />
                    <span className="text-sm font-medium">Recent Activity</span>
                  </div>
                  <ChevronDown
                    className={cn(
                      'h-4 w-4 shrink-0 transition-transform duration-200',
                      isTransactionsOpen && 'rotate-180',
                    )}
                  />
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <TransactionHistoryElement
                    limit={5}
                    render={({ transactions, isLoading }) => (
                      <div className="space-y-2 pb-2">
                        {isLoading ? (
                          <div className="space-y-2">
                            {[1, 2, 3].map(i => (
                              <div key={i} className="flex items-center gap-3">
                                <div className="h-8 w-8 animate-pulse rounded-full bg-muted" />
                                <div className="flex-1">
                                  <div className="mb-1 h-4 w-20 animate-pulse rounded bg-muted" />
                                  <div className="h-3 w-16 animate-pulse rounded bg-muted" />
                                </div>
                              </div>
                            ))}
                          </div>
                        ) : transactions.length > 0 ? (
                          transactions.map(tx => (
                            <a
                              key={tx.signature}
                              href={tx.explorerUrl}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="-mx-1 flex cursor-pointer items-center gap-3 rounded-lg px-1 py-1 transition-colors hover:bg-muted/50"
                            >
                              <div className="relative">
                                {tx.type === 'swap' && (tx.swapFromToken || tx.swapToToken) ? (
                                  <SwapTokenIcon
                                    fromIcon={tx.swapFromToken?.icon}
                                    toIcon={tx.swapToToken?.icon}
                                    size={32}
                                  />
                                ) : tx.tokenIcon ? (
                                  <img src={tx.tokenIcon} className="h-8 w-8 rounded-full" alt="" />
                                ) : (
                                  <div className="flex h-8 w-8 items-center justify-center rounded-full bg-muted">
                                    <History className="h-4 w-4" />
                                  </div>
                                )}
                                {(tx.direction === 'in' || tx.direction === 'out') && (
                                  <div
                                    className={cn(
                                      'absolute -right-0.5 -bottom-0.5 flex h-4 w-4 items-center justify-center rounded-full border-2 border-background',
                                      tx.direction === 'in'
                                        ? 'bg-green-500 text-white'
                                        : 'bg-orange-500 text-white',
                                    )}
                                  >
                                    {tx.direction === 'in' ? (
                                      <ArrowDownLeft className="h-2 w-2" />
                                    ) : (
                                      <ArrowUpRight className="h-2 w-2" />
                                    )}
                                  </div>
                                )}
                              </div>
                              <div className="min-w-0 flex-1">
                                <p className="text-sm font-medium">{getTransactionTitle(tx)}</p>
                                <p className="text-xs text-muted-foreground">
                                  {getTransactionSubtitle(tx)}
                                </p>
                              </div>
                              <div className="flex items-center gap-2">
                                {tx.formattedAmount ? (
                                  <span
                                    className={cn(
                                      'text-sm font-medium',
                                      tx.direction === 'in'
                                        ? 'text-green-600'
                                        : tx.direction === 'out'
                                          ? 'text-orange-600'
                                          : 'text-muted-foreground',
                                    )}
                                  >
                                    {tx.formattedAmount}
                                  </span>
                                ) : null}
                                <ExternalLink className="h-3 w-3 text-muted-foreground" />
                              </div>
                            </a>
                          ))
                        ) : (
                          <p className="py-2 text-center text-sm text-muted-foreground">
                            No transactions yet
                          </p>
                        )}
                      </div>
                    )}
                  />
                </CollapsibleContent>
              </Collapsible>
            ) : null}
          </div>
        ) : null}

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

  return (
    <motion.div
      key="network"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.15 }}
      className="w-[360px] space-y-4 p-4"
    >
      <div className="flex items-center gap-3">
        <button
          type="button"
          onClick={() => setView('wallet')}
          className="cursor-pointer rounded-full border border-border p-2 transition-colors hover:bg-accent"
        >
          <ChevronLeft className="h-4 w-4" />
        </button>
        <span className="text-lg font-semibold">Network Settings</span>
      </div>

      <ClusterElement
        render={({ cluster, clusters, setCluster }) => {
          const currentClusterId = cluster?.id || 'solana:mainnet';
          return (
            <div className="overflow-hidden rounded-[12px] border bg-muted/50">
              {clusters.map((network, index) => {
                const isSelected = currentClusterId === network.id;
                return (
                  <button
                    key={network.id}
                    type="button"
                    onClick={() => {
                      void setCluster(network.id);
                    }}
                    className={cn(
                      'flex w-full cursor-pointer items-center justify-between bg-transparent p-4 text-left transition-colors hover:bg-accent/50',
                      index !== clusters.length - 1 && 'border-b border-border',
                    )}
                  >
                    <div className="flex items-center gap-2">
                      <span
                        className={cn(
                          'h-2 w-2 rounded-full',
                          clusterColors[network.id] || 'bg-purple-500',
                        )}
                      />
                      <span className="font-medium">{network.label}</span>
                    </div>
                    <div
                      className={cn(
                        'flex h-5 w-5 items-center justify-center rounded-full border-2 transition-colors',
                        isSelected ? 'border-primary bg-primary' : 'border-muted-foreground/30',
                      )}
                    >
                      {isSelected && <Check className="h-3 w-3 text-primary-foreground" />}
                    </div>
                  </button>
                );
              })}
            </div>
          );
        }}
      />
    </motion.div>
  );
}
