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

export function deriveGlobalConfigPda(programId: PublicKey): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_config")],
    programId
  );
  return pda;
}

export async function fetchGlobalConfig(program: Program<SvmgovProgram>) {
  const pda = deriveGlobalConfigPda(program.programId);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return await (program.account as any).globalConfig.fetch(pda);
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
// TODO: fix dupped ncn api urls
const DEFAULT_NCN_API_URL = "https://ncn.brewlabs.so";

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
