import {
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  CastVoteOverrideParams,
  TransactionResult,
  BlockchainParams,
  SNAPSHOT_PROGRAM_ID,
} from "./types";
import {
  createProgramWithWallet,
  createGovV1ProgramWithWallet,
  getVoteAccountProof,
  getStakeAccountProof,
  resolveSnapshotVoteAccount,
  assertOverrideProofLineage,
  convertMerkleProofStrings,
  convertStakeMerkleLeafDataToIdlType,
  validateVoteBasisPoints,
  deriveVotePda,
  deriveVoteOverridePda,
  deriveVoteOverrideCachePda,
  getMetaMerkleProofPda,
  computeProofCloseTimestamp,
} from "./helpers";
import { BN } from "@coral-xyz/anchor";

/**
 * Casts a vote override using a stake account
 */
export async function castVoteOverride(
  params: CastVoteOverrideParams,
  blockchainParams: BlockchainParams,
  slot: number | undefined
): Promise<TransactionResult> {
  const {
    proposalId,
    forVotesBp,
    againstVotesBp,
    abstainVotesBp,
    stakeAccount,
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

  const stakeAccountPubkey = new PublicKey(stakeAccount);

  // Get proofs. Fetch the stake proof first: its `vote_account` is the validator the stake was
  // delegated to AT SNAPSHOT TIME. We must derive everything (the meta proof, the spl_vote_account
  // and the validator_vote / vote_override PDAs) from this snapshot validator, not the live
  // on-chain delegation. If the delegator redelegated after the snapshot, the live vote account
  // would pair this stake proof with the wrong validator's meta proof and the override would fail
  // on-chain even though the delegator was eligible at snapshot time.
  const network = blockchainParams.network;
  const stakeMerkleProof = await getStakeAccountProof(
    stakeAccount,
    network,
    slot,
    blockchainParams.ncnApiUrl
  );
  const splVoteAccount = resolveSnapshotVoteAccount(stakeMerkleProof);
  const metaMerkleProof = await getVoteAccountProof(
    splVoteAccount.toBase58(),
    network,
    slot,
    blockchainParams.ncnApiUrl
  );
  assertOverrideProofLineage(stakeMerkleProof, metaMerkleProof);

  const metaMerkleProofPda = getMetaMerkleProofPda(
    metaMerkleProof,
    SNAPSHOT_PROGRAM_ID,
    consensusResult
  );

  // Check if merkle account exists
  const merkleAccountInfo = await program.provider.connection.getAccountInfo(
    metaMerkleProofPda,
    "confirmed"
  );

  // Instructions are sent in a single atomic transaction so the proof cannot be deleted by an
  // attacker between creating it and consuming it in the override vote.
  const instructions: TransactionInstruction[] = [];

  if (!merkleAccountInfo) {
    const govV1Program = createGovV1ProgramWithWallet(
      wallet,
      blockchainParams.endpoint
    );

    const stakeMerkleRootData = Array.from(
      new PublicKey(
        metaMerkleProof.meta_merkle_leaf.stake_merkle_root
      ).toBytes()
    );

    const metaMerkleProofData = metaMerkleProof.meta_merkle_proof.map((proof) =>
      Array.from(new PublicKey(proof).toBytes())
    );

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
            metaMerkleProof.meta_merkle_leaf.voting_wallet
          ),
          voteAccount: new PublicKey(
            metaMerkleProof.meta_merkle_leaf.vote_account
          ),
          stakeMerkleRoot: stakeMerkleRootData,
          activeStake: new BN(
            `${metaMerkleProof.meta_merkle_leaf.active_stake}`
          ),
        },
        metaMerkleProofData,
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

  // Convert merkle proof data
  const stakeMerkleProofVec = convertMerkleProofStrings(
    stakeMerkleProof.stake_merkle_proof
  );
  const stakeMerkleLeaf = convertStakeMerkleLeafDataToIdlType(
    stakeMerkleProof.stake_merkle_leaf
  );

  const forVotesBn = new BN(forVotesBp);
  const againstVotesBn = new BN(againstVotesBp);
  const abstainVotesBn = new BN(abstainVotesBp);

  const votePda = deriveVotePda(
    proposalPubkey,
    splVoteAccount,
    program.programId
  );

  const voteOverridePda = deriveVoteOverridePda(
    proposalPubkey,
    stakeAccountPubkey,
    votePda,
    program.programId
  );

  const voteOverrideCachePda = deriveVoteOverrideCachePda(
    proposalPubkey,
    votePda,
    program.programId
  );

  // Build cast vote override instruction
  const castVoteOverrideInstruction = await program.methods
    .castVoteOverride(
      forVotesBn,
      againstVotesBn,
      abstainVotesBn,
      stakeMerkleProofVec,
      stakeMerkleLeaf
    )
    .accountsStrict({
      signer: wallet.publicKey,
      splVoteAccount: splVoteAccount,
      splStakeAccount: stakeAccountPubkey,
      proposal: proposalPubkey,
      consensusResult,
      metaMerkleProof: metaMerkleProofPda,
      snapshotProgram: SNAPSHOT_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      voteOverride: voteOverridePda,
      voteOverrideCache: voteOverrideCachePda,
      validatorVote: votePda,
    })
    .instruction();

  instructions.push(castVoteOverrideInstruction);

  const transaction = new Transaction();
  transaction.add(...instructions);
  transaction.feePayer = wallet.publicKey;
  transaction.recentBlockhash = (
    await program.provider.connection.getLatestBlockhash("confirmed")
  ).blockhash;

  const tx = await wallet.signTransaction(transaction);

  const signature = await program.provider.connection.sendRawTransaction(
    tx.serialize(),
    { preflightCommitment: "confirmed" }
  );
  console.log("signature cast vote override", signature);
  return {
    signature,
    success: true,
  };
}
