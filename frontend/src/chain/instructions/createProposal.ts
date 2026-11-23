import { SystemProgram, Transaction } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  BlockchainParams,
  CreateProposalParams,
  TransactionResult,
} from "./types";
import {
  createProgramWithWallet,
  deriveProposalIndexPda,
  deriveGlobalConfigPda,
  deriveVoteDelegationPda,
  resolveSignerVoteAccount,
  signTransactionForWallet,
  confirmTransactionByPolling,
} from "./helpers";
import { deriveProposalAccount } from "../helpers";
import {
  assertValidProposalDocument,
  assertValidProposalUrl,
} from "@/lib/github";

/**
 * Creates a new governance proposal
 */
export async function createProposal(
  params: CreateProposalParams,
  blockchainParams: BlockchainParams,
): Promise<TransactionResult> {
  const { title, seed, wallet } = params;
  if (!wallet || !wallet.publicKey) {
    throw new Error("Wallet not connected");
  }

  // The transaction is built and signed for this account. signTransactionForWallet verifies
  // that the wallet-returned transaction contains its signature before the proposal is submitted.
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

  // Generate random seed if not provided
  const seedValue = new BN(
    seed ?? Math.floor(Math.random() * Number.MAX_SAFE_INTEGER),
  );

  // The wallet is either the validator identity or its accepted vote delegate.
  const splVoteAccount = await resolveSignerVoteAccount(program, signer);
  const proposalPda = deriveProposalAccount(program, seedValue, splVoteAccount);

  // Build and send transaction using accountsPartial like in tests
  const proposalInstruction = await program.methods
    .createProposal(seedValue, title, description)
    .accountsStrict({
      signer: wallet.publicKey,
      proposal: proposalPda,
      splVoteAccount,
      voteDelegation: deriveVoteDelegationPda(splVoteAccount, program.programId),
      systemProgram: SystemProgram.programId,
      proposalIndex: deriveProposalIndexPda(program.programId),
      globalConfig,
    })
    .instruction();

  const transaction = new Transaction();
  transaction.add(proposalInstruction);
  transaction.feePayer = signer;
  const latestBlockhash =
    await program.provider.connection.getLatestBlockhash("confirmed");
  transaction.recentBlockhash = latestBlockhash.blockhash;
  transaction.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

  const tx = await signTransactionForWallet(wallet, transaction, signer);

  const signature = await program.provider.connection.sendRawTransaction(
    tx.serialize(),
  );
  const confirmation = await confirmTransactionByPolling(
    program.provider.connection,
    signature,
    latestBlockhash.lastValidBlockHeight,
  );
  if (confirmation.value.err) {
    throw new Error(
      `Failed to create proposal: ${JSON.stringify(confirmation.value.err)}`,
    );
  }

  return {
    signature,
    success: true,
  };
}
