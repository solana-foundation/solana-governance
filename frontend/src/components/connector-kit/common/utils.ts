export function shortAddress(id: string) {
  return `${id.slice(0, 4)}...${id.slice(-4)}`;
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
