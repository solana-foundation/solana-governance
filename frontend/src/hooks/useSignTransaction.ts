"use client";

import {
  useKitTransactionSigner,
  useSolanaClient,
  useTransactionPreparer,
} from "@solana/connector/react";
import {
  appendTransactionMessageInstructions,
  createTransactionMessage,
  getSignatureFromTransaction,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  signTransactionMessageWithSigners,
  type Instruction,
  type TransactionModifyingSigner,
  type TransactionWithBlockhashLifetime,
} from "@solana/kit";
import { useCallback } from "react";

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
    await sendAndConfirmTransactionFactory({
      rpc: client.rpc,
      rpcSubscriptions: client.rpcSubscriptions,
    })(signed as typeof signed & TransactionWithBlockhashLifetime, { commitment: "confirmed" });
    return getSignatureFromTransaction(signed);
  }, [client, prepare, signer]);

  return { ready: Boolean(client && signer && signerReady && prepareReady), signAndSend, signer };
}
