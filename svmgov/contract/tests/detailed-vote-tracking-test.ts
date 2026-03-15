import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Govcontract } from "../target/types/govcontract";
import { MockGovV1 } from "../target/types/mock_gov_v1";
import { randomBytes } from "crypto";
import { LAMPORTS_PER_SOL, StakeProgram, SystemProgram } from "@solana/web3.js";
import { setupTestEnvironment, TestAccounts } from "./test-setup";
import {
  TEST_PROPOSAL_PARAMS,
  TEST_VOTE_PARAMS,
  TEST_VOTE_MODIFY_PARAMS,
  TEST_VOTE_OVERRIDE_PARAMS,
  TEST_VOTE_OVERRIDE_MODIFY_PARAMS,
  MERKLE_ROOT_HASH,
} from "./test-constants";
import {
  deriveVoteOverrideAccount,
  deriveVoteOverrideCacheAccount,
} from "./test-helpers";

// Helper function to log account states with unique identifiers
async function logAccountStates(
  program: Program<Govcontract>,
  testAccounts: TestAccounts,
  stepName: string,
  voteOverrideAccount?: anchor.web3.PublicKey,
  voteOverrideCacheAccount?: anchor.web3.PublicKey
) {
  console.log(`\n🔍 === ${stepName.toUpperCase()} ACCOUNT STATES ===`);

  // Log Proposal State
  const proposal = await program.account.proposal.fetch(
    testAccounts.proposalAccount
  );
  console.log(
    `${stepName}_PROPOSAL_FOR_LAMPORTS: ${proposal.forVotesLamports.toString()} (${
      Number(proposal.forVotesLamports) / LAMPORTS_PER_SOL
    } SOL)`
  );
  console.log(
    `${stepName}_PROPOSAL_AGAINST_LAMPORTS: ${proposal.againstVotesLamports.toString()} (${
      Number(proposal.againstVotesLamports) / LAMPORTS_PER_SOL
    } SOL)`
  );
  console.log(
    `${stepName}_PROPOSAL_ABSTAIN_LAMPORTS: ${proposal.abstainVotesLamports.toString()} (${
      Number(proposal.abstainVotesLamports) / LAMPORTS_PER_SOL
    } SOL)`
  );
  console.log(`${stepName}_PROPOSAL_VOTE_COUNT: ${proposal.voteCount}`);

  // Log all Validator Vote Account States
  for (let i = 0; i < 3; i++) {
    try {
      const vote = await program.account.vote.fetch(
        testAccounts.voteAccounts[i]
      );
      console.log(
        `${stepName}_VALIDATOR${i + 1}_FOR_BP: ${vote.forVotesBp.toString()}`
      );
      console.log(
        `${stepName}_VALIDATOR${
          i + 1
        }_AGAINST_BP: ${vote.againstVotesBp.toString()}`
      );
      console.log(
        `${stepName}_VALIDATOR${
          i + 1
        }_ABSTAIN_BP: ${vote.abstainVotesBp.toString()}`
      );
      console.log(
        `${stepName}_VALIDATOR${
          i + 1
        }_FOR_LAMPORTS: ${vote.forVotesLamports.toString()} (${
          Number(vote.forVotesLamports) / LAMPORTS_PER_SOL
        } SOL)`
      );
      console.log(
        `${stepName}_VALIDATOR${
          i + 1
        }_AGAINST_LAMPORTS: ${vote.againstVotesLamports.toString()} (${
          Number(vote.againstVotesLamports) / LAMPORTS_PER_SOL
        } SOL)`
      );
      console.log(
        `${stepName}_VALIDATOR${
          i + 1
        }_ABSTAIN_LAMPORTS: ${vote.abstainVotesLamports.toString()} (${
          Number(vote.abstainVotesLamports) / LAMPORTS_PER_SOL
        } SOL)`
      );
      console.log(
        `${stepName}_VALIDATOR${
          i + 1
        }_OVERRIDE_LAMPORTS: ${vote.overrideLamports.toString()} (${
          Number(vote.overrideLamports) / LAMPORTS_PER_SOL
        } SOL)`
      );
      console.log(
        `${stepName}_VALIDATOR${i + 1}_STAKE: ${vote.stake.toString()} (${
          Number(vote.stake) / LAMPORTS_PER_SOL
        } SOL)`
      );
    } catch (error) {
      console.log(`${stepName}_VALIDATOR${i + 1}_STATUS: NOT_CREATED`);
    }
  }

  // Log Vote Override Account State (if exists)
  if (voteOverrideAccount) {
    try {
      const override = await program.account.voteOverride.fetch(
        voteOverrideAccount
      );
      console.log(
        `${stepName}_OVERRIDE_FOR_BP: ${override.forVotesBp.toString()}`
      );
      console.log(
        `${stepName}_OVERRIDE_AGAINST_BP: ${override.againstVotesBp.toString()}`
      );
      console.log(
        `${stepName}_OVERRIDE_ABSTAIN_BP: ${override.abstainVotesBp.toString()}`
      );
      console.log(
        `${stepName}_OVERRIDE_FOR_LAMPORTS: ${override.forVotesLamports.toString()} (${
          Number(override.forVotesLamports) / LAMPORTS_PER_SOL
        } SOL)`
      );
      console.log(
        `${stepName}_OVERRIDE_AGAINST_LAMPORTS: ${override.againstVotesLamports.toString()} (${
          Number(override.againstVotesLamports) / LAMPORTS_PER_SOL
        } SOL)`
      );
      console.log(
        `${stepName}_OVERRIDE_ABSTAIN_LAMPORTS: ${override.abstainVotesLamports.toString()} (${
          Number(override.abstainVotesLamports) / LAMPORTS_PER_SOL
        } SOL)`
      );
      console.log(
        `${stepName}_OVERRIDE_STAKE_AMOUNT: ${override.stakeAmount.toString()} (${
          Number(override.stakeAmount) / LAMPORTS_PER_SOL
        } SOL)`
      );
    } catch (error) {
      console.log(`${stepName}_OVERRIDE_STATUS: NOT_CREATED`);
    }
  }

  // Log Vote Override Cache Account State (if exists)
  if (voteOverrideCacheAccount) {
    try {
      const cache = await program.account.voteOverrideCache.fetch(
        voteOverrideCacheAccount
      );
      console.log(`${stepName}_CACHE_FOR_BP: ${cache.forVotesBp.toString()}`);
      console.log(
        `${stepName}_CACHE_AGAINST_BP: ${cache.againstVotesBp.toString()}`
      );
      console.log(
        `${stepName}_CACHE_ABSTAIN_BP: ${cache.abstainVotesBp.toString()}`
      );
      console.log(
        `${stepName}_CACHE_FOR_LAMPORTS: ${cache.forVotesLamports.toString()} (${
          Number(cache.forVotesLamports) / LAMPORTS_PER_SOL
        } SOL)`
      );
      console.log(
        `${stepName}_CACHE_AGAINST_LAMPORTS: ${cache.againstVotesLamports.toString()} (${
          Number(cache.againstVotesLamports) / LAMPORTS_PER_SOL
        } SOL)`
      );
      console.log(
        `${stepName}_CACHE_ABSTAIN_LAMPORTS: ${cache.abstainVotesLamports.toString()} (${
          Number(cache.abstainVotesLamports) / LAMPORTS_PER_SOL
        } SOL)`
      );
      console.log(
        `${stepName}_CACHE_TOTAL_STAKE: ${cache.totalStake.toString()} (${
          Number(cache.totalStake) / LAMPORTS_PER_SOL
        } SOL)`
      );
    } catch (error) {
      console.log(`${stepName}_CACHE_STATUS: NOT_CREATED`);
    }
  }
}

describe.only("Detailed Vote Tracking Test", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.govcontract as Program<Govcontract>;
  const mockProgram = anchor.workspace.mock_gov_v1 as Program<MockGovV1>;

  const seed = new anchor.BN(randomBytes(8));
  let testAccounts: TestAccounts;

  before(async () => {
    testAccounts = await setupTestEnvironment(program, mockProgram, seed);
  });

  it("Complete Vote Flow with Detailed Tracking", async () => {
    console.log("\n🚀 === STARTING DETAILED VOTE TRACKING TEST ===");
    console.log("Test Parameters:");
    console.log(`- Validator Stake: 100,000 SOL each`);
    console.log(`- Delegator Stake: 0.5 SOL`);
    console.log(
      `- TEST_VOTE_PARAMS: for=${TEST_VOTE_PARAMS.for}, against=${TEST_VOTE_PARAMS.against}, abstain=${TEST_VOTE_PARAMS.abstain}`
    );
    console.log(
      `- TEST_VOTE_OVERRIDE_PARAMS: for=${TEST_VOTE_OVERRIDE_PARAMS.for}, against=${TEST_VOTE_OVERRIDE_PARAMS.against}, abstain=${TEST_VOTE_OVERRIDE_PARAMS.abstain}`
    );
    console.log(
      `- TEST_VOTE_OVERRIDE_MODIFY_PARAMS: for=${TEST_VOTE_OVERRIDE_MODIFY_PARAMS.for}, against=${TEST_VOTE_OVERRIDE_MODIFY_PARAMS.against}, abstain=${TEST_VOTE_OVERRIDE_MODIFY_PARAMS.abstain}`
    );

    // STEP 1: Create Proposal
    await program.methods
      .createProposal(
        seed,
        TEST_PROPOSAL_PARAMS.title,
        TEST_PROPOSAL_PARAMS.description
      )
      .accountsPartial({
        signer: provider.publicKey,
        proposal: testAccounts.proposalAccount,
        proposalIndex: testAccounts.proposalIndexAccount,
        splVoteAccount: testAccounts.splVoteAccounts[0].publicKey,
        snapshotProgram: mockProgram.programId,
        consensusResult: testAccounts.consensusResult,
        metaMerkleProof: testAccounts.metaMerkleProofs[0],
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .addMerkleRoot(MERKLE_ROOT_HASH)
      .accountsPartial({
        signer: provider.publicKey,
        proposal: testAccounts.proposalAccount,
      })
      .rpc();

    await program.methods
      .supportProposal()
      .accountsPartial({
        signer: provider.publicKey,
        proposal: testAccounts.proposalAccount,
        support: testAccounts.supportAccount,
        splVoteAccount: testAccounts.splVoteAccounts[1].publicKey,
        snapshotProgram: mockProgram.programId,
        consensusResult: testAccounts.consensusResult,
        metaMerkleProof: testAccounts.metaMerkleProofs[1],
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await logAccountStates(program, testAccounts, "STEP1_INITIAL");

    // STEP 2: Cast Votes for All 3 Validators
    const validators = [
      {
        name: "Validator 1",
        voteAccount: testAccounts.voteAccounts[0],
        splVoteAccount: testAccounts.splVoteAccounts[2],
        metaMerkleProof: testAccounts.metaMerkleProofs[2],
      },
      {
        name: "Validator 2",
        voteAccount: testAccounts.voteAccounts[1],
        splVoteAccount: testAccounts.splVoteAccounts[3],
        metaMerkleProof: testAccounts.metaMerkleProofs[3],
      },
      {
        name: "Validator 3",
        voteAccount: testAccounts.voteAccounts[2],
        splVoteAccount: testAccounts.splVoteAccounts[4],
        metaMerkleProof: testAccounts.metaMerkleProofs[4],
      },
    ];

    for (const [index, validator] of validators.entries()) {
      const voteOverrideCacheAccount = deriveVoteOverrideCacheAccount(
        program,
        testAccounts.proposalAccount,
        validator.voteAccount
      );

      await program.methods
        .castVote(
          TEST_VOTE_PARAMS.for,
          TEST_VOTE_PARAMS.against,
          TEST_VOTE_PARAMS.abstain
        )
        .accountsPartial({
          signer: provider.publicKey,
          proposal: testAccounts.proposalAccount,
          vote: validator.voteAccount,
          voteOverrideCache: voteOverrideCacheAccount,
          splVoteAccount: validator.splVoteAccount.publicKey,
          snapshotProgram: mockProgram.programId,
          consensusResult: testAccounts.consensusResult,
          metaMerkleProof: validator.metaMerkleProof,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      await logAccountStates(
        program,
        testAccounts,
        `STEP2_VALIDATOR${index + 1}_VOTED`
      );
    }

    // STEP 3: Modify Validator 1's Vote
    await program.methods
      .modifyVote(
        TEST_VOTE_MODIFY_PARAMS.for,
        TEST_VOTE_MODIFY_PARAMS.against,
        TEST_VOTE_MODIFY_PARAMS.abstain
      )
      .accountsPartial({
        signer: provider.publicKey,
        proposal: testAccounts.proposalAccount,
        vote: testAccounts.voteAccounts[0],
        splVoteAccount: testAccounts.splVoteAccounts[2].publicKey,
        snapshotProgram: mockProgram.programId,
        consensusResult: testAccounts.consensusResult,
        metaMerkleProof: testAccounts.metaMerkleProofs[2],
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await logAccountStates(program, testAccounts, "STEP3_VALIDATOR1_MODIFIED");

    // STEP 4: Create Delegator and Cast Vote Override on Validator 3
    const delegator = anchor.web3.Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        delegator.publicKey,
        2 * LAMPORTS_PER_SOL
      )
    );

    const delegatorStakeAccount = anchor.web3.Keypair.generate();
    const stakeAccountSize = 200;
    const rentExempt =
      await provider.connection.getMinimumBalanceForRentExemption(
        stakeAccountSize
      );

    const createStakeAccountIx = SystemProgram.createAccount({
      fromPubkey: delegator.publicKey,
      newAccountPubkey: delegatorStakeAccount.publicKey,
      lamports: rentExempt + LAMPORTS_PER_SOL,
      space: stakeAccountSize,
      programId: StakeProgram.programId,
    });

    const initializeStakeIx = StakeProgram.initialize({
      stakePubkey: delegatorStakeAccount.publicKey,
      authorized: {
        staker: delegator.publicKey,
        withdrawer: delegator.publicKey,
      },
    });

    const stakeTx = new anchor.web3.Transaction().add(
      createStakeAccountIx,
      initializeStakeIx
    );
    await provider.sendAndConfirm(stakeTx, [delegator, delegatorStakeAccount]);

    const delegateStakeIx = StakeProgram.delegate({
      stakePubkey: delegatorStakeAccount.publicKey,
      authorizedPubkey: delegator.publicKey,
      votePubkey: testAccounts.splVoteAccounts[4].publicKey,
    });

    const delegateTx = new anchor.web3.Transaction().add(
      delegateStakeIx as any
    );
    await provider.sendAndConfirm(delegateTx, [delegator]);

    const stakeMerkleLeaf = {
      votingWallet: delegator.publicKey,
      stakeAccount: delegatorStakeAccount.publicKey,
      activeStake: new anchor.BN(500000000), // 0.5 SOL stake
    };

    const voteOverrideAccount = deriveVoteOverrideAccount(
      program,
      testAccounts.proposalAccount,
      delegatorStakeAccount.publicKey,
      testAccounts.voteAccounts[2]
    );

    const voteOverrideCacheAccount = deriveVoteOverrideCacheAccount(
      program,
      testAccounts.proposalAccount,
      testAccounts.voteAccounts[2]
    );

    console.log("\n📋 DELEGATOR DETAILS:");
    console.log(`DELEGATOR_PUBKEY: ${delegator.publicKey.toBase58()}`);
    console.log(
      `DELEGATOR_STAKE_ACCOUNT: ${delegatorStakeAccount.publicKey.toBase58()}`
    );
    console.log(`DELEGATOR_STAKE_AMOUNT: 0.5 SOL`);
    console.log(`VOTE_OVERRIDE_ACCOUNT: ${voteOverrideAccount.toBase58()}`);
    console.log(
      `VOTE_OVERRIDE_CACHE_ACCOUNT: ${voteOverrideCacheAccount.toBase58()}`
    );

    await logAccountStates(
      program,
      testAccounts,
      "STEP4_BEFORE_OVERRIDE",
      voteOverrideAccount,
      voteOverrideCacheAccount
    );

    // Cast Vote Override
    await program.methods
      .castVoteOverride(
        TEST_VOTE_OVERRIDE_PARAMS.for,
        TEST_VOTE_OVERRIDE_PARAMS.against,
        TEST_VOTE_OVERRIDE_PARAMS.abstain,
        [],
        stakeMerkleLeaf
      )
      .accountsPartial({
        signer: delegator.publicKey,
        proposal: testAccounts.proposalAccount,
        validatorVote: testAccounts.voteAccounts[2],
        voteOverrideCache: voteOverrideCacheAccount,
        splVoteAccount: testAccounts.splVoteAccounts[4].publicKey,
        voteOverride: voteOverrideAccount,
        splStakeAccount: delegatorStakeAccount.publicKey,
        snapshotProgram: mockProgram.programId,
        consensusResult: testAccounts.consensusResult,
        metaMerkleProof: testAccounts.metaMerkleProofs[4],
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([delegator])
      .rpc();

    await logAccountStates(
      program,
      testAccounts,
      "STEP5_AFTER_OVERRIDE",
      voteOverrideAccount,
      voteOverrideCacheAccount
    );

    // STEP 5: Modify Vote Override
    console.log("\n🔄 === MODIFYING VOTE OVERRIDE ===");
    console.log(
      `Changing from: for=${TEST_VOTE_OVERRIDE_PARAMS.for}, against=${TEST_VOTE_OVERRIDE_PARAMS.against}, abstain=${TEST_VOTE_OVERRIDE_PARAMS.abstain}`
    );
    console.log(
      `Changing to: for=${TEST_VOTE_OVERRIDE_MODIFY_PARAMS.for}, against=${TEST_VOTE_OVERRIDE_MODIFY_PARAMS.against}, abstain=${TEST_VOTE_OVERRIDE_MODIFY_PARAMS.abstain}`
    );

    try {
      await program.methods
        .modifyVoteOverride(
          TEST_VOTE_OVERRIDE_MODIFY_PARAMS.for,
          TEST_VOTE_OVERRIDE_MODIFY_PARAMS.against,
          TEST_VOTE_OVERRIDE_MODIFY_PARAMS.abstain,
          [],
          stakeMerkleLeaf
        )
        .accountsPartial({
          signer: delegator.publicKey,
          proposal: testAccounts.proposalAccount,
          validatorVote: testAccounts.voteAccounts[2],
          voteOverrideCache: voteOverrideCacheAccount,
          splVoteAccount: testAccounts.splVoteAccounts[4].publicKey,
          voteOverride: voteOverrideAccount,
          splStakeAccount: delegatorStakeAccount.publicKey,
          snapshotProgram: mockProgram.programId,
          consensusResult: testAccounts.consensusResult,
          metaMerkleProof: testAccounts.metaMerkleProofs[4],
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([delegator])
        .rpc();

      await logAccountStates(
        program,
        testAccounts,
        "STEP6_AFTER_MODIFY_OVERRIDE",
        voteOverrideAccount,
        voteOverrideCacheAccount
      );

      console.log("\n✅ === MODIFY VOTE OVERRIDE SUCCEEDED ===");
    } catch (error: any) {
      console.log("\n❌ === MODIFY VOTE OVERRIDE FAILED ===");
      console.log(`ERROR: ${error.message}`);
      if (error.logs) {
        console.log("PROGRAM LOGS:");
        error.logs.forEach((log: string, index: number) => {
          console.log(`LOG_${index}: ${log}`);
        });
      }

      // Still log the account states to see what happened
      await logAccountStates(
        program,
        testAccounts,
        "STEP6_FAILED_MODIFY_OVERRIDE",
        voteOverrideAccount,
        voteOverrideCacheAccount
      );
    }

    console.log("\n🏁 === TEST COMPLETED ===");
  });
});
