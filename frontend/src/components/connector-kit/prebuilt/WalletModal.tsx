'use client';

import { Dialog, DialogContent } from '@/components/ui/dialog';

import { WalletContent, type WalletContentProps } from './WalletContent';

export type WalletModalProps = WalletContentProps;

export function WalletModal(props: WalletModalProps) {
  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange}>
      <DialogContent
        showCloseButton={false}
        className="max-w-md rounded-[24px] bg-[#15101b] p-6 opacity-100"
      >
        <WalletContent {...props} />
      </DialogContent>
    </Dialog>
  );
}
