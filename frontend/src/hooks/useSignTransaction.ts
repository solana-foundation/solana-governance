"use client";

import { walletSupportsTransactionVersion } from "@solana/connector/headless";
import { useConnector, useKitTransactionSigner } from "@solana/connector/react";
import {
  assertIsTransactionWithinSizeLimit,
  createSolanaRpc,
  getSignatureFromTransaction,
  sendTransactionWithoutConfirmingFactory,
  signTransactionMessageWithSigners,
  type Instruction,
  type TransactionModifyingSigner,
} from "@solana/kit";
import { useCallback, useMemo } from "react";
import { useEndpoint } from "@/contexts/EndpointContext";
import { confirmTransactionByPolling } from "@/lib/confirmTransaction";
import { buildTransactionMessage } from "@/lib/transactionMessage";
import { rethrowAsTransactionFailure } from "@/lib/transactionErrors";
import { assertSignedByAccount } from "@/lib/walletSigning";

/**
 * Signs with the connected wallet and submits through the app's selected
 * endpoint.
 *
 * `signAndSend` builds a transaction from the instructions `build` returns,
 * has the wallet sign it, sends it, and resolves with its signature once it is
 * confirmed. The blockhash, submission and confirmation all use the same RPC
 * the instructions were built against, whatever cluster the wallet connector
 * itself is on.
 *
 * Transactions use the v1 format when the wallet declares it can sign v1, and
 * v0 otherwise. `options.computeUnitLimit` sets the compute-unit limit instead
 * of the simulated one, for instructions whose cost can grow before they run.
 *
 * A transaction that fails simulation or execution rejects with a
 * `TransactionFailedError` whose message states the reason.
 */
export function useSignTransaction() {
  const { endpointUrl } = useEndpoint();
  const { selectedWallet } = useConnector();
  const { ready, signer } = useKitTransactionSigner();
  // The connector's kit signer signs through `solana:signTransaction`, and a
  // wallet that has not declared v1 there rejects it.
  const version = useMemo(
    () =>
      selectedWallet && walletSupportsTransactionVersion(selectedWallet, 1, "solana:signTransaction")
        ? 1
        : 0,
    [selectedWallet],
  );

  const signAndSend = useCallback(async (
    build: (input: { signer: TransactionModifyingSigner }) => Promise<readonly Instruction[]>,
    options: { computeUnitLimit?: number } = {},
  ) => {
    if (!signer) throw new Error("Wallet not connected");
    const rpc = createSolanaRpc(endpointUrl);
    const [instructions, { value: latestBlockhash }] = await Promise.all([
      build({ signer }),
      rpc.getLatestBlockhash({ commitment: "confirmed" }).send(),
    ]);
    let message;
    try {
      message = await buildTransactionMessage({
        computeUnitLimit: options.computeUnitLimit,
        feePayer: signer,
        instructions,
        latestBlockhash,
        rpc,
        version,
      });
    } catch (error) {
      // Sizing a v1 transaction simulates it, which is where a failing
      // instruction is first reported.
      rethrowAsTransactionFailure(error);
    }
    const signed = await signTransactionMessageWithSigners(message);
    assertIsTransactionWithinSizeLimit(signed);
    await assertSignedByAccount(signed, signer.address);
    const signature = getSignatureFromTransaction(signed);
    try {
      await sendTransactionWithoutConfirmingFactory({ rpc })(signed, { commitment: "confirmed" });
      await confirmTransactionByPolling(rpc, signature, latestBlockhash.lastValidBlockHeight);
    } catch (error) {
      rethrowAsTransactionFailure(error);
    }
    return signature;
  }, [endpointUrl, signer, version]);

  return { ready: Boolean(signer && ready), signAndSend, signer };
}
