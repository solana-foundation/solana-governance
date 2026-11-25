import {
  computeProofCloseTimestamp,
  ensureMetaMerkleProofInitialized,
} from "../metaMerkleProofSetup";

type Rpc = Parameters<typeof computeProofCloseTimestamp>[0];

const SLOTS_IN_EPOCH = 432_000n;
const NOW = 1_800_000_000n;

/** An RPC stub at `slotIndex` of epoch 900, with a block time for the given slots. */
function rpcAt(slotIndex: bigint, blockTimes: Record<string, bigint | null>) {
  const absoluteSlot = 900n * SLOTS_IN_EPOCH + slotIndex;
  const getBlockTime = jest.fn((slot: bigint) => ({
    send: () => {
      const blockTime = blockTimes[(slot - absoluteSlot).toString()];
      return blockTime === undefined
        ? Promise.reject(new Error("Block not available for slot"))
        : Promise.resolve(blockTime);
    },
  }));
  return {
    getBlockTime,
    rpc: {
      getBlockTime,
      getEpochInfo: () => ({
        send: () =>
          Promise.resolve({
            absoluteSlot,
            epoch: 900n,
            slotIndex,
            slotsInEpoch: SLOTS_IN_EPOCH,
          }),
      }),
    } as unknown as Rpc,
  };
}

describe("computeProofCloseTimestamp", () => {
  it("projects the start of the end epoch at 400ms slots and pads it by 20%", async () => {
    const { rpc } = rpcAt(0n, { "0": NOW });

    // Two full epochs away: 864,000 slots = 345,600s, plus a 69,120s buffer.
    await expect(computeProofCloseTimestamp(rpc, 902n)).resolves.toBe(
      NOW + 345_600n + 69_120n,
    );
  });

  it("pads by at least an hour when the vote ends soon", async () => {
    // 9,000 slots (one hour) before the end epoch starts: 20% would be only
    // 12 minutes, too little slack for a proof that must outlive the vote.
    const { rpc } = rpcAt(SLOTS_IN_EPOCH - 9_000n, { "0": NOW });

    await expect(computeProofCloseTimestamp(rpc, 901n)).resolves.toBe(
      NOW + 3_600n + 3_600n,
    );
  });

  it("falls back to an earlier slot when the latest one has no block time", async () => {
    // The tip can be a skipped slot (null) or not yet served by the RPC (error).
    const { rpc, getBlockTime } = rpcAt(0n, { "0": null, "-2": NOW });

    // Projected from slot -2: 432,002 slots = 172,800s (truncated) + 34,560s.
    await expect(computeProofCloseTimestamp(rpc, 901n)).resolves.toBe(
      NOW + 172_800n + 34_560n,
    );
    expect(getBlockTime).toHaveBeenCalledTimes(3);
  });

  it("fails rather than guess when no recent slot has a block time", async () => {
    const { rpc } = rpcAt(0n, {});

    await expect(computeProofCloseTimestamp(rpc, 901n)).rejects.toThrow(
      "Failed to fetch a recent block time",
    );
  });
});

describe("ensureMetaMerkleProofInitialized", () => {
  it("sends nothing when the proof account already exists", async () => {
    // Initialising an existing account used to be signed and sent as an empty
    // transaction, charging the delegator a fee for nothing.
    const initialize = jest.fn();

    await ensureMetaMerkleProofInitialized({
      initialize,
      isInitialized: () => Promise.resolve(true),
    });

    expect(initialize).not.toHaveBeenCalled();
  });

  it("carries on when another delegator created the proof first", async () => {
    // Every delegator of a validator shares one proof account, so two wallets
    // voting at once race to create it and the loser's transaction fails.
    const isInitialized = jest
      .fn()
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(true);

    await expect(
      ensureMetaMerkleProofInitialized({
        initialize: () => Promise.reject(new Error("account already in use")),
        isInitialized,
      }),
    ).resolves.toBeUndefined();
  });

  it("reports the failure when the proof is still missing afterwards", async () => {
    const failure = new Error("User rejected the request.");

    await expect(
      ensureMetaMerkleProofInitialized({
        initialize: () => Promise.reject(failure),
        isInitialized: () => Promise.resolve(false),
      }),
    ).rejects.toBe(failure);
  });
});
