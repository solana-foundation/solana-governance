import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Govcontract } from "../target/types/govcontract";
import { MockGovV1 } from "../target/types/mock_gov_v1";
import { randomBytes } from "crypto";
import { LAMPORTS_PER_SOL, StakeProgram, SystemProgram } from "@solana/web3.js";
import { setupTestEnvironment, TestAccounts } from "./test-setup";
import { TEST_PROPOSAL_PARAMS, MERKLE_ROOT_HASH } from "./test-constants";
import {
  deriveVoteOverrideAccount,
  deriveVoteOverrideCacheAccount,
} from "./test-helpers";
import { expect } from "chai";

describe.only("Account Size Validation Bug Test", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.govcontract as Program<Govcontract>;
  const mockProgram = anchor.workspace.mock_gov_v1 as Program<MockGovV1>;

  const seed = new anchor.BN(randomBytes(8));
  let testAccounts: TestAccounts;

  before(async () => {
    testAccounts = await setupTestEnvironment(program, mockProgram, seed);

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

  it("Demonstrates Account Size Validation Bug - Wrong Execution Paths", async () => {
    console.log("=== Testing Account Size Validation Bug ===");
    console.log("Path: Delegator1 → Validator → Delegator2");

    // Create two delegators
    const delegator1 = anchor.web3.Keypair.generate();
    const delegator2 = anchor.web3.Keypair.generate();

    // Fund delegators
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        delegator1.publicKey,
        2 * LAMPORTS_PER_SOL
      )
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        delegator2.publicKey,
        2 * LAMPORTS_PER_SOL
      )
    );

    // Create stake accounts for both delegators
    const createStakeAccount = async (delegator: anchor.web3.Keypair) => {
      const stakeAccount = anchor.web3.Keypair.generate();
      const stakeAccountSize = 200;
      const rentExempt =
        await provider.connection.getMinimumBalanceForRentExemption(
          stakeAccountSize
        );

      const createStakeAccountIx = SystemProgram.createAccount({
        fromPubkey: delegator.publicKey,
        newAccountPubkey: stakeAccount.publicKey,
        lamports: rentExempt + LAMPORTS_PER_SOL,
        space: stakeAccountSize,
        programId: StakeProgram.programId,
      });

      const initializeStakeIx = StakeProgram.initialize({
        stakePubkey: stakeAccount.publicKey,
        authorized: {
          staker: delegator.publicKey,
          withdrawer: delegator.publicKey,
        },
      });

      const stakeTx = new anchor.web3.Transaction().add(
        createStakeAccountIx,
        initializeStakeIx
      );
      await provider.sendAndConfirm(stakeTx, [delegator, stakeAccount]);

      // Delegate to validator
      const delegateStakeIx = StakeProgram.delegate({
        stakePubkey: stakeAccount.publicKey,
        authorizedPubkey: delegator.publicKey,
        votePubkey: testAccounts.splVoteAccounts[2].publicKey,
      });

      const delegateTx = new anchor.web3.Transaction().add(
        delegateStakeIx as any
      );
      await provider.sendAndConfirm(delegateTx, [delegator]);

      return stakeAccount;
    };

    const stakeAccount1 = await createStakeAccount(delegator1);
    const stakeAccount2 = await createStakeAccount(delegator2);

    const validatorVoteAccount = testAccounts.voteAccounts[0];
    const voteOverrideCacheAccount = deriveVoteOverrideCacheAccount(
      program,
      testAccounts.proposalAccount,
      validatorVoteAccount
    );

    console.log("\\n=== STEP 1: DELEGATOR 1 VOTES FIRST ===");
    console.log("Expected: Creates vote override cache");

    const stakeMerkleLeaf1 = {
      votingWallet: delegator1.publicKey,
      stakeAccount: stakeAccount1.publicKey,
      activeStake: new anchor.BN(300000000), // 0.3 SOL
    };

    const voteOverrideAccount1 = deriveVoteOverrideAccount(
      program,
      testAccounts.proposalAccount,
      stakeAccount1.publicKey,
      validatorVoteAccount
    );

    await program.methods
      .castVoteOverride(
        new anchor.BN(5000), // 50% for
        new anchor.BN(3000), // 30% against
        new anchor.BN(2000), // 20% abstain
        [],
        stakeMerkleLeaf1
      )
      .accountsPartial({
        signer: delegator1.publicKey,
        proposal: testAccounts.proposalAccount,
        validatorVote: validatorVoteAccount,
        voteOverrideCache: voteOverrideCacheAccount,
        splVoteAccount: testAccounts.splVoteAccounts[2].publicKey,
        voteOverride: voteOverrideAccount1,
        splStakeAccount: stakeAccount1.publicKey,
        snapshotProgram: mockProgram.programId,
        consensusResult: testAccounts.consensusResult,
        metaMerkleProof: testAccounts.metaMerkleProofs[2],
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([delegator1])
      .rpc();

    console.log("�� Delegator 1 override completed - Cache created");

    // Verify cache exists
    let cacheAccountInfo = await provider.connection.getAccountInfo(
      voteOverrideCacheAccount
    );
    expect(cacheAccountInfo).to.not.be.null;
    console.log("Cache account size:", cacheAccountInfo!.data.length, "bytes");

    console.log("\\n=== STEP 2: VALIDATOR VOTES ===");
    console.log("Expected: Should process cache (first if branch)");
    console.log(
      "Actual (with bug): Goes to else branch due to size validation bug"
    );

    // Check if validator vote account exists (should not exist yet)
    let validatorVoteInfo = await provider.connection.getAccountInfo(
      validatorVoteAccount
    );
    console.log(
      "Validator vote account exists before voting:",
      !!validatorVoteInfo
    );

    try {
      await program.methods
        .castVote(
          new anchor.BN(6000), // 60% for
          new anchor.BN(3000), // 30% against
          new anchor.BN(1000) // 10% abstain
        )
        .accountsPartial({
          signer: provider.publicKey,
          proposal: testAccounts.proposalAccount,
          vote: validatorVoteAccount,
          voteOverrideCache: voteOverrideCacheAccount,
          splVoteAccount: testAccounts.splVoteAccounts[2].publicKey,
          snapshotProgram: mockProgram.programId,
          consensusResult: testAccounts.consensusResult,
          metaMerkleProof: testAccounts.metaMerkleProofs[2],
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      console.log("✅ Validator vote completed");

      // Check validator vote account after voting
      validatorVoteInfo = await provider.connection.getAccountInfo(
        validatorVoteAccount
      );
      console.log(
        "Validator vote account exists after voting:",
        !!validatorVoteInfo
      );

      if (validatorVoteInfo) {
        console.log(
          "Validator vote account size:",
          validatorVoteInfo.data.length,
          "bytes"
        );

        // Fetch and analyze the vote account
        const validatorVote = await program.account.vote.fetch(
          validatorVoteAccount
        );
        console.log(
          "Validator vote override lamports:",
          validatorVote.overrideLamports.toString()
        );

        // BUG ANALYSIS: If size validation bug exists, override lamports should be 0
        // because the cache processing was skipped
        if (Number(validatorVote.overrideLamports) === 0) {
          console.log("🐛 SIZE VALIDATION BUG DETECTED!");
          console.log(
            "   Validator vote went to ELSE branch instead of cache processing IF branch"
          );
          console.log("   Cache was not processed due to incorrect size check");
        } else {
          console.log("✅ Cache was processed correctly");
        }
      }
    } catch (error: any) {
      console.log("❌ Validator vote failed:", error.message);
    }

    console.log("\\n=== STEP 3: DELEGATOR 2 TRIES TO VOTE ===");
    console.log(
      "Expected: Should update existing validator vote (first if branch)"
    );
    console.log(
      "Actual (with bug): Goes to else branch, tries to create cache again"
    );

    const stakeMerkleLeaf2 = {
      votingWallet: delegator2.publicKey,
      stakeAccount: stakeAccount2.publicKey,
      activeStake: new anchor.BN(200000000), // 0.2 SOL
    };

    const voteOverrideAccount2 = deriveVoteOverrideAccount(
      program,
      testAccounts.proposalAccount,
      stakeAccount2.publicKey,
      validatorVoteAccount
    );

    // Get validator vote data BEFORE delegator 2 attempts override
    let validatorVoteBeforeDel2: any = null;
    if (validatorVoteInfo) {
      validatorVoteBeforeDel2 = await program.account.vote.fetch(
        validatorVoteAccount
      );
      console.log("\\nValidator vote state BEFORE delegator 2:");
      console.log(
        "- Override lamports:",
        validatorVoteBeforeDel2.overrideLamports.toString()
      );
      console.log(
        "- For votes lamports:",
        validatorVoteBeforeDel2.forVotesLamports.toString()
      );
      console.log(
        "- Against votes lamports:",
        validatorVoteBeforeDel2.againstVotesLamports.toString()
      );
      console.log(
        "- Abstain votes lamports:",
        validatorVoteBeforeDel2.abstainVotesLamports.toString()
      );
    }

    try {
      await program.methods
        .castVoteOverride(
          new anchor.BN(2000), // 20% for
          new anchor.BN(6000), // 60% against
          new anchor.BN(2000), // 20% abstain
          [],
          stakeMerkleLeaf2
        )
        .accountsPartial({
          signer: delegator2.publicKey,
          proposal: testAccounts.proposalAccount,
          validatorVote: validatorVoteAccount,
          voteOverrideCache: voteOverrideCacheAccount,
          splVoteAccount: testAccounts.splVoteAccounts[2].publicKey,
          voteOverride: voteOverrideAccount2,
          splStakeAccount: stakeAccount2.publicKey,
          snapshotProgram: mockProgram.programId,
          consensusResult: testAccounts.consensusResult,
          metaMerkleProof: testAccounts.metaMerkleProofs[2],
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([delegator2])
        .rpc();

      console.log("✅ Delegator 2 override completed");

      // CRITICAL ANALYSIS: Check validator vote data AFTER delegator 2
      const validatorVoteAfterDel2 = await program.account.vote.fetch(
        validatorVoteAccount
      );
      console.log("\\nValidator vote state AFTER delegator 2:");
      console.log(
        "- Override lamports:",
        validatorVoteAfterDel2.overrideLamports.toString()
      );
      console.log(
        "- For votes lamports:",
        validatorVoteAfterDel2.forVotesLamports.toString()
      );
      console.log(
        "- Against votes lamports:",
        validatorVoteAfterDel2.againstVotesLamports.toString()
      );
      console.log(
        "- Abstain votes lamports:",
        validatorVoteAfterDel2.abstainVotesLamports.toString()
      );

      // EXECUTION PATH ANALYSIS
      console.log("\\n=== EXECUTION PATH ANALYSIS ===");

      if (validatorVoteBeforeDel2) {
        const overrideBefore = Number(validatorVoteBeforeDel2.overrideLamports);
        const overrideAfter = Number(validatorVoteAfterDel2.overrideLamports);
        const forBefore = Number(validatorVoteBeforeDel2.forVotesLamports);
        const forAfter = Number(validatorVoteAfterDel2.forVotesLamports);
        const againstBefore = Number(
          validatorVoteBeforeDel2.againstVotesLamports
        );
        const againstAfter = Number(
          validatorVoteAfterDel2.againstVotesLamports
        );

        console.log("Changes in validator vote account:");
        console.log(
          "- Override lamports change:",
          overrideAfter - overrideBefore,
          overrideAfter,
          overrideBefore
        );
        console.log(
          "- For votes lamports change:",
          forAfter - forBefore,
          forAfter,
          forBefore
        );
        console.log(
          "- Against votes lamports change:",
          againstAfter - againstBefore,
          againstAfter,
          againstBefore
        );

        // DETERMINE WHICH EXECUTION PATH WAS TAKEN
        if (overrideAfter > overrideBefore) {
          console.log("\\n✅ CORRECT PATH: First IF branch executed");
          console.log(
            "   - Delegator 2 successfully updated existing validator vote"
          );
          console.log("   - Validator's vote weights were recalculated");
          console.log(
            "   - Override lamports increased by delegator 2's stake"
          );

          // Verify the math: delegator 2 has 0.2 SOL = 200,000,000 lamports
          const expectedIncrease = 200000000;
          const actualIncrease = overrideAfter - overrideBefore;
          if (actualIncrease === expectedIncrease) {
            console.log(
              "   - Override increase matches delegator 2's stake ✅"
            );
          } else {
            console.log(
              "   - Override increase mismatch:",
              actualIncrease,
              "vs expected:",
              expectedIncrease
            );
          }
        } else {
          console.log("\\n🐛 WRONG PATH: ELSE branch executed");
          console.log(
            "   - Delegator 2 went to cache creation path instead of validator vote update"
          );
          console.log("   - Validator vote account was not modified");
          console.log("   - This indicates the size validation bug exists");
        }
      }

      // Check if delegator 2's individual override account was created
      const del2OverrideInfo = await provider.connection.getAccountInfo(
        voteOverrideAccount2
      );
      if (del2OverrideInfo) {
        const del2Override = await program.account.voteOverride.fetch(
          voteOverrideAccount2
        );
        console.log("\\nDelegator 2 override account created:");
        console.log("- Stake amount:", del2Override.stakeAmount.toString());
        console.log("- For votes BP:", del2Override.forVotesBp.toString());
        console.log(
          "- Against votes BP:",
          del2Override.againstVotesBp.toString()
        );
        console.log("- This confirms delegator 2's transaction completed");
      }
    } catch (error: any) {
      console.log("❌ Delegator 2 override failed:", error.message);

      if (error.message.includes("already in use")) {
        console.log("\\n🐛 SIZE VALIDATION BUG CONFIRMED!");
        console.log("   - Delegator 2 went to ELSE branch (cache creation)");
        console.log("   - Tried to create cache that already exists");
        console.log("   - This proves the size validation check failed");
      }
    }

    // FINAL COMPREHENSIVE ANALYSIS
    console.log("\\n=== COMPREHENSIVE BUG ANALYSIS ===");

    // Check cache account data to see if it contains both delegators
    cacheAccountInfo = await provider.connection.getAccountInfo(
      voteOverrideCacheAccount
    );
    if (cacheAccountInfo) {
      console.log("\\nCache account analysis:");
      console.log("- Size:", cacheAccountInfo.data.length, "bytes");
      console.log("- Owner:", cacheAccountInfo.owner.toBase58());

      // Parse cache data to see accumulated values
      try {
        const cacheData = cacheAccountInfo.data.slice(8);
        const view = new DataView(cacheData.buffer, cacheData.byteOffset);
        let offset = 96; // Skip pubkeys

        const forVotesBp = Number(view.getBigUint64(offset, true));
        offset += 8;
        const againstVotesBp = Number(view.getBigUint64(offset, true));
        offset += 8;
        const abstainVotesBp = Number(view.getBigUint64(offset, true));
        offset += 8;
        const forVotesLamports = Number(view.getBigUint64(offset, true));
        offset += 8;
        const againstVotesLamports = Number(view.getBigUint64(offset, true));
        offset += 8;
        const abstainVotesLamports = Number(view.getBigUint64(offset, true));
        offset += 8;
        const totalStake = Number(view.getBigUint64(offset, true));

        console.log("Cache accumulated data:");
        console.log("- For votes BP:", forVotesBp);
        console.log("- Against votes BP:", againstVotesBp);
        console.log("- Abstain votes BP:", abstainVotesBp);
        console.log("- Total stake:", totalStake);

        // Expected if both delegators hit cache:
        // Del1: 5000 for, 3000 against, 2000 abstain, 300M stake
        // Del2: 2000 for, 6000 against, 2000 abstain, 200M stake
        // Total: 7000 for, 9000 against, 4000 abstain, 500M stake

        if (totalStake === 300000000) {
          console.log("\\n🐛 CACHE ONLY HAS DELEGATOR 1 DATA");
          console.log("   - Delegator 2 did NOT update the cache");
          console.log(
            "   - This suggests delegator 2 went to validator vote update path"
          );
        } else if (totalStake === 500000000) {
          console.log("\\n✅ CACHE HAS BOTH DELEGATORS DATA");
          console.log("   - Both delegators updated the cache");
          console.log("   - This suggests both went to cache update path");
        } else {
          console.log("\\n❓ UNEXPECTED CACHE STATE");
          console.log("   - Total stake doesn't match expected patterns");
        }
      } catch (error) {
        console.log("Failed to parse cache data:", error);
      }
    }

    console.log("\\n=== BUG ANALYSIS SUMMARY ===");
    console.log("The size validation bug causes wrong execution paths:");
    console.log("1. cast_vote.rs: data_len() == VoteOverrideCache::INIT_SPACE");
    console.log("   - Actual: 161 bytes (8 + 153)");
    console.log("   - Expected: 153 bytes");
    console.log("   - Result: Goes to ELSE branch, skips cache processing");
    console.log("");
    console.log("2. cast_vote_override.rs: data_len() == Vote::INIT_SPACE");
    console.log("   - Actual: 161 bytes (8 + 153)");
    console.log("   - Expected: 153 bytes");
    console.log(
      "   - Result: Goes to ELSE branch, tries to create existing cache"
    );
    console.log("");
    console.log("Fix: Use (8 + INIT_SPACE) in all size validation checks");
  });
});
