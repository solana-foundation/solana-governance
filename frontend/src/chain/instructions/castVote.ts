import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  Transaction,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  BlockchainParams,
  CastVoteParams,
  SNAPSHOT_PROGRAM_ID,
  TransactionResult,
} from "./types";
import {
  createProgramWithWallet,
  deriveVotePda,
  deriveVoteOverrideCachePda,
  validateVoteBasisPoints,
  createGovV1ProgramWithWallet,
  getVoteAccountProof,
  getMetaMerkleProofPda,
  computeProofCloseTimestamp,
} from "./helpers";

/**
 * Casts a vote on a governance proposal
 */
export async function castVote(
  params: CastVoteParams,
  blockchainParams: BlockchainParams,
  slot: number | undefined
): Promise<TransactionResult> {
  const {
    proposalId,
    forVotesBp,
    againstVotesBp,
    abstainVotesBp,
    wallet,
    consensusResult,
  } = params;

  if (!wallet || !wallet.publicKey) {
    throw new Error("Wallet not connected");
  }

  if (slot === undefined) {
    throw new Error("Slot is not defined");
  }

  if (consensusResult === undefined) {
    throw new Error("Consensus result not defined");
  }

  // Validate vote distribution
  validateVoteBasisPoints(forVotesBp, againstVotesBp, abstainVotesBp);

  const proposalPubkey = new PublicKey(proposalId);
  const program = createProgramWithWallet(wallet, blockchainParams.endpoint);

  const voteAccounts = await program.provider.connection.getVoteAccounts();
  const validatorVoteAccount = voteAccounts.current.find(
    (acc) => acc.nodePubkey === wallet.publicKey.toBase58()
  );

  if (!validatorVoteAccount) {
    throw new Error(
      `No SPL vote account found for validator identity ${wallet.publicKey.toBase58()}`
    );
  }

  const splVoteAccount = new PublicKey(validatorVoteAccount.votePubkey);

  // Derive vote PDA - based on IDL, it uses proposal and vote account
  const votePda = deriveVotePda(
    proposalPubkey,
    splVoteAccount,
    program.programId
  );
  const voteOverrideCachePda = deriveVoteOverrideCachePda(
    proposalPubkey,
    votePda,
    program.programId
  );

  const govV1Program = createGovV1ProgramWithWallet(
    wallet,
    blockchainParams.endpoint
  );

  const voteAccountProof = await getVoteAccountProof(
    validatorVoteAccount.votePubkey,
    blockchainParams.network,
    slot,
    blockchainParams.ncnApiUrl
  );

  const metaMerkleProofPda = getMetaMerkleProofPda(
    voteAccountProof,
    SNAPSHOT_PROGRAM_ID,
    consensusResult
  );

  const merkleAccountInfo = await program.provider.connection.getAccountInfo(
    metaMerkleProofPda,
    "confirmed"
  );

  const instructions: TransactionInstruction[] = [];

  if (!merkleAccountInfo) {
    // Set close_timestamp to the proposal's vote expiry so the proof cannot be closed
    // permissionlessly while voting is open. See computeProofCloseTimestamp.
    const proposalAccount =
      await program.account.proposal.fetch(proposalPubkey);
    const closeTimestamp = await computeProofCloseTimestamp(
      program.provider.connection,
      proposalAccount.endEpoch.toNumber()
    );

    const initMerkleInstruction = await govV1Program.methods
      .initMetaMerkleProof(
        {
          votingWallet: new PublicKey(
            voteAccountProof.meta_merkle_leaf.voting_wallet
          ),
          voteAccount: new PublicKey(
            voteAccountProof.meta_merkle_leaf.vote_account
          ),
          stakeMerkleRoot: Array.from(
            new PublicKey(
              voteAccountProof.meta_merkle_leaf.stake_merkle_root
            ).toBytes()
          ),
          activeStake: new BN(voteAccountProof.meta_merkle_leaf.active_stake),
        },
        voteAccountProof.meta_merkle_proof.map((proof) =>
          Array.from(new PublicKey(proof).toBytes())
        ),
        new BN(closeTimestamp)
      )
      .accountsStrict({
        consensusResult,
        merkleProof: metaMerkleProofPda,
        payer: wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    instructions.push(initMerkleInstruction);
  }

  // Build cast vote instruction
  const castVoteInstruction = await program.methods
    .castVote(
      new BN(forVotesBp),
      new BN(againstVotesBp),
      new BN(abstainVotesBp)
    )
    .accountsStrict({
      signer: wallet.publicKey,
      proposal: proposalPubkey,
      vote: votePda,
      splVoteAccount: splVoteAccount,
      snapshotProgram: SNAPSHOT_PROGRAM_ID,
      consensusResult,
      metaMerkleProof: metaMerkleProofPda,
      systemProgram: SystemProgram.programId,
      voteOverrideCache: voteOverrideCachePda,
    })
    .instruction();

  instructions.push(castVoteInstruction);

  const transaction = new Transaction();
  transaction.add(...instructions);
  transaction.feePayer = wallet.publicKey;
  transaction.recentBlockhash = (
    await program.provider.connection.getLatestBlockhash("confirmed")
  ).blockhash;

  const tx = await wallet.signTransaction(transaction);

  const signature = await program.provider.connection.sendRawTransaction(
    tx.serialize()
  );

  console.log("signature cast vote", signature);

  return {
    signature,
    success: true,
  };
}
