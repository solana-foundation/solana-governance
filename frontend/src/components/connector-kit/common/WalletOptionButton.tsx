import type { WalletConnectorMetadata } from '@solana/connector/react';
import { Wallet } from 'lucide-react';

import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';
import { cn } from '@/lib/utils';

export function WalletOptionButton({
  connector,
  isConnecting,
  isRecent,
  hasError,
  compact,
  onSelect,
}: {
  connector: WalletConnectorMetadata;
  isConnecting: boolean;
  isRecent: boolean;
  hasError: boolean;
  compact?: boolean;
  onSelect: (connector: WalletConnectorMetadata) => void;
}) {
  return (
    <Button
      variant="outline"
      className={cn(
        'h-auto w-full justify-between rounded-[16px] p-4',
        hasError && 'border-destructive/50 bg-destructive/5 hover:bg-destructive/10',
      )}
      onClick={() => onSelect(connector)}
      disabled={isConnecting}
    >
      <div className="flex flex-1 items-center gap-3">
        <div className="flex-1 text-left">
          <div className="flex items-center gap-2">
            <span className={cn('font-semibold', compact ? 'text-sm' : 'text-md')}>
              {connector.name}
            </span>
            {isRecent && <Badge variant="secondary">Recent</Badge>}
          </div>
          {isConnecting && <div className="text-xs text-muted-foreground">Connecting...</div>}
          {hasError && <div className="text-xs text-destructive">Click to retry</div>}
        </div>
      </div>
      <div className="flex items-center gap-2">
        {isConnecting && <Spinner />}
        <Avatar size="lg">
          <AvatarImage src={connector.icon} alt={connector.name} />
          <AvatarFallback>
            <Wallet className="size-5" />
          </AvatarFallback>
        </Avatar>
      </div>
    </Button>
  );
}
