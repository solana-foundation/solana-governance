import { PublicKey, Transaction } from "@solana/web3.js";
import {
  BlockchainParams,
  FinalizeProposalParams,
  TransactionResult,
} from "./types";
import {
  confirmTransactionByPolling,
  createProgramWithWallet,
  signTransactionForWallet,
} from "./helpers";

/**
 * Finalizes a governance proposal
 */
export async function finalizeProposal(
  params: FinalizeProposalParams,
  blockchainParams: BlockchainParams,
): Promise<TransactionResult> {
  try {
    const { proposalId, wallet } = params;

    if (!wallet || !wallet.publicKey) {
      throw new Error("Wallet not connected");
    }

    const proposalPubkey = new PublicKey(proposalId);
    const program = createProgramWithWallet(wallet, blockchainParams.endpoint);
    const signer = wallet.publicKey;

    const finalizeInstruction = await program.methods
      .finalizeProposal()
      .accounts({
        signer,
        proposal: proposalPubkey,
      })
      .instruction();

    const transaction = new Transaction().add(finalizeInstruction);
    transaction.feePayer = signer;
    const latestBlockhash = await program.provider.connection.getLatestBlockhash(
      "confirmed",
    );
    transaction.recentBlockhash = latestBlockhash.blockhash;
    transaction.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

    const signedTransaction = await signTransactionForWallet(
      wallet,
      transaction,
      signer,
    );
    const signature = await program.provider.connection.sendRawTransaction(
      signedTransaction.serialize(),
      { preflightCommitment: "confirmed" },
    );
    const confirmation = await confirmTransactionByPolling(
      program.provider.connection,
      signature,
      latestBlockhash.lastValidBlockHeight,
    );
    if (confirmation.value.err) {
      throw new Error(
        `Failed to finalize proposal: ${JSON.stringify(confirmation.value.err)}`,
      );
    }

    return {
      signature,
      success: true,
    };
  } catch (error) {
    console.error("Error finalizing proposal:", error);
    return {
      signature: "",
      success: false,
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
}
