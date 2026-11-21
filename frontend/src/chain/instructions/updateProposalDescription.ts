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
  deriveVoteDelegationPda,
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

  const signer = wallet.publicKey;
  const program = createProgramWithWallet(wallet, blockchainParams.endpoint);
  const globalConfig = deriveGlobalConfigPda(program.programId);
  const config = await program.account.globalConfig.fetch(globalConfig);
  const description = assertValidProposalUrl(
    params.description,
    config.maxDescriptionLength,
  );
  if (!params.skipDocumentCheck) {
    await assertValidProposalDocument(description);
  }

  const proposal = new PublicKey(params.proposalId);
  const proposalAccount = await program.account.proposal.fetch(proposal);
  const voteDelegation = deriveVoteDelegationPda(
    proposalAccount.voteAccountPubkey,
    program.programId,
  );
  // With an accepted delegation on the proposal's vote account only the
  // delegate may update it; otherwise only the author (the identity) may.
  const delegation =
    await program.account.voteDelegation.fetchNullable(voteDelegation);
  if (delegation?.delegationAccepted) {
    if (!delegation.delegate.equals(signer)) {
      throw new Error(
        "Only the vote account's accepted vote delegate can update the description",
      );
    }
  } else if (!proposalAccount.author.equals(signer)) {
    throw new Error("Only the proposal author can update the description");
  }
  const instruction = await program.methods
    .updateProposalDescription(description)
    .accountsStrict({
      signer,
      proposal,
      voteDelegation,
      globalConfig,
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
