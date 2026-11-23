"use client";

import {
  useKitTransactionSigner,
  useSolanaClient,
  useTransactionPreparer,
} from "@solana/connector/react";
import {
  appendTransactionMessageInstructions,
  assertIsTransactionWithBlockhashLifetime,
  createTransactionMessage,
  getSignatureFromTransaction,
  sendTransactionWithoutConfirmingFactory,
  setTransactionMessageFeePayerSigner,
  signTransactionMessageWithSigners,
  type Instruction,
  type TransactionModifyingSigner,
} from "@solana/kit";
import { useCallback } from "react";
import { confirmTransactionByPolling } from "@/lib/confirmTransaction";

export function useSignTransaction() {
  const { client } = useSolanaClient();
  const { prepare, ready: prepareReady } = useTransactionPreparer();
  const { ready: signerReady, signer } = useKitTransactionSigner();

  const signAndSend = useCallback(async (build: (input: { signer: TransactionModifyingSigner }) => Promise<readonly Instruction[]>) => {
    if (!client || !signer) throw new Error("Wallet not connected");
    const instructions = await build({ signer });
    const message = appendTransactionMessageInstructions(
      [...instructions],
      setTransactionMessageFeePayerSigner(signer, createTransactionMessage({ version: 0 })),
    );
    const prepared = await prepare(message);
    const signed = await signTransactionMessageWithSigners(prepared);
    assertIsTransactionWithBlockhashLifetime(signed);
    const signature = getSignatureFromTransaction(signed);
    await sendTransactionWithoutConfirmingFactory({ rpc: client.rpc })(signed, { commitment: "confirmed" });
    await confirmTransactionByPolling(client.rpc, signature, signed.lifetimeConstraint.lastValidBlockHeight);
    return signature;
  }, [client, prepare, signer]);

  return { ready: Boolean(client && signer && signerReady && prepareReady), signAndSend, signer };
}
