'use client';

import QRCode from 'qrcode';
import { useEffect, useState } from 'react';

import { Spinner } from '@/components/ui/spinner';

export function WalletConnectQR({ value, size = 280 }: { value: string; size?: number }) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    if (!value) {
      setSrc(null);
    } else {
      void QRCode.toDataURL(value, { width: size, margin: 1, errorCorrectionLevel: 'M' })
        .then(url => {
          if (!cancelled) setSrc(url);
        })
        .catch(() => {
          if (!cancelled) setSrc(null);
        });
    }

    return () => {
      cancelled = true;
    };
  }, [value, size]);

  if (!src) {
    return (
      <div
        className="flex items-center justify-center rounded-[28px] border bg-muted/50"
        style={{ width: size, height: size }}
      >
        <Spinner className="size-6" />
      </div>
    );
  }

  return (
    <img
      src={src}
      width={size}
      height={size}
      alt="WalletConnect QR code"
      className="rounded-[28px]"
    />
  );
}
