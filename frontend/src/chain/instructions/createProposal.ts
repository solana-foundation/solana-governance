import { PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  BlockchainParams,
  CreateProposalParams,
  TransactionResult,
} from "./types";
import {
  createProgramWithWallet,
  getVoteAccountProof,
  deriveProposalIndexPda,
  deriveGlobalConfigPda,
} from "./helpers";
import { deriveProposalAccount } from "../helpers";

/**
 * Creates a new governance proposal
 */
export async function createProposal(
  params: CreateProposalParams,
  blockchainParams: BlockchainParams,
  slot: number | undefined,
): Promise<TransactionResult> {
  const { title, description, seed, wallet } = params;
  if (!wallet || !wallet.publicKey) {
    throw new Error("Wallet not connected");
  }

  if (slot === undefined) {
    throw new Error("Slot is not defined");
  }

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

  const voteAccountProof = await getVoteAccountProof(
    validatorVoteAccount.votePubkey,
    blockchainParams.network,
    slot,
  );
  console.log("fetched voteAccountProof", voteAccountProof);

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
  transaction.feePayer = wallet.publicKey;
  transaction.recentBlockhash = (
    await program.provider.connection.getLatestBlockhash("confirmed")
  ).blockhash;

  const tx = await wallet.signTransaction(transaction);

  const signature = await program.provider.connection.sendRawTransaction(
    tx.serialize(),
  );

  return {
    signature,
    success: true,
  };
}
