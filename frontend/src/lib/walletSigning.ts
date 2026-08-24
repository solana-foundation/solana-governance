/** User-facing message for a wallet approval the user chose not to complete. */
export const WALLET_SIGNING_CANCELLED_MESSAGE = "Transaction approval cancelled";

const WALLET_SIGNING_ERROR_NAMES = new Set([
  "WalletSignTransactionError",
  "WalletSignAllTransactionsError",
  "WalletSendTransactionError",
]);

const WALLET_CANCELLATION_MESSAGE =
  /approval denied|popup closed|user (?:rejected|denied)|request rejected|(?:transaction|request) (?:cancelled|canceled)/i;

/**
 * Determines whether an error represents a wallet approval the user cancelled.
 *
 * Wallets expose cancellation differently, but Wallet Standard uses a named signing error and
 * EIP-1193-compatible providers commonly use code 4001. The message match is deliberately
 * limited to known user-cancellation wording so genuine wallet failures remain reportable.
 *
 * @param error - Error returned by a wallet adapter or provider.
 * @returns Whether the error is an expected user cancellation.
 */
export function isWalletSigningCancellation(error: unknown): boolean {
  if (typeof error !== "object" || error === null) return false;

  const { code, message, name } = error as Record<string, unknown>;
  if (code === 4001 || code === "4001") return true;

  return (
    typeof name === "string" &&
    WALLET_SIGNING_ERROR_NAMES.has(name) &&
    typeof message === "string" &&
    WALLET_CANCELLATION_MESSAGE.test(message)
  );
}
