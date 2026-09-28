import { PublicKey, Transaction } from "@solana/web3.js";
import {
  BlockchainParams,
  TransactionResult,
  UpdateProposalDescriptionParams,
} from "./types";
import {
  createProgramWithWallet,
  confirmTransactionByPolling,
  deriveGlobalConfigPda,
  signTransactionForWallet,
} from "./helpers";
import {
  assertValidProposalDocument,
  assertValidProposalUrl,
} from "@/lib/github";

/** Updates an eligible proposal's commit-pinned document URL. */
export async function updateProposalDescription(
  params: UpdateProposalDescriptionParams,
  blockchainParams: BlockchainParams,
): Promise<TransactionResult> {
  const { wallet } = params;
  if (!wallet?.publicKey) throw new Error("Wallet not connected");

  const description = assertValidProposalUrl(params.description);
  if (!params.skipDocumentCheck) {
    await assertValidProposalDocument(description);
  }

  const signer = wallet.publicKey;
  const program = createProgramWithWallet(wallet, blockchainParams.endpoint);
  const proposal = new PublicKey(params.proposalId);
  const proposalAccount = await program.account.proposal.fetch(proposal);
  if (proposalAccount.author.toBase58() !== signer.toBase58()) {
    throw new Error("Only the proposal author can update the description");
  }
  const instruction = await program.methods
    .updateProposalDescription(description)
    .accountsStrict({
      signer,
      proposal,
      globalConfig: deriveGlobalConfigPda(program.programId),
    })
    .instruction();

  const transaction = new Transaction().add(instruction);
  transaction.feePayer = signer;
  const latestBlockhash =
    await program.provider.connection.getLatestBlockhash("confirmed");
  transaction.recentBlockhash = latestBlockhash.blockhash;
  transaction.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
  const signed = await signTransactionForWallet(wallet, transaction, signer);
  const signature = await program.provider.connection.sendRawTransaction(
    signed.serialize(),
  );
  const confirmation = await confirmTransactionByPolling(
    program.provider.connection,
    signature,
    latestBlockhash.lastValidBlockHeight,
  );
  if (confirmation.value.err) {
    throw new Error(
      `Failed to update proposal document: ${JSON.stringify(confirmation.value.err)}`,
    );
  }
  return { signature, success: true };
}
