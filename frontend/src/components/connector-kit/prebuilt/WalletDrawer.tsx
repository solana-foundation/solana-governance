'use client';

import { Drawer, DrawerContent } from '@/components/ui/drawer';

import { WalletConnectContent, type WalletConnectContentProps } from './WalletConnectContent';

export function WalletDrawer(props: WalletConnectContentProps) {
  return (
    <Drawer open={props.open} onOpenChange={props.onOpenChange}>
      <DrawerContent className="max-h-[90dvh] overflow-hidden rounded-t-[24px] bg-[#15101b] opacity-100">
        <div className="min-h-0 overflow-y-auto overscroll-contain p-6 pb-[max(1.5rem,env(safe-area-inset-bottom))]">
          <WalletConnectContent {...props} />
        </div>
      </DrawerContent>
    </Drawer>
  );
}
