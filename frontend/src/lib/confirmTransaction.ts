import {
  getSolanaErrorFromTransactionError,
  SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED,
  SolanaError,
  type GetBlockHeightApi,
  type GetSignatureStatusesApi,
  type Rpc,
  type Signature,
} from "@solana/kit";

const CONFIRMATION_POLL_INTERVAL_MS = 1_000;
// A blockhash stays valid for roughly a minute, so an RPC that has been
// unreachable this long can no longer say whether the transaction landed.
const RPC_UNREACHABLE_TIMEOUT_MS = 90_000;

/**
 * Thrown when the RPC stopped answering before a submitted transaction's
 * outcome was known. The transaction may still have been processed, so it must
 * not be treated as failed.
 */
export class TransactionOutcomeUnknownError extends Error {
  constructor(
    readonly signature: Signature,
    options?: ErrorOptions,
  ) {
    super(
      `Could not confirm transaction ${signature} because the RPC stopped responding. It may still have been processed, so check it in an explorer before trying again.`,
      options,
    );
    this.name = "TransactionOutcomeUnknownError";
  }
}

/**
 * Waits until a submitted transaction reaches `confirmed` commitment by polling
 * `getSignatureStatuses` over HTTP. The app's RPC proxy exposes no WebSocket
 * endpoint, so signature subscriptions are unavailable. A poll that fails is
 * retried on the next interval.
 *
 * @throws A `SolanaError` describing the transaction error if it failed,
 * `SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED` once the block height passes
 * `lastValidBlockHeight` without the transaction confirming, or
 * `TransactionOutcomeUnknownError` if every poll fails for 90 seconds.
 */
export async function confirmTransactionByPolling(
  rpc: Rpc<GetBlockHeightApi & GetSignatureStatusesApi>,
  signature: Signature,
  lastValidBlockHeight: bigint,
): Promise<void> {
  let lastAnsweredAt = Date.now();
  while (true) {
    let status;
    let currentBlockHeight;
    try {
      ({
        value: [status],
      } = await rpc.getSignatureStatuses([signature]).send());
      if (!status) {
        currentBlockHeight = await rpc
          .getBlockHeight({ commitment: "confirmed" })
          .send();
      }
      lastAnsweredAt = Date.now();
    } catch (error) {
      // The transaction is already submitted, so a rate-limited or failed poll
      // says nothing about its outcome.
      if (Date.now() - lastAnsweredAt > RPC_UNREACHABLE_TIMEOUT_MS) {
        throw new TransactionOutcomeUnknownError(signature, { cause: error });
      }
    }

    if (status) {
      if (status.err) throw getSolanaErrorFromTransactionError(status.err);
      if (
        status.confirmationStatus === "confirmed" ||
        status.confirmationStatus === "finalized"
      ) {
        return;
      }
    } else if (
      currentBlockHeight !== undefined &&
      currentBlockHeight > lastValidBlockHeight
    ) {
      throw new SolanaError(SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED, {
        currentBlockHeight,
        lastValidBlockHeight,
      });
    }

    await new Promise((resolve) =>
      setTimeout(resolve, CONFIRMATION_POLL_INTERVAL_MS),
    );
  }
}
