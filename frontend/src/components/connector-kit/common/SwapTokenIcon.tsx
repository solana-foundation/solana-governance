import { Coins } from 'lucide-react';

export function SwapTokenIcon({
  fromIcon,
  toIcon,
  size = 32,
}: {
  fromIcon?: string;
  toIcon?: string;
  size?: number;
}) {
  const offset = size * 0.6;
  return (
    <div className="relative flex-shrink-0" style={{ width: size + offset, height: size }}>
      <div
        className="absolute top-0 left-0 flex items-center justify-center rounded-full border-2 border-background bg-muted"
        style={{ width: size, height: size }}
      >
        {fromIcon ? (
          <img
            src={fromIcon}
            className="rounded-full"
            style={{ width: size - 4, height: size - 4 }}
            alt=""
          />
        ) : (
          <Coins className="h-4 w-4 text-muted-foreground" />
        )}
      </div>
      <div
        className="absolute top-0 flex items-center justify-center rounded-full border-2 border-background bg-muted"
        style={{ left: offset, width: size, height: size }}
      >
        {toIcon ? (
          <img
            src={toIcon}
            className="rounded-full"
            style={{ width: size - 4, height: size - 4 }}
            alt=""
          />
        ) : (
          <Coins className="h-4 w-4 text-muted-foreground" />
        )}
      </div>
    </div>
  );
}
