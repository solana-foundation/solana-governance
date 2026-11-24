"use client";

import { useKitTransactionSigner } from "@solana/connector/react";
import {
  appendTransactionMessageInstructions,
  createSolanaRpc,
  createTransactionMessage,
  getSignatureFromTransaction,
  pipe,
  sendTransactionWithoutConfirmingFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  type Instruction,
  type TransactionModifyingSigner,
} from "@solana/kit";
import { useCallback } from "react";
import { useEndpoint } from "@/contexts/EndpointContext";
import { confirmTransactionByPolling } from "@/lib/confirmTransaction";
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
 */
export function useSignTransaction() {
  const { endpointUrl } = useEndpoint();
  const { ready, signer } = useKitTransactionSigner();

  const signAndSend = useCallback(async (build: (input: { signer: TransactionModifyingSigner }) => Promise<readonly Instruction[]>) => {
    if (!signer) throw new Error("Wallet not connected");
    const rpc = createSolanaRpc(endpointUrl);
    const [instructions, { value: latestBlockhash }] = await Promise.all([
      build({ signer }),
      rpc.getLatestBlockhash({ commitment: "confirmed" }).send(),
    ]);
    const message = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(signer, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) => appendTransactionMessageInstructions([...instructions], m),
    );
    const signed = await signTransactionMessageWithSigners(message);
    await assertSignedByAccount(signed, signer.address);
    const signature = getSignatureFromTransaction(signed);
    await sendTransactionWithoutConfirmingFactory({ rpc })(signed, { commitment: "confirmed" });
    await confirmTransactionByPolling(rpc, signature, latestBlockhash.lastValidBlockHeight);
    return signature;
  }, [endpointUrl, signer]);

  return { ready: Boolean(signer && ready), signAndSend, signer };
}
