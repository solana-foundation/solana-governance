import { getAddressEncoder, type Address } from "@solana/kit";
import { decodeWalletStakeAccount } from "../stakeAccounts";

const ACCOUNT = "Stake11111111111111111111111111111111111111" as Address;
const STAKER = "11111111111111111111111111111112" as Address;
const WITHDRAWER = "11111111111111111111111111111113" as Address;
const VOTER = "Vote111111111111111111111111111111111111111" as Address;
const NEVER_DEACTIVATED = 18_446_744_073_709_551_615n;

/** Raw StakeStateV2 bytes as the stake program lays them out. */
function stakeAccountData(
  variant: number,
  delegation?: { deactivationEpoch: bigint; stake: bigint },
) {
  const data = new Uint8Array(200);
  const view = new DataView(data.buffer);
  view.setUint32(0, variant, true);
  view.setBigUint64(4, 2_282_880n, true);
  data.set(getAddressEncoder().encode(STAKER), 12);
  data.set(getAddressEncoder().encode(WITHDRAWER), 44);
  if (delegation) {
    data.set(getAddressEncoder().encode(VOTER), 124);
    view.setBigUint64(156, delegation.stake, true);
    view.setBigUint64(164, 700n, true);
    view.setBigUint64(172, delegation.deactivationEpoch, true);
  }
  return data;
}

describe("decodeWalletStakeAccount", () => {
  it("reads the authorities, voter and stake of a delegated account", () => {
    const data = stakeAccountData(2, {
      deactivationEpoch: NEVER_DEACTIVATED,
      stake: 5_000_000_000n,
    });

    expect(decodeWalletStakeAccount(ACCOUNT, data, 900n)).toEqual({
      activeStakeLamports: 5_000_000_000n,
      address: ACCOUNT,
      staker: STAKER,
      state: "delegated",
      voter: VOTER,
      withdrawer: WITHDRAWER,
    });
  });

  it("distinguishes an account that is deactivating from one that has cooled down", () => {
    const deactivatedAt = (epoch: bigint) =>
      decodeWalletStakeAccount(
        ACCOUNT,
        stakeAccountData(2, { deactivationEpoch: epoch, stake: 1n }),
        900n,
      )?.state;

    expect(deactivatedAt(900n)).toBe("deactivating");
    expect(deactivatedAt(899n)).toBe("cooldown");
  });

  it("reports an initialized account as undelegated", () => {
    expect(decodeWalletStakeAccount(ACCOUNT, stakeAccountData(1), 900n)).toMatchObject({
      activeStakeLamports: 0n,
      state: "initialized",
      voter: null,
      withdrawer: WITHDRAWER,
    });
  });

  it("skips accounts with no authorities", () => {
    // The 200-byte size filter also returns uninitialized and rewards-pool accounts.
    expect(decodeWalletStakeAccount(ACCOUNT, stakeAccountData(0), 900n)).toBeNull();
    expect(decodeWalletStakeAccount(ACCOUNT, stakeAccountData(3), 900n)).toBeNull();
  });
});
