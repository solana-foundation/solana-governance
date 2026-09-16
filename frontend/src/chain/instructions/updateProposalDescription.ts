import { PublicKey, Transaction } from "@solana/web3.js";
import {
  BlockchainParams,
  TransactionResult,
  UpdateProposalDescriptionParams,
} from "./types";
import {
  createProgramWithWallet,
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
  transaction.recentBlockhash = (
    await program.provider.connection.getLatestBlockhash("confirmed")
  ).blockhash;
  const signed = await signTransactionForWallet(wallet, transaction, signer);
  const signature = await program.provider.connection.sendRawTransaction(
    signed.serialize(),
  );
  return { signature, success: true };
}
