import { PublicKey, Connection, Keypair } from "@solana/web3.js";
import { AnchorProvider, Program, BN } from "@coral-xyz/anchor";
import idl from "@/chain/idl/svmgov_program.json";
import govV1Idl from "@/chain/idl/gov-v1.json";
import {
  VoteAccountProofResponse,
  StakeAccountProofResponse,
  SNAPSHOT_PROGRAM_ID,
  StakeMerkleLeafRaw,
  StakeMerkleLeafConverted,
} from "./types";
import { AnchorWallet } from "@solana/wallet-adapter-react";
import { SvmgovProgram, GovV1 } from "../types";
import { RPC_URLS } from "@/contexts/EndpointContext";
import { DEFAULT_NCN_API_URL } from "@/lib/defaultNcnApiUrl";

// PDA derivation functions (based on test implementation)
export function deriveProposalPda(
  seed: BN,
  signer: PublicKey,
  programId: PublicKey
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("proposal"),
      seed.toArrayLike(Buffer, "le", 8),
      signer.toBuffer(),
    ],
    programId
  );
  return pda;
}

export function deriveProposalIndexPda(programId: PublicKey): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("index")],
    programId
  );
  return pda;
}

export function deriveGlobalConfigPda(programId: PublicKey): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_config")],
    programId
  );
  return pda;
}

export function deriveVotePda(
  proposal: PublicKey,
  voteAccount: PublicKey,
  programId: PublicKey
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vote"), proposal.toBuffer(), voteAccount.toBuffer()],
    programId
  );
  return pda;
}

export function deriveSupportPda(
  proposal: PublicKey,
  voteAccount: PublicKey,
  programId: PublicKey
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("support"), proposal.toBuffer(), voteAccount.toBuffer()],
    programId
  );
  return pda;
}

export function deriveVoteOverridePda(
  proposal: PublicKey,
  stakeAccount: PublicKey,
  validatorVote: PublicKey,
  programId: PublicKey
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("vote_override"),
      proposal.toBuffer(),
      stakeAccount.toBuffer(),
      validatorVote.toBuffer(),
    ],
    programId
  );
  return pda;
}

export function deriveVoteOverrideCachePda(
  proposal: PublicKey,
  vote: PublicKey,
  programId: PublicKey
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vote_override_cache"), proposal.toBuffer(), vote.toBuffer()],
    programId
  );
  return pda;
}

// Create program instance with wallet
export function createProgramWithWallet(
  wallet: AnchorWallet,
  endpoint?: string
) {
  // Use provided endpoint or default to devnet
  const rpcEndpoint = endpoint || RPC_URLS.testnet;
  const connection = new Connection(rpcEndpoint, "confirmed");

  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });

  const program = new Program(idl, provider) as Program<SvmgovProgram>;

  return program;
}

// Create program instance with wallet
export function createGovV1ProgramWithWallet(
  wallet: AnchorWallet,
  endpoint?: string
) {
  // Use provided endpoint or default to devnet
  const rpcEndpoint = endpoint || RPC_URLS.testnet;
  const connection = new Connection(rpcEndpoint, "confirmed");

  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });

  const program = new Program(govV1Idl, provider) as Program<GovV1>;

  return program;
}

// Create program instance with dummy wallet (just for data fetching)
export function createProgramWitDummyWallet(endpoint?: string) {
  // Use provided endpoint or default to devnet
  const rpcEndpoint = endpoint || RPC_URLS.testnet;
  const connection = new Connection(rpcEndpoint, "confirmed");

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const dummyWallet: any = {
    publicKey: Keypair.generate().publicKey,
    signAllTransactions: async () => {},
    signTransaction: async () => {},
  };

  const provider = new AnchorProvider(connection, dummyWallet, {
    commitment: "confirmed",
  });

  const program = new Program(idl, provider) as Program<SvmgovProgram>;

  return program;
}
// API helpers using the solgov.online service
export async function getVoteAccountProof(
  voteAccount: string,
  network: string,
  slot: number,
  ncnApiUrl?: string
): Promise<VoteAccountProofResponse> {
  const baseUrl = ncnApiUrl || DEFAULT_NCN_API_URL;
  const url = `${baseUrl}/proof/vote_account/${voteAccount}?network=${network}&slot=${slot}`;
  const response = await fetch(url);

  if (!response.ok) {
    throw new Error(`Failed to get vote account proof: ${response.statusText}`);
  }

  return await response.json();
}

export async function getStakeAccountProof(
  stakeAccount: string,
  network: string,
  slot: number,
  ncnApiUrl?: string
): Promise<StakeAccountProofResponse> {
  const baseUrl = ncnApiUrl || DEFAULT_NCN_API_URL;
  const url = `${baseUrl}/proof/stake_account/${stakeAccount}?network=${network}&slot=${slot}`;
  const response = await fetch(url);

  if (!response.ok) {
    throw new Error(
      `Failed to get stake account proof: ${response.statusText}`
    );
  }

  return await response.json();
}

// Generate PDAs from vote proof response
export function getMetaMerkleProofPda(
  proofResponse: VoteAccountProofResponse,
  snapshotProgramId: PublicKey = SNAPSHOT_PROGRAM_ID,
  consensusResult: PublicKey
): PublicKey {
  // Derive meta merkle proof PDA (this is typically derived from the vote account)
  const [metaMerkleProofPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("MetaMerkleProof"),
      consensusResult.toBuffer(),
      new PublicKey(proofResponse.meta_merkle_leaf.vote_account).toBuffer(),
    ],
    snapshotProgramId
  );

  return metaMerkleProofPda;
}

/**
 * Resolve the snapshot validator vote account from a stake proof, guarding against a verifier
 * response that omits the field. `vote_account` is typed as `string` but comes from an
 * unvalidated JSON response — an older backend that predates surfacing `vote_account` on the
 * stake-proof endpoint would leave it `undefined`, and `new PublicKey(undefined)` throws an opaque
 * "Invalid public key input" with no hint that the field is missing. This surfaces a clear,
 * actionable error instead.
 */
export function resolveSnapshotVoteAccount(
  stakeMerkleProof: StakeAccountProofResponse
): PublicKey {
  if (!stakeMerkleProof.vote_account) {
    throw new Error(
      "Stake account proof is missing the snapshot vote_account; the verifier service may be out of date"
    );
  }
  return new PublicKey(stakeMerkleProof.vote_account);
}

/**
 * Cross-check that a stake proof and a meta (vote-account) proof belong to the same snapshot
 * lineage before they are paired in an override vote. The override builders derive the vote
 * account from the stake proof's snapshot `vote_account` and fetch the meta proof for it, so these
 * should always agree; this is defense-in-depth against the verifier returning inconsistent
 * records and surfaces a clear client-side error instead of an opaque on-chain failure.
 *
 * The on-chain program enforces the same lineage: `meta_merkle_leaf.vote_account == spl_vote_account`
 * and the stake leaf must verify nested inside `meta_merkle_leaf.stake_merkle_root`.
 */
export function assertOverrideProofLineage(
  stakeMerkleProof: StakeAccountProofResponse,
  metaMerkleProof: VoteAccountProofResponse
): void {
  if (
    metaMerkleProof.meta_merkle_leaf.vote_account !== stakeMerkleProof.vote_account
  ) {
    throw new Error(
      `Override proof mismatch: stake proof's snapshot vote account ${stakeMerkleProof.vote_account} ` +
        `does not match meta proof vote account ${metaMerkleProof.meta_merkle_leaf.vote_account}`
    );
  }

  if (
    metaMerkleProof.meta_merkle_leaf.voting_wallet !==
    stakeMerkleProof.stake_merkle_leaf.voting_wallet
  ) {
    throw new Error(
      `Override proof mismatch: stake proof voting wallet ${stakeMerkleProof.stake_merkle_leaf.voting_wallet} ` +
        `does not match meta proof voting wallet ${metaMerkleProof.meta_merkle_leaf.voting_wallet}`
    );
  }
}

// Milliseconds per slot (matches solana_sdk::clock::DEFAULT_MS_PER_SLOT and the svmgov CLI's
// compute_vote_expiry_timestamp).
const MS_PER_SLOT = 400;
// Buffer added to forward-looking estimates so a slower-than-400ms network can't make the proof
// closable before voting truly ends. Matches the svmgov CLI's compute_vote_expiry_timestamp.
const BUFFER_PCT = 20; // tolerate an average of ~480ms/slot over the projection
const MIN_BUFFER_SECONDS = 3600; // floor for proposals ending near an epoch boundary

/**
 * Estimate the Unix timestamp (in seconds) at which voting expires for a proposal — i.e. the
 * start of `endEpoch`, after which voting is no longer valid. This is the value a newly created
 * `MetaMerkleProof` should use for its `close_timestamp`: until this time the proof cannot be
 * closed permissionlessly, so it survives for the whole voting window and an attacker cannot
 * delete the shared proof state between the init and vote steps. Mirrors the svmgov CLI's
 * `compute_vote_expiry_timestamp`.
 *
 * There is no on-chain expiry timestamp (the proposal only stores `end_epoch`), so this estimates
 * it: anchor to a recent confirmed block time and project forward to the start of `endEpoch` at
 * ~400ms/slot. If voting has already ended the result is in the past, which correctly allows
 * immediate permissionless close.
 *
 * The 400ms/slot projection assumes default slot times. When slots run slower the chain reaches
 * `endEpoch` later than estimated, so a bare estimate can land before voting ends and let the
 * proof be closed too soon. To stay safe we add a buffer to forward-looking estimates — a
 * percentage of the projected window (the error grows with distance to `endEpoch`) with a fixed
 * floor — but never to already-expired proposals, which keep a past timestamp and stay
 * immediately closable.
 */
export async function computeProofCloseTimestamp(
  connection: Connection,
  endEpoch: number
): Promise<number> {
  const info = await connection.getEpochInfo("confirmed");

  // Absolute slot at the start of `endEpoch` — the point at which voting expires.
  const epochStartSlot = info.absoluteSlot - info.slotIndex;
  const targetSlot =
    epochStartSlot + (endEpoch - info.epoch) * info.slotsInEpoch;

  // Anchor the estimate to a real block time and project forward. Whichever slot resolves is
  // used as the reference, so the slot delta (and thus the estimate) stays consistent.
  const [refSlot, refTime] = await blockTimeAtOrBefore(
    connection,
    info.absoluteSlot
  );
  const slotDelta = targetSlot - refSlot;

  // Math.trunc (toward zero) matches Rust's i64 integer division in the CLI helper, so an
  // already-expired proposal yields the same past timestamp in both code paths.
  const projectedSecs = Math.trunc((slotDelta * MS_PER_SLOT) / 1000);
  const buffer =
    projectedSecs > 0
      ? Math.max(
          Math.trunc((projectedSecs * BUFFER_PCT) / 100),
          MIN_BUFFER_SECONDS
        )
      : 0;

  return refTime + projectedSecs + buffer;
}

/**
 * Fetch a block time at or before `slot`, walking backwards over skipped slots (which have no
 * block and therefore no block time) until one resolves. Returns the `[slot, unixTimestamp]`
 * pair that succeeded. Mirrors the svmgov CLI helper of the same purpose.
 */
async function blockTimeAtOrBefore(
  connection: Connection,
  slot: number
): Promise<[number, number]> {
  const MAX_ATTEMPTS = 8;

  let candidate = slot;
  let lastTried = slot;
  let attempts = 0;
  let lastErr: unknown;
  for (let attempt = 0; attempt < MAX_ATTEMPTS; attempt++) {
    lastTried = candidate;
    attempts = attempt + 1;
    try {
      const blockTime = await connection.getBlockTime(candidate);
      if (blockTime !== null) {
        return [candidate, blockTime];
      }
      lastErr = new Error(`no block time available for slot ${candidate}`);
    } catch (e) {
      lastErr = e;
    }
    if (candidate === 0) break;
    candidate = Math.max(0, candidate - 1);
  }

  throw new Error(
    `Failed to fetch a recent block time (tried ${attempts} slots ending at ${lastTried}): ${String(
      lastErr
    )}`
  );
}

// Convert merkle proof strings to the format expected by the program
export function convertMerkleProofStrings(proofStrings: string[]): number[][] {
  return proofStrings.map((proof) =>
    Array.from(new PublicKey(proof).toBytes())
  );
}

// Convert stake merkle leaf data to IDL type
export function convertStakeMerkleLeafDataToIdlType(
  leafData: StakeMerkleLeafRaw
): StakeMerkleLeafConverted {
  // This is a placeholder - you'll need to implement the actual conversion
  // based on your IDL structure
  return {
    activeStake: new BN(`${leafData.active_stake}`),
    stakeAccount: new PublicKey(leafData.stake_account),
    votingWallet: new PublicKey(leafData.voting_wallet),
  };
}

// Validate vote basis points
export function validateVoteBasisPoints(
  forVotes: number,
  againstVotes: number,
  abstainVotes: number
): void {
  const total = forVotes + againstVotes + abstainVotes;
  if (total !== 10000) {
    throw new Error(`Total vote basis points must sum to 10000, got ${total}`);
  }
}

// Convert hex string to byte array
export function hexToBytes(hex: string): Uint8Array {
  const cleanHex = hex.startsWith("0x") ? hex.slice(2) : hex;
  if (cleanHex.length !== 64) {
    // 32 bytes * 2 hex chars per byte
    throw new Error(
      "Merkle root hash must be exactly 32 bytes (64 hex characters)"
    );
  }
  return new Uint8Array(Buffer.from(cleanHex, "hex"));
}
