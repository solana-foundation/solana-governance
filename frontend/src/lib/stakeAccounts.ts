import {
  getAddressDecoder,
  getBase64Encoder,
  type Address,
  type Base58EncodedBytes,
  type GetEpochInfoApi,
  type GetProgramAccountsApi,
  type Rpc,
} from "@solana/kit";

const STAKE_PROGRAM_ADDRESS =
  "Stake11111111111111111111111111111111111111" as Address;
const STAKE_ACCOUNT_SIZE = 200n;
const AUTHORIZED_WITHDRAWER_OFFSET = 44n;
const PERMANENT_DELEGATION_EPOCH = 18_446_744_073_709_551_615n;

export type WalletStakeAccount = {
  activeStakeLamports: bigint;
  address: Address;
  staker: Address;
  state: "cooldown" | "deactivating" | "delegated" | "inactive" | "initialized";
  voter: Address | null;
  withdrawer: Address;
};

// Base58 because the RPC proxy only admits stake scans keyed by a base58 public key.
function addressMemcmp(offset: bigint, address: Address) {
  return {
    memcmp: { bytes: address as string as Base58EncodedBytes, encoding: "base58" as const, offset },
  };
}

function getStakeAccountStatus(
  delegatedStakeLamports: bigint,
  deactivationEpoch: bigint,
  currentEpoch: bigint,
): WalletStakeAccount["state"] {
  if (delegatedStakeLamports === 0n) return "inactive";
  if (deactivationEpoch === PERMANENT_DELEGATION_EPOCH) return "delegated";
  return deactivationEpoch >= currentEpoch ? "deactivating" : "cooldown";
}

// StakeStateV2 layout: a u32 variant, then Meta (rent reserve, staker,
// withdrawer, lockup) and, for delegated accounts, the Delegation.
const STATE_INITIALIZED = 1;
const STATE_STAKE = 2;
const STAKER_OFFSET = 12;
const WITHDRAWER_OFFSET = 44;
const VOTER_OFFSET = 124;
const DELEGATED_STAKE_OFFSET = 156;
const DEACTIVATION_EPOCH_OFFSET = 172;

/**
 * Decodes a stake account's raw data, or returns `null` when the account is
 * neither initialized nor delegated (uninitialized and rewards-pool accounts
 * have no authorities).
 */
export function decodeWalletStakeAccount(
  address: Address,
  data: Uint8Array,
  currentEpoch: bigint,
): WalletStakeAccount | null {
  if (data.length < Number(STAKE_ACCOUNT_SIZE)) return null;
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const variant = view.getUint32(0, true);
  if (variant !== STATE_INITIALIZED && variant !== STATE_STAKE) return null;

  const readAddress = (offset: number) =>
    getAddressDecoder().decode(data, offset);
  const authorities = {
    address,
    staker: readAddress(STAKER_OFFSET),
    withdrawer: readAddress(WITHDRAWER_OFFSET),
  };
  if (variant === STATE_INITIALIZED) {
    return { ...authorities, activeStakeLamports: 0n, state: "initialized", voter: null };
  }

  const delegatedStake = view.getBigUint64(DELEGATED_STAKE_OFFSET, true);
  return {
    ...authorities,
    activeStakeLamports: delegatedStake,
    state: getStakeAccountStatus(
      delegatedStake,
      view.getBigUint64(DEACTIVATION_EPOCH_OFFSET, true),
      currentEpoch,
    ),
    voter: readAddress(VOTER_OFFSET),
  };
}

/**
 * Lists the stake accounts whose withdraw authority is `owner`. A wallet that
 * is only the staker is not matched: a snapshot leaf's voting wallet is the
 * withdraw authority, so only the withdrawer can vote with the account.
 */
export async function fetchStakeAccounts(
  rpc: Rpc<GetEpochInfoApi & GetProgramAccountsApi>,
  owner: Address,
): Promise<WalletStakeAccount[]> {
  const [epochInfo, accounts] = await Promise.all([
    rpc.getEpochInfo().send(),
    rpc
      .getProgramAccounts(STAKE_PROGRAM_ADDRESS, {
        encoding: "base64",
        filters: [
          { dataSize: STAKE_ACCOUNT_SIZE },
          addressMemcmp(AUTHORIZED_WITHDRAWER_OFFSET, owner),
        ],
      })
      .send(),
  ]);

  return accounts.flatMap(({ account, pubkey }) => {
    const data = new Uint8Array(getBase64Encoder().encode(account.data[0]));
    const mapped = decodeWalletStakeAccount(pubkey, data, epochInfo.epoch);
    return mapped ? [mapped] : [];
  });
}
