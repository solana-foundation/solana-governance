import {
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  ModifyVoteOverrideParams,
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
  resolveProposalSnapshotSlot,
  convertMerkleProofStrings,
  convertStakeMerkleLeafDataToIdlType,
  validateVoteBasisPoints,
  deriveVoteOverridePda,
  deriveVoteOverrideCachePda,
  deriveVotePda,
  getMetaMerkleProofPda,
  isMetaMerkleProofInitialized,
  computeProofCloseTimestamp,
  signTransactionForWallet,
  confirmTransactionByPolling,
} from "./helpers";
import { BN } from "@coral-xyz/anchor";

/**
 * Modifies an existing vote override using a stake account
 */
export async function modifyVoteOverride(
  params: ModifyVoteOverrideParams,
  blockchainParams: BlockchainParams
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

  // Both transactions require this same account. signTransactionForWallet verifies that the
  // wallet-returned transaction contains its signature before either transaction is submitted.
  const signer = wallet.publicKey;

  if (consensusResult === undefined) {
    throw new Error("Consensus result not defined");
  }

  // Validate vote distribution
  validateVoteBasisPoints(forVotesBp, againstVotesBp, abstainVotesBp);

  const proposalPubkey = new PublicKey(proposalId);
  const program = createProgramWithWallet(wallet, blockchainParams.endpoint);
  const proposalAccount = await program.account.proposal.fetch(proposalPubkey);
  const slot = resolveProposalSnapshotSlot(proposalAccount.snapshotSlot);

  const stakeAccountPubkey = new PublicKey(stakeAccount);

  // Get proofs. Fetch the stake proof first: its `vote_account` is the validator the stake was
  // delegated to AT SNAPSHOT TIME. We must derive everything (the meta proof, the spl_vote_account
  // and the validator_vote / vote_override PDAs) from this snapshot validator, not the live
  // on-chain delegation. If the delegator redelegated after the snapshot, the live vote account
  // would pair this stake proof with the wrong validator's meta proof and the override would fail
  // on-chain even though the delegator was eligible at snapshot time.
  const network = blockchainParams.network || "mainnet";
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

  // Every delegator under this validator shares one proof account, so it may
  // already exist.
  const proofInitialized = await isMetaMerkleProofInitialized(
    program.provider.connection,
    metaMerkleProofPda
  );

  if (!proofInitialized) {
    console.log("merkle proof does not exist, initializing");
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
        payer: signer,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    // The meta proof and stake proof are both large. Combining their instructions can exceed
    // Solana's 1,232-byte transaction limit, so initialize and confirm the reusable meta proof
    // before building the override transaction. Its close timestamp is the proposal's vote
    // expiry, which prevents permissionless deletion between these transactions.
    const initBlockhash =
      await program.provider.connection.getLatestBlockhash("confirmed");
    const initTransaction = new Transaction();
    initTransaction.add(initMerkleInstruction);
    initTransaction.feePayer = signer;
    initTransaction.recentBlockhash = initBlockhash.blockhash;
    initTransaction.lastValidBlockHeight = initBlockhash.lastValidBlockHeight;

    try {
      const signedInitTransaction = await signTransactionForWallet(
        wallet,
        initTransaction,
        signer
      );
      const initSignature =
        await program.provider.connection.sendRawTransaction(
          signedInitTransaction.serialize(),
          { preflightCommitment: "confirmed" }
        );
      const initConfirmation = await confirmTransactionByPolling(
        program.provider.connection,
        initSignature,
        initBlockhash.lastValidBlockHeight
      );

      if (initConfirmation.value.err) {
        throw new Error(
          `Failed to initialize meta merkle proof: ${JSON.stringify(initConfirmation.value.err)}`
        );
      }
    } catch (error) {
      // Another delegator can create the shared proof between the check above
      // and this transaction landing. Preflight rejects the duplicate before a
      // signature returns, so the send is inside the try as well. Recovery is
      // conditional on the account existing, so a rejected signature or an
      // underfunded payer still surfaces.
      const createdConcurrently = await isMetaMerkleProofInitialized(
        program.provider.connection,
        metaMerkleProofPda
      );
      if (!createdConcurrently) {
        throw error;
      }
    }
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

  // Build modify vote override instruction
  const modifyVoteOverrideInstruction = await program.methods
    .modifyVoteOverride(
      forVotesBn,
      againstVotesBn,
      abstainVotesBn,
      stakeMerkleProofVec,
      stakeMerkleLeaf
    )
    .accountsStrict({
      signer,
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

  const transaction = new Transaction();
  transaction.add(modifyVoteOverrideInstruction);
  transaction.feePayer = signer;
  transaction.recentBlockhash = (
    await program.provider.connection.getLatestBlockhash("confirmed")
  ).blockhash;

  const tx = await signTransactionForWallet(wallet, transaction, signer);

  const signature = await program.provider.connection.sendRawTransaction(
    tx.serialize(),
    { preflightCommitment: "confirmed" }
  );

  console.log("signature modify vote override", signature);

  return {
    signature,
    success: true,
  };
}
