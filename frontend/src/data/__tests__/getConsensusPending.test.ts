import { Connection, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { getConsensusPending } from "../getProposals";
import type { RawProposalAccount } from "@/types";

const CURRENT_EPOCH = 805;

function proposal(
  overrides: Partial<{
    voting: boolean;
    finalized: boolean;
    consensusResult: PublicKey | null;
    startEpoch: number;
    endEpoch: number;
  }> = {},
): RawProposalAccount {
  const { startEpoch = 805, endEpoch = 808, ...rest } = overrides;
  return {
    publicKey: PublicKey.unique(),
    account: {
      voting: true,
      finalized: false,
      consensusResult: PublicKey.unique(),
      startEpoch: new BN(startEpoch),
      endEpoch: new BN(endEpoch),
      ...rest,
    } as unknown as RawProposalAccount["account"],
  };
}

describe("getConsensusPending", () => {
  it("only marks proposals whose ConsensusResult account is missing", async () => {
    const reached = proposal();
    const pending = proposal();
    const getAccountInfo = jest.fn(async (pk: PublicKey) =>
      pk.equals(reached.account.consensusResult as PublicKey)
        ? { data: Buffer.alloc(8), lamports: 1 }
        : null,
    );
    const connection = { getAccountInfo } as unknown as Connection;

    const result = await getConsensusPending(
      connection,
      [reached, pending],
      CURRENT_EPOCH,
    );

    expect(result.has(reached.publicKey.toBase58())).toBe(false);
    expect(result.has(pending.publicKey.toBase58())).toBe(true);
    expect(getAccountInfo).toHaveBeenCalledTimes(2);
  });

  it("does not query proposals outside their voting window", async () => {
    const getAccountInfo = jest.fn(async () => null);
    const connection = { getAccountInfo } as unknown as Connection;

    const result = await getConsensusPending(
      connection,
      [
        proposal({ voting: false }),
        proposal({ finalized: true }),
        proposal({ consensusResult: null }),
        proposal({ startEpoch: 806 }), // voting not started
        proposal({ endEpoch: 805 }), // voting ended, awaiting finalize
      ],
      CURRENT_EPOCH,
    );

    expect(result.size).toBe(0);
    expect(getAccountInfo).not.toHaveBeenCalled();
  });

  it("retries a failed lookup before reporting it as pending", async () => {
    const flaky = proposal();
    let calls = 0;
    const getAccountInfo = jest.fn(async () => {
      calls++;
      if (calls === 1) throw new Error("429 Too Many Requests");
      return { data: Buffer.alloc(8), lamports: 1 };
    });
    const connection = { getAccountInfo } as unknown as Connection;

    const result = await getConsensusPending(
      connection,
      [flaky],
      CURRENT_EPOCH,
    );

    expect(result.size).toBe(0);
    expect(getAccountInfo).toHaveBeenCalledTimes(2);
  });

  it("reports a persistently failing lookup as pending instead of failing the list", async () => {
    const failing = proposal();
    const pending = proposal();
    const getAccountInfo = jest.fn(async (pk: PublicKey) => {
      if (pk.equals(failing.account.consensusResult as PublicKey)) {
        throw new Error("429 Too Many Requests");
      }
      return null;
    });
    const connection = { getAccountInfo } as unknown as Connection;

    const result = await getConsensusPending(
      connection,
      [failing, pending],
      CURRENT_EPOCH,
    );

    expect(result.has(failing.publicKey.toBase58())).toBe(true);
    expect(result.has(pending.publicKey.toBase58())).toBe(true);
    expect(getAccountInfo).toHaveBeenCalledTimes(4); // 3 attempts + 1
  });
});
