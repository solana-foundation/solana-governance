import {
  getPublicKeyFromAddress,
  verifySignature,
  type Address,
  type Transaction,
} from "@solana/kit";

/** User-facing message for a wallet approval the user chose not to complete. */
export const WALLET_SIGNING_CANCELLED_MESSAGE = "Transaction approval cancelled";

const WALLET_SIGNING_ERROR_NAMES = new Set([
  "WalletSignTransactionError",
  "WalletSignAllTransactionsError",
  "WalletSendTransactionError",
]);

const WALLET_CANCELLATION_MESSAGE =
  /approval denied|popup closed|user (?:rejected|denied)|request rejected|(?:transaction|request) (?:cancelled|canceled)/i;

/** Enough to reach a wallet's error through the connector's own wrappers. */
const MAX_ERROR_CHAIN_DEPTH = 5;

/** Thrown when the wallet cannot provide the signature required by a transaction. */
export class WalletSigningError extends Error {
  name = "WalletSigningError";
}

/**
 * Determines whether an error represents a wallet approval the user cancelled.
 *
 * Wallets expose cancellation differently, but Wallet Standard uses a named signing error and
 * EIP-1193-compatible providers commonly use code 4001. The wallet connector reports a failed
 * signature as its own `SIGNING_FAILED` error with the wallet's error attached as
 * `originalError`, so the wrapped errors are inspected as well. The message match is
 * deliberately limited to known user-cancellation wording so genuine wallet failures remain
 * reportable.
 *
 * @param error - Error returned by a wallet, or by the connector on its behalf.
 * @returns Whether the error is an expected user cancellation.
 */
export function isWalletSigningCancellation(error: unknown): boolean {
  let current = error;
  let wrappedBySigningFailure = false;

  for (let depth = 0; depth < MAX_ERROR_CHAIN_DEPTH; depth += 1) {
    if (typeof current !== "object" || current === null) return false;

    const { cause, code, message, name, originalError } = current as Record<
      string,
      unknown
    >;
    if (code === 4001 || code === "4001") return true;

    const isSigningError =
      wrappedBySigningFailure ||
      (typeof name === "string" && WALLET_SIGNING_ERROR_NAMES.has(name));
    if (
      isSigningError &&
      typeof message === "string" &&
      WALLET_CANCELLATION_MESSAGE.test(message)
    ) {
      return true;
    }

    wrappedBySigningFailure ||= code === "SIGNING_FAILED";
    current = originalError ?? cause;
  }

  return false;
}

/**
 * Verifies that a signed transaction carries a valid signature from `signer`.
 *
 * A wallet can sign with a different account than the one the transaction was built for. The
 * RPC would reject that with an opaque signature-verification failure, so it is reported here
 * in terms the user can act on. Browsers that cannot verify Ed25519 signatures skip the check
 * and leave it to the RPC.
 *
 * @throws {WalletSigningError} If the signature is missing or was not made by `signer`.
 */
export async function assertSignedByAccount(
  transaction: Transaction,
  signer: Address,
): Promise<void> {
  const signature = transaction.signatures[signer];
  if (!signature) {
    throw new WalletSigningError(
      "Your wallet did not sign the transaction with the connected account. Reconnect the account that started this transaction and try again.",
    );
  }

  let isValid: boolean;
  try {
    const publicKey = await getPublicKeyFromAddress(signer);
    isValid = await verifySignature(
      publicKey,
      signature,
      transaction.messageBytes,
    );
  } catch {
    return;
  }

  if (!isValid) {
    throw new WalletSigningError(
      "Your wallet signed with a different account than the one that started this transaction. Reconnect the original account and try again.",
    );
  }
}
