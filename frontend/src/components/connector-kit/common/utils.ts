export const clusterColors: Record<string, string> = {
  'solana:mainnet': 'bg-green-500',
  'solana:devnet': 'bg-blue-500',
  'solana:testnet': 'bg-yellow-500',
  'solana:localnet': 'bg-red-500',
};

export function shortAddress(id: string) {
  return `${id.slice(0, 4)}...${id.slice(-4)}`;
}

export function getTransactionTitle(tx: {
  type: string;
  programName?: string;
  programId?: string;
}) {
  if (tx.type === 'tokenAccountClosed') return 'Token Account Closed';
  if (tx.type === 'program') {
    const program = tx.programName ?? (tx.programId ? shortAddress(tx.programId) : 'Unknown');
    return `Program: ${program}`;
  }
  return tx.type;
}

export function getTransactionSubtitle(tx: {
  type: string;
  formattedTime: string;
  instructionTypes?: string[];
}) {
  if (tx.type === 'program' && tx.instructionTypes?.length) {
    const summary = tx.instructionTypes.slice(0, 2).join(' · ');
    return `${tx.formattedTime} · ${summary}`;
  }
  return tx.formattedTime;
}

export function getInstallUrl(walletName: string, walletUrl?: string): string | undefined {
  if (walletUrl) return walletUrl;

  const name = walletName.toLowerCase();
  if (name.includes('phantom')) return 'https://phantom.app';
  if (name.includes('solflare')) return 'https://solflare.com';
  if (name.includes('backpack')) return 'https://backpack.app';
  if (name.includes('glow')) return 'https://glow.app';
  if (name.includes('coinbase')) return 'https://www.coinbase.com/wallet';
  if (name.includes('ledger')) return 'https://www.ledger.com';
  if (name.includes('trust')) return 'https://trustwallet.com';
  if (name.includes('exodus')) return 'https://www.exodus.com';

  return undefined;
}
