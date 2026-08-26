import { X } from 'lucide-react';

export function ErrorAlert({ message, onDismiss }: { message: string; onDismiss: () => void }) {
  return (
    <div className="flex items-start gap-3 rounded-[12px] border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
      <div className="flex-1">
        <p className="font-medium">Connection failed</p>
        <p className="mt-0.5 text-xs opacity-90">{message}</p>
      </div>
      <button
        onClick={onDismiss}
        className="shrink-0 rounded-md p-1 transition-colors hover:bg-destructive/20"
        aria-label="Dismiss error"
      >
        <X className="h-3 w-3" />
      </button>
    </div>
  );
}
