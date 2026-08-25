import { cn } from '@/lib/utils';

export function HiddenWalletIcons({
  wallets,
  maxIcons = 4,
  className,
}: {
  wallets: { id: string; icon?: string | null }[];
  maxIcons?: number;
  className?: string;
}) {
  const previewWallets = wallets.slice(0, maxIcons);
  const placeholderCount = Math.max(0, maxIcons - previewWallets.length);
  const placeholderIds = ['ph-0', 'ph-1', 'ph-2', 'ph-3'].slice(0, placeholderCount);

  return (
    <div className={cn('grid grid-cols-2 gap-1', className)} aria-hidden="true">
      {previewWallets.map(wallet => (
        <div
          key={wallet.id}
          className="relative h-4.5 w-4.5 overflow-hidden rounded-full border border-border bg-muted"
        >
          {wallet.icon ? (
            <img
              src={wallet.icon}
              alt=""
              className="h-full w-full object-cover"
              draggable={false}
              onError={e => {
                e.currentTarget.style.display = 'none';
              }}
            />
          ) : null}
        </div>
      ))}
      {placeholderIds.map(placeholderId => (
        <div
          key={placeholderId}
          className="h-4.5 w-4.5 rounded-full border border-border bg-muted"
        />
      ))}
    </div>
  );
}
