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
  createEventListener,
  removeEventListener,
} from "./test-helpers";

describe.only("Cache Serialization Isolated Test", () => {
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

  it("Test Cache Serialization - Two Delegator Overrides Only", async () => {
    console.log("=== Testing Cache Serialization - Two Delegators Only ===");

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
        votePubkey: testAccounts.splVoteAccounts[2].publicKey, // Validator that hasn't voted yet
      });

      const delegateTx = new anchor.web3.Transaction().add(
        delegateStakeIx as any
      );
      await provider.sendAndConfirm(delegateTx, [delegator]);

      return stakeAccount;
    };

    const stakeAccount1 = await createStakeAccount(delegator1);
    const stakeAccount2 = await createStakeAccount(delegator2);

    // Get the validator vote account that doesn't exist yet (validator hasn't voted)
    const validatorVoteAccount = testAccounts.voteAccounts[0]; // This validator hasn't voted
    const voteOverrideCacheAccount = deriveVoteOverrideCacheAccount(
      program,
      testAccounts.proposalAccount,
      validatorVoteAccount
    );

    console.log(
      "Vote Override Cache Account:",
      voteOverrideCacheAccount.toBase58()
    );
    console.log("Validator Vote Account:", validatorVoteAccount.toBase58());

    // ===== FIRST DELEGATOR OVERRIDE =====
    console.log("\\n=== FIRST DELEGATOR OVERRIDE ===");
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

    console.log("Delegator 1 inputs:");
    console.log("- Stake: 0.3 SOL (300,000,000 lamports)");
    console.log("- Vote: 50% for, 30% against, 20% abstain");
    console.log("- Expected lamports: 150M for, 90M against, 60M abstain");

    const delegator1EventListener = createEventListener(
      program,
      "voteOverrideCast",
      (event: any, slot: number) => {
        console.log("Delegator 1 override event:");
        console.log("- For votes BP:", event.forVotesBp?.toString());
        console.log("- Against votes BP:", event.againstVotesBp?.toString());
        console.log("- Abstain votes BP:", event.abstainVotesBp?.toString());
        console.log(
          "- For votes lamports:",
          event.forVotesLamports?.toString()
        );
        console.log(
          "- Against votes lamports:",
          event.againstVotesLamports?.toString()
        );
        console.log(
          "- Abstain votes lamports:",
          event.abstainVotesLamports?.toString()
        );
        console.log("- Stake amount:", event.stakeAmount?.toString());
      }
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

    removeEventListener(program, delegator1EventListener);
    console.log("✅ First delegator override completed");

    // Check cache after first delegator
    let cacheAccountInfo = await provider.connection.getAccountInfo(
      voteOverrideCacheAccount
    );
    console.log("\\n--- Cache State After First Delegator ---");
    console.log("Cache exists:", !!cacheAccountInfo);

    if (cacheAccountInfo) {
      console.log("Cache data length:", cacheAccountInfo.data.length);
      console.log("Cache owner:", cacheAccountInfo.owner.toBase58());

      // Try to manually deserialize cache data
      try {
        const cacheData = cacheAccountInfo.data.slice(8); // Skip discriminator
        console.log(
          "Raw cache data (first 100 bytes):",
          Array.from(cacheData.slice(0, 100))
        );

        // Manual deserialization attempt (rough approximation)
        // VoteOverrideCache structure: validator(32) + proposal(32) + vote_account_validator(32) +
        // for_votes_bp(8) + against_votes_bp(8) + abstain_votes_bp(8) +
        // for_votes_lamports(8) + against_votes_lamports(8) + abstain_votes_lamports(8) + total_stake(8) + bump(1)

        const view = new DataView(cacheData.buffer, cacheData.byteOffset);
        let offset = 96; // Skip the 3 pubkeys (32 bytes each)

        const forVotesBp = view.getBigUint64(offset, true);
        offset += 8;
        const againstVotesBp = view.getBigUint64(offset, true);
        offset += 8;
        const abstainVotesBp = view.getBigUint64(offset, true);
        offset += 8;
        const forVotesLamports = view.getBigUint64(offset, true);
        offset += 8;
        const againstVotesLamports = view.getBigUint64(offset, true);
        offset += 8;
        const abstainVotesLamports = view.getBigUint64(offset, true);
        offset += 8;
        const totalStake = view.getBigUint64(offset, true);

        console.log("Parsed cache data after first delegator:");
        console.log("- For votes BP:", forVotesBp.toString());
        console.log("- Against votes BP:", againstVotesBp.toString());
        console.log("- Abstain votes BP:", abstainVotesBp.toString());
        console.log("- For votes lamports:", forVotesLamports.toString());
        console.log(
          "- Against votes lamports:",
          againstVotesLamports.toString()
        );
        console.log(
          "- Abstain votes lamports:",
          abstainVotesLamports.toString()
        );
        console.log("- Total stake:", totalStake.toString());
      } catch (error) {
        console.log("Failed to parse cache data:", error);
      }
    }

    // ===== SECOND DELEGATOR OVERRIDE =====
    console.log("\\n=== SECOND DELEGATOR OVERRIDE ===");
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

    console.log("Delegator 2 inputs:");
    console.log("- Stake: 0.2 SOL (200,000,000 lamports)");
    console.log("- Vote: 20% for, 60% against, 20% abstain");
    console.log("- Expected lamports: 40M for, 120M against, 40M abstain");

    const delegator2EventListener = createEventListener(
      program,
      "voteOverrideCast",
      (event: any, slot: number) => {
        console.log("Delegator 2 override event:");
        console.log("- For votes BP:", event.forVotesBp?.toString());
        console.log("- Against votes BP:", event.againstVotesBp?.toString());
        console.log("- Abstain votes BP:", event.abstainVotesBp?.toString());
        console.log(
          "- For votes lamports:",
          event.forVotesLamports?.toString()
        );
        console.log(
          "- Against votes lamports:",
          event.againstVotesLamports?.toString()
        );
        console.log(
          "- Abstain votes lamports:",
          event.abstainVotesLamports?.toString()
        );
        console.log("- Stake amount:", event.stakeAmount?.toString());
      }
    );

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

    removeEventListener(program, delegator2EventListener);
    console.log("✅ Second delegator override completed");

    // Check cache after second delegator
    cacheAccountInfo = await provider.connection.getAccountInfo(
      voteOverrideCacheAccount
    );
    console.log("\\n--- Cache State After Second Delegator ---");
    console.log("Cache still exists:", !!cacheAccountInfo);

    if (cacheAccountInfo) {
      console.log("Cache data length:", cacheAccountInfo.data.length);

      try {
        const cacheData = cacheAccountInfo.data.slice(8);
        console.log(
          "Raw cache data (first 100 bytes):",
          Array.from(cacheData.slice(0, 100))
        );

        const view = new DataView(cacheData.buffer, cacheData.byteOffset);
        let offset = 96; // Skip the 3 pubkeys

        const forVotesBp = view.getBigUint64(offset, true);
        offset += 8;
        const againstVotesBp = view.getBigUint64(offset, true);
        offset += 8;
        const abstainVotesBp = view.getBigUint64(offset, true);
        offset += 8;
        const forVotesLamports = view.getBigUint64(offset, true);
        offset += 8;
        const againstVotesLamports = view.getBigUint64(offset, true);
        offset += 8;
        const abstainVotesLamports = view.getBigUint64(offset, true);
        offset += 8;
        const totalStake = view.getBigUint64(offset, true);

        console.log("Parsed cache data after second delegator:");
        console.log("- For votes BP:", forVotesBp.toString());
        console.log("- Against votes BP:", againstVotesBp.toString());
        console.log("- Abstain votes BP:", abstainVotesBp.toString());
        console.log("- For votes lamports:", forVotesLamports.toString());
        console.log(
          "- Against votes lamports:",
          againstVotesLamports.toString()
        );
        console.log(
          "- Abstain votes lamports:",
          abstainVotesLamports.toString()
        );
        console.log("- Total stake:", totalStake.toString());
      } catch (error) {
        console.log("Failed to parse cache data:", error);
      }
    }

    // ===== ANALYSIS =====
    console.log("\\n=== CACHE SERIALIZATION BUG ANALYSIS ===");

    console.log("Expected accumulated values if cache serialization works:");
    console.log("- Total stake: 500,000,000 lamports (0.3 + 0.2 SOL)");
    console.log("- For votes BP: 7000 (5000 + 2000)");
    console.log("- Against votes BP: 9000 (3000 + 6000)");
    console.log("- Abstain votes BP: 4000 (2000 + 2000)");
    console.log("- For votes lamports: 190,000,000 (150M + 40M)");
    console.log("- Against votes lamports: 210,000,000 (90M + 120M)");
    console.log("- Abstain votes lamports: 100,000,000 (60M + 40M)");

    console.log("\\nIf cache serialization bug exists:");
    console.log("- Cache will only contain first delegator's data");
    console.log("- Second delegator's updates will be lost");
    console.log(
      "- Raw cache data bytes will be identical before and after second override"
    );
  });
});
