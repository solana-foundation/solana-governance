import type { Connection } from "@solana/web3.js";

// helpers.ts transitively imports EndpointContext -> env.ts (an ESM-only package Jest does not
// transform). computeProofCloseTimestamp does not use it, so stub it out to keep the unit isolated.
jest.mock("@/contexts/EndpointContext", () => ({
  RPC_URLS: { testnet: "http://localhost:8899" },
}));

import { BN } from "@coral-xyz/anchor";
import {
  PublicKey,
  TransactionExpiredBlockheightExceededError,
} from "@solana/web3.js";

import {
  assertMetaMerkleProofInitialized,
  assertOverrideProofLineage,
  confirmTransactionByPolling,
  computeProofCloseTimestamp,
  resolveProposalSnapshotSlot,
  resolveSnapshotVoteAccount,
} from "../helpers";
import type {
  StakeAccountProofResponse,
  VoteAccountProofResponse,
} from "../types";

describe("confirmTransactionByPolling", () => {
  it("returns after an HTTP status reaches confirmed", async () => {
    const getSignatureStatuses = jest.fn().mockResolvedValue({
      value: [
        {
          confirmationStatus: "confirmed",
          confirmations: 1,
          err: null,
          slot: 123,
        },
      ],
    });
    const connection = { getSignatureStatuses } as unknown as Connection;

    await expect(
      confirmTransactionByPolling(connection, "signature", 123),
    ).resolves.toEqual({ value: { err: null } });
    expect(getSignatureStatuses).toHaveBeenCalledWith(["signature"]);
  });

  it("returns the transaction error from status polling", async () => {
    const err = { InstructionError: [0, "Custom"] };
    const connection = {
      getSignatureStatuses: jest.fn().mockResolvedValue({
        value: [{ confirmationStatus: "confirmed", confirmations: 1, err }],
      }),
    } as unknown as Connection;

    await expect(
      confirmTransactionByPolling(connection, "signature", 123),
    ).resolves.toEqual({ value: { err } });
  });

  it("stops polling only after the transaction's blockhash expires", async () => {
    const getSignatureStatuses = jest.fn().mockResolvedValue({ value: [null] });
    const getBlockHeight = jest.fn().mockResolvedValue(124);
    const connection = {
      getSignatureStatuses,
      getBlockHeight,
    } as unknown as Connection;

        await expect(
      confirmTransactionByPolling(connection, "signature", 123),
    ).rejects.toBeInstanceOf(TransactionExpiredBlockheightExceededError);
    expect(getBlockHeight).toHaveBeenCalledWith("confirmed");
  });
});

describe("assertMetaMerkleProofInitialized", () => {
  const proofPda = new PublicKey(new Uint8Array(32).fill(9));
  const initializationError = { InstructionError: [0, "Custom"] };

  it("accepts a failed initialization when a competing transaction created the proof", async () => {
    const getAccountInfo = jest.fn().mockResolvedValue({ data: Buffer.alloc(0) });
    const connection = { getAccountInfo } as unknown as Connection;

    await expect(
      assertMetaMerkleProofInitialized(
        connection,
        proofPda,
        initializationError
      )
    ).resolves.toBeUndefined();
    expect(getAccountInfo).toHaveBeenCalledWith(proofPda, "confirmed");
  });

  it("preserves an initialization failure when the proof is still absent", async () => {
    const getAccountInfo = jest.fn().mockResolvedValue(null);
    const connection = { getAccountInfo } as unknown as Connection;

    await expect(
      assertMetaMerkleProofInitialized(
        connection,
        proofPda,
        initializationError
      )
    ).rejects.toThrow(/Failed to initialize meta merkle proof/);
  });

  it("does not issue an account read after successful initialization", async () => {
    const getAccountInfo = jest.fn();
    const connection = { getAccountInfo } as unknown as Connection;

    await expect(
      assertMetaMerkleProofInitialized(connection, proofPda, null)
    ).resolves.toBeUndefined();
    expect(getAccountInfo).not.toHaveBeenCalled();
  });
});

/**
 * Builds a minimal Connection stand-in exposing only the two methods
 * computeProofCloseTimestamp uses.
 */
function mockConnection(
  epochInfo: {
    absoluteSlot: number;
    slotIndex: number;
    epoch: number;
    slotsInEpoch: number;
  },
  getBlockTime: (slot: number) => Promise<number | null>
): { connection: Connection; getBlockTime: jest.Mock } {
  const getBlockTimeMock = jest.fn(getBlockTime);
  const connection = {
    getEpochInfo: jest.fn(async () => epochInfo),
    getBlockTime: getBlockTimeMock,
  } as unknown as Connection;
  return { connection, getBlockTime: getBlockTimeMock };
}

describe("computeProofCloseTimestamp", () => {
  it("projects a future end_epoch forward to a real future timestamp (never the vulnerable 1)", async () => {
    const { connection } = mockConnection(
      {
        absoluteSlot: 500_000,
        slotIndex: 100_000,
        epoch: 100,
        slotsInEpoch: 432_000,
      },
      async () => 1_700_000_000
    );

    const ts = await computeProofCloseTimestamp(connection, 102);

    // epochStartSlot = 400_000; targetSlot = 400_000 + 2 * 432_000 = 1_264_000
    // slotDelta = 1_264_000 - 500_000 = 764_000; projected = 764_000 * 400 / 1000 = 305_600
    // buffer = max(305_600 * 20 / 100, 3600) = 61_120; result = 305_600 + 61_120 = 366_720
    expect(ts).toBe(1_700_366_720);

    // Security regression guard: the shipped frontend used to hard-code 1, which made the proof
    // permissionlessly closable immediately. The fix must produce the real vote-expiry instant.
    expect(ts).toBeGreaterThan(1);
    expect(ts).toBeGreaterThan(1_700_000_000);
  });

  it("walks backwards over a skipped reference slot until a block time resolves", async () => {
    const { connection, getBlockTime } = mockConnection(
      {
        absoluteSlot: 500_000,
        slotIndex: 100_000,
        epoch: 100,
        slotsInEpoch: 432_000,
      },
      async (slot: number) => (slot === 500_000 ? null : 1_700_000_000)
    );

    const ts = await computeProofCloseTimestamp(connection, 102);

    expect(getBlockTime).toHaveBeenNthCalledWith(1, 500_000);
    expect(getBlockTime).toHaveBeenNthCalledWith(2, 499_999);
    // refSlot = 499_999; slotDelta = 1_264_000 - 499_999 = 764_001; *400/1000 = 305_600.4 -> 305_600
    // buffer = max(305_600 * 20 / 100, 3600) = 61_120; result = 305_600 + 61_120 = 366_720
    expect(ts).toBe(1_700_366_720);
  });

  it("returns a past timestamp once voting has already ended (allows immediate permissionless close)", async () => {
    const { connection } = mockConnection(
      {
        absoluteSlot: 1_000_000,
        slotIndex: 50_000,
        epoch: 200,
        slotsInEpoch: 432_000,
      },
      async () => 1_700_000_000
    );

    const ts = await computeProofCloseTimestamp(connection, 199);

    // epochStartSlot = 950_000; targetSlot = 950_000 - 432_000 = 518_000
    // slotDelta = 518_000 - 1_000_000 = -482_000; -482_000 * 400 / 1000 = -192_800
    // projected <= 0, so no buffer is added and the result stays in the past (immediately closable)
    expect(ts).toBe(1_699_807_200);
    expect(ts).toBeLessThan(1_700_000_000);
  });

  it("throws if no block time resolves within the attempt budget", async () => {
    const { connection, getBlockTime } = mockConnection(
      {
        absoluteSlot: 500_000,
        slotIndex: 100_000,
        epoch: 100,
        slotsInEpoch: 432_000,
      },
      async () => null
    );

    // Walks back from absoluteSlot (500_000) over MAX_ATTEMPTS = 8 slots, so the
    // lowest slot actually tried — and thus the reported "ending at" — is 499_993.
    await expect(computeProofCloseTimestamp(connection, 102)).rejects.toThrow(
      /Failed to fetch a recent block time \(tried 8 slots ending at 499993\)/
    );
    expect(getBlockTime).toHaveBeenCalledTimes(8);
  });
});

describe("assertOverrideProofLineage", () => {
  // Snapshot validator the stake was delegated to at snapshot time.
  const SNAPSHOT_VOTE_ACCOUNT = "SnapshotVoteAccount11111111111111111111111";
  // A different validator the stake was redelegated to after the snapshot.
  const LIVE_VOTE_ACCOUNT = "LiveVoteAccount2222222222222222222222222222";
  // Snapshot generation gives the meta leaf the validator identity and the stake leaf the
  // delegator withdrawer (or stake-pool manager). These are distinct in the ordinary case.
  const VALIDATOR_WALLET = "ValidatorWallet3333333333333333333333333333";
  const DELEGATOR_WALLET = "DelegatorWallet4444444444444444444444444444";

  function stakeProof(
    overrides: Partial<StakeAccountProofResponse> = {}
  ): StakeAccountProofResponse {
    return {
      network: "testnet",
      snapshot_slot: 340_850_340,
      stake_merkle_leaf: {
        active_stake: 500,
        stake_account: "StakeAccount4444444444444444444444444444444",
        voting_wallet: DELEGATOR_WALLET,
      },
      stake_merkle_proof: [],
      vote_account: SNAPSHOT_VOTE_ACCOUNT,
      ...overrides,
    };
  }

  function metaProof(
    overrides: Partial<VoteAccountProofResponse["meta_merkle_leaf"]> = {}
  ): VoteAccountProofResponse {
    return {
      network: "testnet",
      snapshot_slot: 340_850_340,
      meta_merkle_leaf: {
        active_stake: 500,
        stake_merkle_root: "StakeMerkleRoot55555555555555555555555555555",
        vote_account: SNAPSHOT_VOTE_ACCOUNT,
        voting_wallet: VALIDATOR_WALLET,
        ...overrides,
      },
      meta_merkle_proof: [],
    };
  }

  it("passes when proofs share the snapshot vote account, even if voting wallets differ", () => {
    expect(VALIDATOR_WALLET).not.toBe(DELEGATOR_WALLET);
    expect(() =>
      assertOverrideProofLineage(stakeProof(), metaProof())
    ).not.toThrow();
  });

  it("throws when the meta proof is for a different (live) validator than the stake snapshot", () => {
    // This is the redelegation case: pairing the live validator's meta proof with the snapshot
    // stake proof must be rejected client-side rather than failing opaquely on-chain.
    expect(() =>
      assertOverrideProofLineage(
        stakeProof(),
        metaProof({ vote_account: LIVE_VOTE_ACCOUNT })
      )
    ).toThrow(/does not match meta proof vote account/);
  });

  it("does not throw when the voting wallets disagree", () => {
    // Ordinary override: validator identity on the meta leaf, delegator withdrawer on the
    // stake leaf. The on-chain handlers only require the stake wallet to match the signer.
    expect(() =>
      assertOverrideProofLineage(
        stakeProof(),
        metaProof({ voting_wallet: "OtherWallet66666666666666666666666666666666" })
      )
    ).not.toThrow();
  });
});

describe("resolveSnapshotVoteAccount", () => {
  const VOTE_ACCOUNT = new PublicKey(new Uint8Array(32).fill(1)).toBase58();

  function stakeProof(
    overrides: Partial<StakeAccountProofResponse> = {}
  ): StakeAccountProofResponse {
    return {
      network: "testnet",
      snapshot_slot: 340_850_340,
      stake_merkle_leaf: {
        active_stake: 500,
        stake_account: new PublicKey(new Uint8Array(32).fill(3)).toBase58(),
        voting_wallet: new PublicKey(new Uint8Array(32).fill(4)).toBase58(),
      },
      stake_merkle_proof: [],
      vote_account: VOTE_ACCOUNT,
      ...overrides,
    };
  }

  it("returns the snapshot vote account as a PublicKey", () => {
    const resolved = resolveSnapshotVoteAccount(stakeProof());
    expect(resolved.toBase58()).toBe(VOTE_ACCOUNT);
  });

  it("throws a clear error when the verifier omits vote_account", () => {
    // An older verifier build that predates surfacing vote_account on the stake-proof endpoint
    // leaves the field undefined at runtime; surface that explicitly rather than letting
    // `new PublicKey(undefined)` throw an opaque "Invalid public key input".
    const proof = stakeProof({
      vote_account: undefined as unknown as string,
    });
    expect(() => resolveSnapshotVoteAccount(proof)).toThrow(
      /missing the snapshot vote_account/
    );
  });
});

describe("resolveProposalSnapshotSlot", () => {
  it("returns the proposal's on-chain snapshot slot", () => {
    expect(resolveProposalSnapshotSlot(new BN(340_850_340))).toBe(340_850_340);
  });

  it("throws when voting has not been activated (slot is 0)", () => {
    expect(() => resolveProposalSnapshotSlot(new BN(0))).toThrow(
      /no snapshot slot/
    );
  });
});
