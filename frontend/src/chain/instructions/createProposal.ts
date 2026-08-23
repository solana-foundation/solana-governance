import { PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
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
  signTransactionForWallet,
} from "./helpers";
import { deriveProposalAccount } from "../helpers";
import { assertValidProposalUrl } from "@/lib/github";

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

  // Enforced here rather than only in the modal so every caller inherits it. The on-chain
  // check accepts anything github.com-shaped, including pull request links, which the
  // frontend then cannot resolve to a proposal document.
  //
  // The normalized URL is what gets encoded below: validation trims, and the program requires
  // a literal https://github.com/ prefix, so sending the raw input would be rejected on chain
  // after the frontend had already accepted it.
  const description = assertValidProposalUrl(params.description);

  // Generate random seed if not provided
  const seedValue = new BN(
    seed ?? Math.floor(Math.random() * Number.MAX_SAFE_INTEGER),
  );

  const program = createProgramWithWallet(wallet, blockchainParams.endpoint);

  const voteAccounts = await program.provider.connection.getVoteAccounts();
  const validatorVoteAccount = voteAccounts.current.find(
    (acc) => acc.nodePubkey === wallet.publicKey.toBase58(),
  );

  if (!validatorVoteAccount) {
    throw new Error(
      `No SPL vote account found for validator identity ${wallet.publicKey.toBase58()}`,
    );
  }

  const splVoteAccount = new PublicKey(validatorVoteAccount.votePubkey);
  const proposalPda = deriveProposalAccount(program, seedValue, splVoteAccount);

  // Build and send transaction using accountsPartial like in tests
  const proposalInstruction = await program.methods
    .createProposal(seedValue, title, description)
    .accountsStrict({
      signer: wallet.publicKey,
      proposal: proposalPda,
      splVoteAccount,
      systemProgram: SystemProgram.programId,
      proposalIndex: deriveProposalIndexPda(program.programId),
      globalConfig: deriveGlobalConfigPda(program.programId),
    })
    .instruction();

  const transaction = new Transaction();
  transaction.add(proposalInstruction);
  transaction.feePayer = signer;
  transaction.recentBlockhash = (
    await program.provider.connection.getLatestBlockhash("confirmed")
  ).blockhash;

  const tx = await signTransactionForWallet(wallet, transaction, signer);

  const signature = await program.provider.connection.sendRawTransaction(
    tx.serialize(),
  );

  return {
    signature,
    success: true,
  };
}
