'use client';

import {
  isWalletConnectorId,
  useConnector,
  type WalletConnectorId,
  type WalletConnectorMetadata,
} from '@solana/connector/react';
import { Wallet, ExternalLink, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';

import { ErrorAlert } from '@/components/connector-kit/common/ErrorAlert';
import { HiddenWalletIcons } from '@/components/connector-kit/common/HiddenWalletIcons';
import { getInstallUrl } from '@/components/connector-kit/common/utils';
import { WalletOptionButton } from '@/components/connector-kit/common/WalletOptionButton';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '@/components/ui/collapsible';
import { Separator } from '@/components/ui/separator';
import { Spinner } from '@/components/ui/spinner';

export interface WalletContentProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function WalletContent({ open, onOpenChange }: WalletContentProps) {
  const { walletStatus, isConnecting, connectorId, connectors, connectWallet, disconnectWallet } =
    useConnector();
  const status = walletStatus.status;

  const [connectingConnectorId, setConnectingConnectorId] = useState<WalletConnectorId | null>(
    null,
  );
  const [isClient, setIsClient] = useState(false);
  const [recentlyConnectedConnectorId, setRecentlyConnectedConnectorId] =
    useState<WalletConnectorId | null>(null);
  const [isOtherWalletsOpen, setIsOtherWalletsOpen] = useState(false);
  const [errorConnectorId, setErrorConnectorId] = useState<WalletConnectorId | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const previousOpenRef = useRef(open);

  useEffect(() => {
    setIsClient(true);
  }, []);

  useEffect(() => {
    const recent = localStorage.getItem('recentlyConnectedConnectorId');
    if (recent && isWalletConnectorId(recent)) {
      setRecentlyConnectedConnectorId(recent);
    }
  }, []);

  useEffect(() => {
    if (status !== 'connected' || !connectorId) return;
    localStorage.setItem('recentlyConnectedConnectorId', connectorId);
    setRecentlyConnectedConnectorId(connectorId);
  }, [status, connectorId]);

  const cancelConnection = useCallback(() => {
    setConnectingConnectorId(null);
    disconnectWallet().catch(() => {});
  }, [disconnectWallet]);

  const clearError = () => {
    setErrorConnectorId(null);
    setErrorMessage(null);
  };

  useEffect(() => {
    if (previousOpenRef.current && !open) {
      clearError();
      if (isConnecting || connectingConnectorId) {
        cancelConnection();
      }
    }
    previousOpenRef.current = open;
  }, [open, cancelConnection, connectingConnectorId, isConnecting]);

  const handleSelectWallet = async (connector: WalletConnectorMetadata) => {
    clearError();
    setConnectingConnectorId(connector.id);
    try {
      await connectWallet(connector.id);
      localStorage.setItem('recentlyConnectedConnectorId', connector.id);
      setRecentlyConnectedConnectorId(connector.id);
      onOpenChange(false);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'An unexpected error occurred';
      if (message.includes('Connection cancelled')) return;

      setErrorConnectorId(connector.id);
      setErrorMessage(message);

      console.error('Failed to connect wallet:', {
        wallet: connector.name,
        connectorId: connector.id,
        error,
        message,
        timestamp: new Date().toISOString(),
      });
    } finally {
      setConnectingConnectorId(null);
    }
  };

  const readyConnectors = connectors.filter(c => c.ready);
  const notReadyConnectors = connectors.filter(c => !c.ready);

  const sortedReadyConnectors = readyConnectors.toSorted((a, b) => {
    const aIsRecent = recentlyConnectedConnectorId === a.id;
    const bIsRecent = recentlyConnectedConnectorId === b.id;
    if (aIsRecent && !bIsRecent) return -1;
    if (!aIsRecent && bIsRecent) return 1;
    return 0;
  });

  const primaryWallets = sortedReadyConnectors.slice(0, 3);
  const otherWallets = sortedReadyConnectors.slice(3);

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-lg font-semibold">Connect your wallet</h2>
        <button
          type="button"
          onClick={() => onOpenChange(false)}
          className="inline-flex h-8 w-8 cursor-pointer items-center justify-center rounded-[16px] border border-input bg-background p-2 shadow-xs hover:bg-accent hover:text-accent-foreground"
        >
          <X className="h-3 w-3" />
        </button>
      </div>

      <div className="space-y-4">
        {errorMessage && <ErrorAlert message={errorMessage} onDismiss={clearError} />}

        {!isClient ? (
          <div className="py-8 text-center">
            <Spinner className="mx-auto mb-2 size-6" />
            <p className="text-sm text-muted-foreground">Detecting wallets...</p>
          </div>
        ) : (
          <>
            {primaryWallets.length > 0 && (
              <div className="space-y-2">
                <div className="grid gap-2">
                  {primaryWallets.map(connector => {
                    const isThisConnecting =
                      connectingConnectorId === connector.id ||
                      (isConnecting && connectorId === connector.id);
                    return (
                      <WalletOptionButton
                        key={connector.id}
                        connector={connector}
                        isConnecting={isThisConnecting}
                        isRecent={recentlyConnectedConnectorId === connector.id}
                        hasError={errorConnectorId === connector.id && !isConnecting}
                        onSelect={handleSelectWallet}
                      />
                    );
                  })}
                </div>
              </div>
            )}

            {otherWallets.length > 0 && (
              <>
                {primaryWallets.length > 0 && <Separator />}
                <Collapsible open={isOtherWalletsOpen} onOpenChange={setIsOtherWalletsOpen}>
                  <CollapsibleTrigger className="flex w-full cursor-pointer items-center justify-between rounded-[16px] border bg-background px-4 py-2 shadow-xs hover:bg-accent hover:text-accent-foreground hover:no-underline active:scale-[0.98] dark:border-input dark:bg-input/30 dark:hover:bg-input/50">
                    <span className="text-md font-semibold">Other Wallets</span>
                    <HiddenWalletIcons wallets={otherWallets} className="shrink-0" />
                  </CollapsibleTrigger>
                  <CollapsibleContent>
                    <div className="grid gap-2 pt-2">
                      {otherWallets.map(connector => {
                        const isThisConnecting =
                          connectingConnectorId === connector.id ||
                          (isConnecting && connectorId === connector.id);
                        return (
                          <WalletOptionButton
                            key={connector.id}
                            connector={connector}
                            isConnecting={isThisConnecting}
                            isRecent={recentlyConnectedConnectorId === connector.id}
                            hasError={errorConnectorId === connector.id && !isConnecting}
                            compact
                            onSelect={handleSelectWallet}
                          />
                        );
                      })}
                    </div>
                  </CollapsibleContent>
                </Collapsible>
              </>
            )}

            {notReadyConnectors.length > 0 && (
              <>
                {(primaryWallets.length > 0 || otherWallets.length > 0) && <Separator />}
                <div className="space-y-2">
                  <h3 className="px-1 text-sm font-medium text-muted-foreground">
                    {readyConnectors.length > 0 ? 'Unavailable Wallets' : 'Wallets'}
                  </h3>
                  <div className="grid gap-2">
                    {notReadyConnectors.slice(0, 3).map(connector => {
                      const installUrl = getInstallUrl(connector.name);

                      return (
                        <div
                          key={connector.id}
                          className="flex w-full items-center justify-between rounded-[16px] border bg-background p-4 shadow-xs"
                        >
                          <div className="flex items-center gap-3">
                            <Avatar>
                              <AvatarImage src={connector.icon} alt={connector.name} />
                              <AvatarFallback>
                                <Wallet className="size-4" />
                              </AvatarFallback>
                            </Avatar>
                            <div className="text-left">
                              <div className="text-sm font-medium">{connector.name}</div>
                              <div className="text-xs text-muted-foreground">Not available</div>
                            </div>
                          </div>
                          {installUrl && (
                            <Button
                              variant="ghost"
                              size="sm"
                              className="h-8 cursor-pointer px-2"
                              onClick={() => window.open(installUrl, '_blank')}
                            >
                              <ExternalLink className="h-4 w-4 text-muted-foreground" />
                            </Button>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              </>
            )}

            {connectors.length === 0 && (
              <div className="rounded-lg border border-dashed p-8 text-center">
                <Wallet className="mx-auto mb-3 h-12 w-12 text-muted-foreground" />
                <h3 className="mb-2 font-semibold">No Wallets Detected</h3>
                <p className="mb-6 text-sm text-muted-foreground">
                  Install a Solana wallet extension to get started
                </p>
                <div className="flex justify-center gap-2">
                  <Button
                    onClick={() => window.open('https://phantom.app', '_blank')}
                    className="bg-purple-600 hover:bg-purple-700"
                  >
                    Get Phantom
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() => window.open('https://backpack.app', '_blank')}
                  >
                    Get Backpack
                  </Button>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
