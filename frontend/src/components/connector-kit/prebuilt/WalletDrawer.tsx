'use client';

import { Drawer, DrawerContent } from '@/components/ui/drawer';

import { WalletContent, type WalletContentProps } from './WalletContent';

export function WalletDrawer(props: WalletContentProps) {
  return (
    <Drawer open={props.open} onOpenChange={props.onOpenChange}>
      <DrawerContent className="max-h-[90dvh] overflow-hidden rounded-t-[24px] bg-[#15101b] opacity-100">
        <div className="min-h-0 overflow-y-auto overscroll-contain p-6 pb-[max(1.5rem,env(safe-area-inset-bottom))]">
          <WalletContent {...props} />
        </div>
      </DrawerContent>
    </Drawer>
  );
}
