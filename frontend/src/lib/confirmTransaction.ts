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

/**
 * Waits until a submitted transaction reaches `confirmed` commitment by polling
 * `getSignatureStatuses` over HTTP. The app's RPC proxy exposes no WebSocket
 * endpoint, so signature subscriptions are unavailable.
 *
 * @throws A `SolanaError` describing the transaction error if it failed, or
 * `SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED` once the block height passes
 * `lastValidBlockHeight` without the transaction confirming.
 */
export async function confirmTransactionByPolling(
  rpc: Rpc<GetBlockHeightApi & GetSignatureStatusesApi>,
  signature: Signature,
  lastValidBlockHeight: bigint,
): Promise<void> {
  while (true) {
    const {
      value: [status],
    } = await rpc.getSignatureStatuses([signature]).send();
    if (status) {
      if (status.err) throw getSolanaErrorFromTransactionError(status.err);
      if (
        status.confirmationStatus === "confirmed" ||
        status.confirmationStatus === "finalized"
      ) {
        return;
      }
    }

    const currentBlockHeight = await rpc
      .getBlockHeight({ commitment: "confirmed" })
      .send();
    if (currentBlockHeight > lastValidBlockHeight) {
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
