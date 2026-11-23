import {
  decodeStakeStateAccount,
  STAKE_PROGRAM_ADDRESS,
  type StakeStateAccount,
} from "@solana-program/stake";
import {
  parseBase64RpcAccount,
  type Account,
  type Address,
  type Base58EncodedBytes,
  type GetEpochInfoApi,
  type GetProgramAccountsApi,
  type Rpc,
} from "@solana/kit";

const STAKE_ACCOUNT_SIZE = 200n;
const AUTHORIZED_STAKER_OFFSET = 12n;
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

async function fetchStakeAccountsByFilter(
  rpc: Rpc<GetProgramAccountsApi>,
  offset: bigint,
  address: Address,
): Promise<Account<StakeStateAccount>[]> {
  const accounts = await rpc
    .getProgramAccounts(STAKE_PROGRAM_ADDRESS, {
      encoding: "base64",
      filters: [
        { dataSize: STAKE_ACCOUNT_SIZE },
        addressMemcmp(offset, address),
      ],
    })
    .send();

  return accounts.flatMap(({ account, pubkey }) => {
    try {
      return [decodeStakeStateAccount(parseBase64RpcAccount(pubkey, account))];
    } catch {
      // The size filter also admits non-decodable legacy data. Ignore it just as
      // getParsedProgramAccounts did by only returning parsed stake accounts.
      return [];
    }
  });
}

function toWalletStakeAccount(
  account: Account<StakeStateAccount>,
  currentEpoch: bigint,
): WalletStakeAccount | null {
  const { state } = account.data;
  if (state.__kind !== "Initialized" && state.__kind !== "Stake") return null;

  const meta = state.fields[0];
  const delegation = state.__kind === "Stake" ? state.fields[1].delegation : null;

  return {
    activeStakeLamports: delegation?.stake ?? 0n,
    address: account.address,
    staker: meta.authorized.staker,
    state:
      state.__kind === "Stake"
        ? getStakeAccountStatus(
            delegation?.stake ?? 0n,
            delegation?.deactivationEpoch ?? PERMANENT_DELEGATION_EPOCH,
            currentEpoch,
          )
        : "initialized",
    voter: delegation?.voterPubkey ?? null,
    withdrawer: meta.authorized.withdrawer,
  };
}

export async function fetchStakeAccounts(
  rpc: Rpc<GetEpochInfoApi & GetProgramAccountsApi>,
  owner: Address,
): Promise<WalletStakeAccount[]> {
  const [epochInfo, byStaker, byWithdrawer] = await Promise.all([
    rpc.getEpochInfo().send(),
    fetchStakeAccountsByFilter(rpc, AUTHORIZED_STAKER_OFFSET, owner),
    fetchStakeAccountsByFilter(rpc, AUTHORIZED_WITHDRAWER_OFFSET, owner),
  ]);

  const byAddress = new Map<string, WalletStakeAccount>();
  for (const account of [...byStaker, ...byWithdrawer]) {
    const mapped = toWalletStakeAccount(account, epochInfo.epoch);
    if (mapped) byAddress.set(mapped.address, mapped);
  }
  return [...byAddress.values()];
}
