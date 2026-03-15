import * as anchor from "@coral-xyz/anchor";
import { Govcontract } from "../target/types/govcontract";
import { MockGovV1 } from "../target/types/mock_gov_v1";
import {
  Connection,
  sendAndConfirmTransaction,
  SystemProgram,
  Transaction,
  VersionedTransaction,
  TransactionMessage,
  LAMPORTS_PER_SOL,
  StakeProgram,
} from "@solana/web3.js";
import { VoteInit, VoteProgram } from "@solana/web3.js";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import {
  BALLOT_ID,
  MERKLE_ROOT_HASH,
  createTestLeaf,
  createTestProof
} from "./test-constants";
import {
  deriveConsensusResultAccount,
  deriveMetaMerkleProofAccount,
} from "./test-helpers";

export interface TestAccounts {
  splVoteAccounts: anchor.web3.Keypair[];
  proposalIndexAccount: anchor.web3.PublicKey;
  proposalAccount: anchor.web3.PublicKey;
  supportAccount: anchor.web3.PublicKey;
  voteAccounts: anchor.web3.PublicKey[];
  consensusResult: anchor.web3.PublicKey;
  metaMerkleProofs: anchor.web3.PublicKey[];
}

// Create SPL vote accounts
export async function createSPLVoteAccounts(
  provider: anchor.AnchorProvider,
  count: number = 5
): Promise<anchor.web3.Keypair[]> {
  const splVoteAccounts = Array.from({ length: count }, () =>
    anchor.web3.Keypair.generate()
  );

  const space = VoteProgram.space;
  const lamports = await provider.connection.getMinimumBalanceForRentExemption(space);
  const extraLamports = 1_000_000_000; // 1 SOL

  const batchSize = 2;
  const signatures: string[] = [];

  console.log(`Creating ${count} SPL Vote Accounts in batches...`);

  for (let batchStart = 0; batchStart < splVoteAccounts.length; batchStart += batchSize) {
    const batchEnd = Math.min(batchStart + batchSize, splVoteAccounts.length);
    const batch = splVoteAccounts.slice(batchStart, batchEnd);

    console.log(`Processing batch: accounts ${batchStart + 1}-${batchEnd}`);

    const instructions = [];
    const signers = [(provider.wallet as NodeWallet).payer];

    for (const account of batch) {
      signers.push(account);

      const createAccountIx = SystemProgram.createAccount({
        fromPubkey: provider.publicKey,
        newAccountPubkey: account.publicKey,
        space,
        lamports: lamports + extraLamports,
        programId: VoteProgram.programId,
      });

      const commission = 1;
      const voteInit = new VoteInit(
        provider.publicKey,
        provider.publicKey,
        provider.publicKey,
        commission
      );
      const initializeIx = VoteProgram.initializeAccount({
        votePubkey: account.publicKey,
        nodePubkey: provider.publicKey,
        voteInit,
      });

      instructions.push(createAccountIx, initializeIx);
    }

    const latestBlockhash = await provider.connection.getLatestBlockhash();
    const message = new TransactionMessage({
      payerKey: provider.publicKey,
      recentBlockhash: latestBlockhash.blockhash,
      instructions,
    }).compileToV0Message();

    const transaction = new VersionedTransaction(message);
    transaction.sign(signers);

    const signature = await provider.connection.sendTransaction(transaction);
    await provider.connection.confirmTransaction({
      signature,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    }, "confirmed");

    signatures.push(signature);
    console.log(`Batch ${Math.floor(batchStart / batchSize) + 1} completed, signature: ${signature}`);
  }

  console.log("All SPL Vote Accounts Created Successfully!");

  // Verify all accounts are owned by the Vote program
  for (let i = 0; i < splVoteAccounts.length; i++) {
    const account = splVoteAccounts[i];
    const accountInfo = await provider.connection.getAccountInfo(account.publicKey);
    if (!accountInfo) {
      throw new Error(`SPL Vote account ${i + 1} not found after creation`);
    }
    if (!accountInfo.owner.equals(VoteProgram.programId)) {
      throw new Error(`SPL Vote account ${i + 1} not owned by Vote program`);
    }
    console.log(`SPL Vote Account ${i + 1}: ${account.publicKey.toBase58()}`);
  }

  console.log("All SPL Vote Accounts owned by Vote program!");
  return splVoteAccounts;
}

// Fund SPL vote accounts
export async function fundSPLVoteAccounts(
  provider: anchor.AnchorProvider,
  splVoteAccounts: anchor.web3.Keypair[],
  lamportsToSend: number = 10_000_000
): Promise<void> {
  const transferTransaction = new anchor.web3.Transaction();

  for (const account of splVoteAccounts) {
    transferTransaction.add(
      SystemProgram.transfer({
        fromPubkey: provider.publicKey,
        toPubkey: account.publicKey,
        lamports: lamportsToSend,
      })
    );
  }

  const tx = await anchor.web3.sendAndConfirmTransaction(
    provider.connection,
    transferTransaction,
    [(provider.wallet as NodeWallet).payer]
  );

  console.log("Dummy Validator SPLVote Accounts Funded Successfully!");
  console.log("Transaction signature:", tx);
}

// Initialize proposal index
export async function initializeProposalIndex(
  program: anchor.Program<Govcontract>,
  proposalIndexAccount: anchor.web3.PublicKey
): Promise<void> {
  const tx = await program.methods
    .initializeIndex()
    .accountsPartial({
      signer: program.provider.publicKey,
      proposalIndex: proposalIndexAccount,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("Proposal Index Initialized Successfully!");
  console.log("Transaction signature:", tx);

  const indexAccount = await program.account.proposalIndex.fetch(proposalIndexAccount);
  console.log("Current Index:", indexAccount.currentIndex.toString());
}

// Create consensus result
export async function createConsensusResult(
  mockProgram: anchor.Program<MockGovV1>,
  consensusResult: anchor.web3.PublicKey
): Promise<void> {
  const snapshotHash = Array.from(anchor.web3.Keypair.generate().publicKey.toBytes());

  const tx1 = await mockProgram.methods
    .createConsensusResult(BALLOT_ID, MERKLE_ROOT_HASH, snapshotHash)
    .accounts({
      consensusResult,
      payer: mockProgram.provider.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("ConsensusResult created, signature:", tx1);

  const result = await mockProgram.account.consensusResult.fetch(consensusResult);
  console.log("ConsensusResult:", result);
}

// Create meta merkle proofs
export async function createMetaMerkleProofs(
  mockProgram: anchor.Program<MockGovV1>,
  consensusResult: anchor.web3.PublicKey,
  splVoteAccounts: anchor.web3.Keypair[]
): Promise<void> {
  const leaves = splVoteAccounts.map((account, index) =>
    createTestLeaf(mockProgram.provider.publicKey, account.publicKey)
  );
  const proofs = Array.from({ length: splVoteAccounts.length }, () => createTestProof());

  for (let i = 0; i < splVoteAccounts.length; i++) {
    const metaMerkleProof = deriveMetaMerkleProofAccount(
      mockProgram,
      consensusResult,
      splVoteAccounts[i].publicKey
    );

    await mockProgram.methods
      .initMetaMerkleProof(leaves[i], proofs[i])
      .accounts({
        metaMerkleProof,
        consensusResult,
        payer: mockProgram.provider.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
  }

  console.log("MetaMerkleProof Accounts Created Successfully!");
}

// Complete test setup
export async function setupTestEnvironment(
  program: anchor.Program<Govcontract>,
  mockProgram: anchor.Program<MockGovV1>,
  seed: anchor.BN
): Promise<TestAccounts> {
  const provider = program.provider as anchor.AnchorProvider;

  // Create SPL vote accounts
  const splVoteAccounts = await createSPLVoteAccounts(provider);

  // Fund accounts
  await fundSPLVoteAccounts(provider, splVoteAccounts);

  // Derive accounts
  const proposalIndexAccount = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("index")],
    program.programId
  )[0];

  const proposalAccount = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("proposal"),
      seed.toArrayLike(Buffer, "le", 8),
      splVoteAccounts[0].publicKey.toBuffer(),
    ],
    program.programId
  )[0];

  const supportAccount = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("support"),
      proposalAccount.toBuffer(),
      splVoteAccounts[1].publicKey.toBuffer(),
    ],
    program.programId
  )[0];

  const voteAccounts = splVoteAccounts.slice(2).map(account =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vote"),
        proposalAccount.toBuffer(),
        account.publicKey.toBuffer(),
      ],
      program.programId
    )[0]
  );

  const consensusResult = deriveConsensusResultAccount(mockProgram);
  const metaMerkleProofs = splVoteAccounts.map(account =>
    deriveMetaMerkleProofAccount(mockProgram, consensusResult, account.publicKey)
  );

  // Initialize components
  await initializeProposalIndex(program, proposalIndexAccount);
  await createConsensusResult(mockProgram, consensusResult);
  await createMetaMerkleProofs(mockProgram, consensusResult, splVoteAccounts);

  return {
    splVoteAccounts,
    proposalIndexAccount,
    proposalAccount,
    supportAccount,
    voteAccounts,
    consensusResult,
    metaMerkleProofs,
  };
}
