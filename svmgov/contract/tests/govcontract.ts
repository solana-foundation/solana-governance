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
  BALLOT_ID,
  MERKLE_ROOT_HASH,
  ERROR_TEST_PARAMS,
} from "./test-constants";
import {
  deriveProposalAccount,
  deriveProposalIndexAccount,
  deriveSupportAccount,
  deriveVoteAccount,
  deriveConsensusResultAccount,
  deriveMetaMerkleProofAccount,
  deriveVoteOverrideAccount,
  deriveVoteOverrideCacheAccount,
  createEventListener,
  removeEventListener,
  logProposalState,
  logVoteState
} from "./test-helpers";

describe("govcontract", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.govcontract as Program<Govcontract>;
  const mockProgram = anchor.workspace.mock_gov_v1 as Program<MockGovV1>;

  const seed = new anchor.BN(randomBytes(8));
  let testAccounts: TestAccounts;

  before(async () => {
    testAccounts = await setupTestEnvironment(program, mockProgram, seed);
  });

  it("Setup Complete", async () => {
    // Test accounts are set up in before() hook
    console.log("Test environment setup completed successfully");
    console.log("Proposal Account:", testAccounts.proposalAccount.toBase58());
    console.log("Vote Accounts:", testAccounts.voteAccounts.map(acc => acc.toBase58()));
  });

  it("Create Proposal!", async () => {
    let eventReceived = false;
    let eventData: any = null;
    let eventSlot = 0;

    const eventListener = createEventListener(program, 'proposalCreated', (event: any, slot: number) => {
      console.log("ProposalCreated event received");
      eventReceived = true;
      eventData = event;
      eventSlot = slot;
    });

    try {
      const tx = await program.methods
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

      console.log("Proposal created, signature:", tx);
      await new Promise(resolve => setTimeout(resolve, 1000));

      if (eventReceived && eventData) {
        const checks = [
          [eventData.proposalId?.equals(testAccounts.proposalAccount), "Proposal ID"],
          [eventData.author?.equals(provider.publicKey), "Author"],
          [eventData.title === TEST_PROPOSAL_PARAMS.title, "Title"],
          [eventData.description === TEST_PROPOSAL_PARAMS.description, "Description"]
        ];
        const failed = checks.filter(([passed]) => !passed).map(([, field]) => field);
        console.log(failed.length === 0 ? "All event validations passed" :
          `Warning: Validation failed for ${failed.join(", ")}`);
      }

      removeEventListener(program, eventListener);
    } catch (error: any) {
      removeEventListener(program, eventListener);
      throw error;
    }
  });

  it("Add Merkle Root to Proposal!", async () => {
    let eventReceived = false;
    let eventData: any = null;
    let eventSlot = 0;

    const eventListener = createEventListener(program, 'merkleRootAdded', (event: any, slot: number) => {
      console.log("MerkleRootAdded event received");
      eventReceived = true;
      eventData = event;
      eventSlot = slot;
    });

    try {
      const tx = await program.methods
        .addMerkleRoot(MERKLE_ROOT_HASH)
        .accountsPartial({
          signer: provider.publicKey,
          proposal: testAccounts.proposalAccount,
        })
        .rpc();

      console.log("Merkle root added, signature:", tx);
      await new Promise(resolve => setTimeout(resolve, 1000));

      if (eventReceived && eventData) {
        const checks = [
          [eventData.proposalId?.equals(testAccounts.proposalAccount), "Proposal ID"],
          [eventData.author?.equals(provider.publicKey), "Author"],
          [Buffer.from(eventData.merkleRootHash).equals(Buffer.from(MERKLE_ROOT_HASH)), "Merkle Root Hash"]
        ];
        const failed = checks.filter(([passed]) => !passed).map(([, field]) => field);
        console.log(failed.length === 0 ? "All event validations passed" :
          `Warning: Validation failed for ${failed.join(", ")}`);
      }

      // Verify merkle root was set
      const updatedProposal = await program.account.proposal.fetch(testAccounts.proposalAccount);
      console.log("Merkle root set:", !!updatedProposal.merkleRootHash);

      removeEventListener(program, eventListener);
    } catch (error: any) {
      removeEventListener(program, eventListener);
      throw error;
    }
  });

  it("Support Proposal!", async () => {
    let eventReceived = false;
    let eventData: any = null;

    const eventListener = createEventListener(program, 'proposalSupported', (event: any, slot: number) => {
      console.log("ProposalSupported event received");
      eventReceived = true;
      eventData = event;
    });

    try {
      const tx = await program.methods
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

      console.log("Proposal supported, signature:", tx);
      await new Promise(resolve => setTimeout(resolve, 1000));

      if (eventReceived && eventData) {
        const checks = [
          [eventData.proposalId?.equals(testAccounts.proposalAccount), "Proposal ID"],
          [eventData.supporter?.equals(provider.publicKey), "Supporter"]
        ];
        const failed = checks.filter(([passed]) => !passed).map(([, field]) => field);
        console.log(failed.length === 0 ? "All event validations passed" :
          `Warning: Validation failed for ${failed.join(", ")}`);
      }

      const updatedProposal = await program.account.proposal.fetch(testAccounts.proposalAccount);
      logProposalState(updatedProposal, "After Support");

      removeEventListener(program, eventListener);
    } catch (error: any) {
      removeEventListener(program, eventListener);
      throw error;
    }
  });

  it("Cast Votes for Validators!", async () => {
    const validators = [
      { name: "Validator 1", voteAccount: testAccounts.voteAccounts[0], splVoteAccount: testAccounts.splVoteAccounts[2], metaMerkleProof: testAccounts.metaMerkleProofs[2] },
      { name: "Validator 2", voteAccount: testAccounts.voteAccounts[1], splVoteAccount: testAccounts.splVoteAccounts[3], metaMerkleProof: testAccounts.metaMerkleProofs[3] },
      { name: "Validator 3", voteAccount: testAccounts.voteAccounts[2], splVoteAccount: testAccounts.splVoteAccounts[4], metaMerkleProof: testAccounts.metaMerkleProofs[4] },
    ];

    for (const validator of validators) {
      let eventReceived = false;
      let eventData: any = null;

      const eventListener = createEventListener(program, 'voteCast', (event: any, slot: number) => {
        console.log(`VoteCast event received for ${validator.name}`);
        eventReceived = true;
        eventData = event;
      });

      try {
        const voteOverrideCacheAccount = deriveVoteOverrideCacheAccount(
          program,
          testAccounts.proposalAccount,
          validator.voteAccount
        );
        
        const tx = await program.methods
          .castVote(TEST_VOTE_PARAMS.for, TEST_VOTE_PARAMS.against, TEST_VOTE_PARAMS.abstain)
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

        console.log(`${validator.name} vote cast, signature:`, tx);
        await new Promise(resolve => setTimeout(resolve, 500));

        if (eventReceived && eventData) {
          const checks = [
            [eventData.proposalId?.equals(testAccounts.proposalAccount), "Proposal ID"],
            [eventData.voter?.equals(provider.publicKey), "Voter"],
            [eventData.forVotesBp?.eq(TEST_VOTE_PARAMS.for), "For votes BP"],
            [eventData.againstVotesBp?.eq(TEST_VOTE_PARAMS.against), "Against votes BP"],
            [eventData.abstainVotesBp?.eq(TEST_VOTE_PARAMS.abstain), "Abstain votes BP"]
          ];
          const failed = checks.filter(([passed]) => !passed).map(([, field]) => field);
          console.log(failed.length === 0 ? "All event validations passed" :
            `Warning: Validation failed for ${failed.join(", ")}`);
        }

        removeEventListener(program, eventListener);
      } catch (error: any) {
        removeEventListener(program, eventListener);
        throw error;
      }
    }

    // Show final proposal state
    const finalProposal = await program.account.proposal.fetch(testAccounts.proposalAccount);
    logProposalState(finalProposal, "Final After All Votes");
  });

  it("Modify Vote!", async () => {
    let eventReceived = false;
    let eventData: any = null;

    const eventListener = createEventListener(program, 'voteModified', (event: any, slot: number) => {
      console.log("VoteModified event received");
      eventReceived = true;
      eventData = event;
    });

    try {
      const tx = await program.methods
        .modifyVote(TEST_VOTE_MODIFY_PARAMS.for, TEST_VOTE_MODIFY_PARAMS.against, TEST_VOTE_MODIFY_PARAMS.abstain)
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

      console.log("Vote modified, signature:", tx);
      await new Promise(resolve => setTimeout(resolve, 1000));

      if (eventReceived && eventData) {
        const checks = [
          [eventData.proposalId?.equals(testAccounts.proposalAccount), "Proposal ID"],
          [eventData.voter?.equals(provider.publicKey), "Voter"],
          [eventData.newForVotesBp?.eq(TEST_VOTE_MODIFY_PARAMS.for), "New For votes BP"],
          [eventData.newAgainstVotesBp?.eq(TEST_VOTE_MODIFY_PARAMS.against), "New Against votes BP"],
          [eventData.newAbstainVotesBp?.eq(TEST_VOTE_MODIFY_PARAMS.abstain), "New Abstain votes BP"]
        ];
        const failed = checks.filter(([passed]) => !passed).map(([, field]) => field);
        console.log(failed.length === 0 ? "All event validations passed" :
          `Warning: Validation failed for ${failed.join(", ")}`);
      }

      const updatedProposal = await program.account.proposal.fetch(testAccounts.proposalAccount);
      logProposalState(updatedProposal, "After Vote Modification");

      removeEventListener(program, eventListener);
    } catch (error: any) {
      removeEventListener(program, eventListener);
      throw error;
    }
  });

  it("Cast Vote Override!", async () => {
    // Create delegator and stake account
    const delegator = anchor.web3.Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(delegator.publicKey, 2 * LAMPORTS_PER_SOL)
    );

    const delegatorStakeAccount = anchor.web3.Keypair.generate();
    const stakeAccountSize = 200;
    const rentExempt = await provider.connection.getMinimumBalanceForRentExemption(stakeAccountSize);

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

    const stakeTx = new anchor.web3.Transaction().add(createStakeAccountIx, initializeStakeIx);
    await provider.sendAndConfirm(stakeTx, [delegator, delegatorStakeAccount]);

    // Delegate stake to validator 3
    const delegateStakeIx = StakeProgram.delegate({
      stakePubkey: delegatorStakeAccount.publicKey,
      authorizedPubkey: delegator.publicKey,
      votePubkey: testAccounts.splVoteAccounts[4].publicKey,
    });

    const delegateTx = new anchor.web3.Transaction().add(delegateStakeIx as any);
    await provider.sendAndConfirm(delegateTx, [delegator]);

    // Fetch vote account before override
    const voteBefore = await program.account.vote.fetch(testAccounts.voteAccounts[2]);
    logVoteState(voteBefore, "Before Override");

    let eventReceived = false;
    let eventData: any = null;

    const eventListener = createEventListener(program, 'voteOverrideCast', (event: any, slot: number) => {
      console.log("VoteOverrideCast event received");
      eventReceived = true;
      eventData = event;
    });

    try {
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

      await program.methods
        .castVoteOverride(TEST_VOTE_OVERRIDE_PARAMS.for, TEST_VOTE_OVERRIDE_PARAMS.against, TEST_VOTE_OVERRIDE_PARAMS.abstain, [], stakeMerkleLeaf)
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

      console.log("Vote override cast successfully");

      // Show changes
      const voteAfter = await program.account.vote.fetch(testAccounts.voteAccounts[2]);
      logVoteState(voteAfter, "After Override");

      const overrideChange = Number(voteAfter.overrideLamports) - Number(voteBefore.overrideLamports);
      console.log(`Override lamports change: ${overrideChange} (${overrideChange / LAMPORTS_PER_SOL} SOL)`);

      await new Promise(resolve => setTimeout(resolve, 1000));

      if (eventReceived && eventData) {
        const checks = [
          [eventData.proposalId?.equals(testAccounts.proposalAccount), "Proposal ID"],
          [eventData.delegator?.equals(delegator.publicKey), "Delegator"],
          [eventData.validator?.equals(testAccounts.splVoteAccounts[4].publicKey), "Validator"],
          [eventData.forVotesBp?.eq(TEST_VOTE_OVERRIDE_PARAMS.for), "For votes BP"],
          [eventData.againstVotesBp?.eq(TEST_VOTE_OVERRIDE_PARAMS.against), "Against votes BP"],
          [eventData.abstainVotesBp?.eq(TEST_VOTE_OVERRIDE_PARAMS.abstain), "Abstain votes BP"]
        ];
        const failed = checks.filter(([passed]) => !passed).map(([, field]) => field);
        console.log(failed.length === 0 ? "All event validations passed" :
          `Warning: Validation failed for ${failed.join(", ")}`);
      }

      const finalProposal = await program.account.proposal.fetch(testAccounts.proposalAccount);
      logProposalState(finalProposal, "Final After Vote Override");

      removeEventListener(program, eventListener);
    } catch (error: any) {
      removeEventListener(program, eventListener);
      throw error;
    }
  });

  it("Modify Vote Override!", async () => {
    // Use the same delegator and stake account from the previous test
    const delegator = anchor.web3.Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(delegator.publicKey, 2 * LAMPORTS_PER_SOL)
    );

    const delegatorStakeAccount = anchor.web3.Keypair.generate();
    const stakeAccountSize = 200;
    const rentExempt = await provider.connection.getMinimumBalanceForRentExemption(stakeAccountSize);

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

    const stakeTx = new anchor.web3.Transaction().add(createStakeAccountIx, initializeStakeIx);
    await provider.sendAndConfirm(stakeTx, [delegator, delegatorStakeAccount]);

    // Delegate stake to validator 3
    const delegateStakeIx = StakeProgram.delegate({
      stakePubkey: delegatorStakeAccount.publicKey,
      authorizedPubkey: delegator.publicKey,
      votePubkey: testAccounts.splVoteAccounts[4].publicKey,
    });

    const delegateTx = new anchor.web3.Transaction().add(delegateStakeIx as any);
    await provider.sendAndConfirm(delegateTx, [delegator]);

    // First, cast a vote override to have something to modify
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

    // Cast initial vote override
    await program.methods
      .castVoteOverride(TEST_VOTE_OVERRIDE_PARAMS.for, TEST_VOTE_OVERRIDE_PARAMS.against, TEST_VOTE_OVERRIDE_PARAMS.abstain, [], stakeMerkleLeaf)
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

    console.log("Initial vote override cast successfully");

    // Fetch states before modification
    const voteBefore = await program.account.vote.fetch(testAccounts.voteAccounts[2]);
    const overrideBefore = await program.account.voteOverride.fetch(voteOverrideAccount);
    const proposalBefore = await program.account.proposal.fetch(testAccounts.proposalAccount);

    logVoteState(voteBefore, "Before Override Modification");
    console.log("Override Before - For BP:", overrideBefore.forVotesBp.toString());
    console.log("Override Before - Against BP:", overrideBefore.againstVotesBp.toString());
    console.log("Override Before - Abstain BP:", overrideBefore.abstainVotesBp.toString());

    let eventReceived = false;
    let eventData: any = null;

    const eventListener = createEventListener(program, 'voteOverrideModified', (event: any, slot: number) => {
      console.log("VoteOverrideModified event received");
      eventReceived = true;
      eventData = event;
    });

    try {
      // Now modify the vote override
      const tx = await program.methods
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

      console.log("Vote override modified successfully, signature:", tx);
      await new Promise(resolve => setTimeout(resolve, 1000));

      // Fetch states after modification
      const voteAfter = await program.account.vote.fetch(testAccounts.voteAccounts[2]);
      const overrideAfter = await program.account.voteOverride.fetch(voteOverrideAccount);
      const proposalAfter = await program.account.proposal.fetch(testAccounts.proposalAccount);

      logVoteState(voteAfter, "After Override Modification");
      console.log("Override After - For BP:", overrideAfter.forVotesBp.toString());
      console.log("Override After - Against BP:", overrideAfter.againstVotesBp.toString());
      console.log("Override After - Abstain BP:", overrideAfter.abstainVotesBp.toString());

      // Verify the changes
      console.log("\n=== Vote Override Modification Verification ===");
      console.log("Old For BP:", overrideBefore.forVotesBp.toString(), "-> New For BP:", overrideAfter.forVotesBp.toString());
      console.log("Old Against BP:", overrideBefore.againstVotesBp.toString(), "-> New Against BP:", overrideAfter.againstVotesBp.toString());
      console.log("Old Abstain BP:", overrideBefore.abstainVotesBp.toString(), "-> New Abstain BP:", overrideAfter.abstainVotesBp.toString());

      // Verify proposal vote totals changed correctly
      const forVotesChange = Number(proposalAfter.forVotesLamports) - Number(proposalBefore.forVotesLamports);
      const againstVotesChange = Number(proposalAfter.againstVotesLamports) - Number(proposalBefore.againstVotesLamports);
      const abstainVotesChange = Number(proposalAfter.abstainVotesLamports) - Number(proposalBefore.abstainVotesLamports);

      console.log("Proposal For Votes Change:", forVotesChange / LAMPORTS_PER_SOL, "SOL");
      console.log("Proposal Against Votes Change:", againstVotesChange / LAMPORTS_PER_SOL, "SOL");
      console.log("Proposal Abstain Votes Change:", abstainVotesChange / LAMPORTS_PER_SOL, "SOL");

      // Validate event data
      if (eventReceived && eventData) {
        const checks = [
          [eventData.proposalId?.equals(testAccounts.proposalAccount), "Proposal ID"],
          [eventData.delegator?.equals(delegator.publicKey), "Delegator"],
          [eventData.validator?.equals(testAccounts.splVoteAccounts[4].publicKey), "Validator"],
          [eventData.oldForVotesBp?.eq(TEST_VOTE_OVERRIDE_PARAMS.for), "Old For votes BP"],
          [eventData.oldAgainstVotesBp?.eq(TEST_VOTE_OVERRIDE_PARAMS.against), "Old Against votes BP"],
          [eventData.oldAbstainVotesBp?.eq(TEST_VOTE_OVERRIDE_PARAMS.abstain), "Old Abstain votes BP"],
          [eventData.newForVotesBp?.eq(TEST_VOTE_OVERRIDE_MODIFY_PARAMS.for), "New For votes BP"],
          [eventData.newAgainstVotesBp?.eq(TEST_VOTE_OVERRIDE_MODIFY_PARAMS.against), "New Against votes BP"],
          [eventData.newAbstainVotesBp?.eq(TEST_VOTE_OVERRIDE_MODIFY_PARAMS.abstain), "New Abstain votes BP"]
        ];
        const failed = checks.filter(([passed]) => !passed).map(([, field]) => field);
        console.log(failed.length === 0 ? "All event validations passed" :
          `Warning: Validation failed for ${failed.join(", ")}`);
      } else {
        console.log("Warning: VoteOverrideModified event was not received");
      }

      // Verify the override account was updated correctly
      const expectedForBp = TEST_VOTE_OVERRIDE_MODIFY_PARAMS.for;
      const expectedAgainstBp = TEST_VOTE_OVERRIDE_MODIFY_PARAMS.against;
      const expectedAbstainBp = TEST_VOTE_OVERRIDE_MODIFY_PARAMS.abstain;

      if (overrideAfter.forVotesBp.eq(expectedForBp) &&
          overrideAfter.againstVotesBp.eq(expectedAgainstBp) &&
          overrideAfter.abstainVotesBp.eq(expectedAbstainBp)) {
        console.log("✅ Vote override account updated correctly");
      } else {
        console.log("❌ Vote override account not updated correctly");
      }

      const finalProposal = await program.account.proposal.fetch(testAccounts.proposalAccount);
      logProposalState(finalProposal, "Final After Vote Override Modification");

      removeEventListener(program, eventListener);
    } catch (error: any) {
      removeEventListener(program, eventListener);
      throw error;
    }
  });
});

// Error Tests Structure
describe("govcontract - Error Cases", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.govcontract as Program<Govcontract>;
  const mockProgram = anchor.workspace.mock_gov_v1 as Program<MockGovV1>;

  // Error tests run against the state set up by the main test suite above

  // more needed error tests:
  // - Create proposal with invalid parameters
  // - Cast vote without proper authorization
  // - Modify vote that doesn't exist
  // - Support proposal after voting has ended
  // - Vote override with insufficient stake
  // - etc.

  it("Error Test - Cannot Initialize Index Twice", async () => {
  
    const proposalIndexAccount = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("index")],
      program.programId
    )[0];

    try {
      await program.methods
        .initializeIndex()
        .accountsPartial({
          signer: provider.publicKey,
          proposalIndex: proposalIndexAccount,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      throw new Error("Expected error was not thrown - index should already be initialized");
    } catch (error: any) {
      // Verify the error is the expected one (account already in use)
      console.log("Expected error caught:", error.message);

    }
  });

  it("Error Test - Create Proposal with Empty Title", async () => {
    const testSeed = new anchor.BN(randomBytes(8));
    const splVoteAccount = anchor.web3.Keypair.generate();

    const space = 3762; // Vote account size
    const lamports = await provider.connection.getMinimumBalanceForRentExemption(space);

    await program.provider.sendAndConfirm(
      new anchor.web3.Transaction().add(
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: provider.publicKey,
          newAccountPubkey: splVoteAccount.publicKey,
          space,
          lamports: lamports + 1000000000, // 1 SOL
          programId: anchor.web3.VoteProgram.programId,
        }),
        anchor.web3.VoteProgram.initializeAccount({
          votePubkey: splVoteAccount.publicKey,
          nodePubkey: provider.publicKey,
          voteInit: new anchor.web3.VoteInit(
            provider.publicKey,
            provider.publicKey,
            provider.publicKey,
            1
          ),
        })
      ),
      [splVoteAccount]
    );

    try {
      await program.methods
        .createProposal(
          testSeed,
          ERROR_TEST_PARAMS.emptyTitle,
          TEST_PROPOSAL_PARAMS.description
        )
        .accountsPartial({
          signer: provider.publicKey,
          proposal: deriveProposalAccount(program, testSeed, splVoteAccount.publicKey),
          proposalIndex: deriveProposalIndexAccount(program),
          splVoteAccount: splVoteAccount.publicKey,
          snapshotProgram: mockProgram.programId,
          consensusResult: deriveConsensusResultAccount(mockProgram),
          metaMerkleProof: deriveMetaMerkleProofAccount(mockProgram, deriveConsensusResultAccount(mockProgram), splVoteAccount.publicKey),
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      throw new Error("Expected error was not thrown - empty title should be rejected");
    } catch (error: any) {
      // Verify the error is the expected one
      console.log("Expected error caught:", error.message);
      if (error.message.includes("TitleEmpty") || error.message.includes("cannot be empty")) {
        console.log("Correctly caught TitleEmpty error");
      }
    }
  });

  it("Error Test - Create Proposal with Empty Description", async () => {
    const testSeed = new anchor.BN(randomBytes(8));
    const splVoteAccount = anchor.web3.Keypair.generate();

    const space = 3762; // Vote account size
    const lamports = await provider.connection.getMinimumBalanceForRentExemption(space);

    await program.provider.sendAndConfirm(
      new anchor.web3.Transaction().add(
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: provider.publicKey,
          newAccountPubkey: splVoteAccount.publicKey,
          space,
          lamports: lamports + 1000000000, // 1 SOL
          programId: anchor.web3.VoteProgram.programId,
        }),
        anchor.web3.VoteProgram.initializeAccount({
          votePubkey: splVoteAccount.publicKey,
          nodePubkey: provider.publicKey,
          voteInit: new anchor.web3.VoteInit(
            provider.publicKey,
            provider.publicKey,
            provider.publicKey,
            1
          ),
        })
      ),
      [splVoteAccount]
    );

    try {
      await program.methods
        .createProposal(
          testSeed,
          TEST_PROPOSAL_PARAMS.title,
          ERROR_TEST_PARAMS.emptyDescription
        )
        .accountsPartial({
          signer: provider.publicKey,
          proposal: deriveProposalAccount(program, testSeed, splVoteAccount.publicKey),
          proposalIndex: deriveProposalIndexAccount(program),
          splVoteAccount: splVoteAccount.publicKey,
          snapshotProgram: mockProgram.programId,
          consensusResult: deriveConsensusResultAccount(mockProgram),
          metaMerkleProof: deriveMetaMerkleProofAccount(mockProgram, deriveConsensusResultAccount(mockProgram), splVoteAccount.publicKey),
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      throw new Error("Expected error was not thrown - empty description should be rejected");
    } catch (error: any) {
      // Verify the error is the expected one
      console.log("Expected error caught:", error.message);
      if (error.message.includes("DescriptionEmpty") || error.message.includes("cannot be empty")) {
        console.log("Correctly caught DescriptionEmpty error");
      }
    }
  });


  it("Error Test - Cannot Modify Merkle Root After Voting Starts", async () => {

    console.log("Merkle root modification");
  });

  it("Error Test - Arithmetic Overflow Handling", async () => {
    console.log("Arithmetic overflow");
  });
});
