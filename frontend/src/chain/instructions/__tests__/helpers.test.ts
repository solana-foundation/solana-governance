import type { Connection } from "@solana/web3.js";

// helpers.ts transitively imports EndpointContext -> env.ts (an ESM-only package Jest does not
// transform). computeProofCloseTimestamp does not use it, so stub it out to keep the unit isolated.
jest.mock("@/contexts/EndpointContext", () => ({
  RPC_URLS: { testnet: "http://localhost:8899" },
}));

import { computeProofCloseTimestamp } from "../helpers";

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
    // slotDelta = 1_264_000 - 500_000 = 764_000; 764_000 * 400 / 1000 = 305_600
    expect(ts).toBe(1_700_305_600);

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
    expect(ts).toBe(1_700_305_600);
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

    await expect(computeProofCloseTimestamp(connection, 102)).rejects.toThrow(
      /Failed to fetch a recent block time/
    );
    // MAX_ATTEMPTS = 8
    expect(getBlockTime).toHaveBeenCalledTimes(8);
  });
});
