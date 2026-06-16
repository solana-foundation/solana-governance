import type { ChainVoteAccountData } from "@/chain";
import {
  determineWalletRole,
  getVoteModalNames,
  hasOnChainValidatorIdentity,
  WalletRole,
} from "../role-detection";

const chainVoteAccount: ChainVoteAccountData = {
  activeStake: 1_000,
  nodePubkey: "validator-wallet",
  voteAccount: "validator-vote-account",
};

describe("determineWalletRole", () => {
  it("keeps on-chain validator identity authoritative when verifier vote accounts are suppressed", () => {
    const role = determineWalletRole(
      [{ stakeAccount: "stake-account" }],
      [],
      chainVoteAccount
    );

    expect(role).toBe(WalletRole.BOTH);
  });

  it("detects validator-only wallets from on-chain identity", () => {
    const role = determineWalletRole(undefined, [], chainVoteAccount);

    expect(role).toBe(WalletRole.VALIDATOR);
  });

  it("does not treat verifier-only vote account summaries as validator identity", () => {
    const role = determineWalletRole(
      [{ stakeAccount: "stake-account" }],
      [{ voteAccount: "verifier-vote-account" }],
      null
    );

    expect(role).toBe(WalletRole.STAKER);
  });
});

describe("vote path modal selection", () => {
  it("routes on-chain validators through validator vote modals", () => {
    expect(getVoteModalNames(chainVoteAccount)).toEqual({
      castModalName: "cast-vote",
      modifyModalName: "modify-vote",
    });
  });

  it("routes wallets without validator identity through stake override modals", () => {
    expect(getVoteModalNames(null)).toEqual({
      castModalName: "override-vote",
      modifyModalName: "modify-override-vote",
    });
  });

  it("reports validator identity only from positive chain data", () => {
    expect(hasOnChainValidatorIdentity(chainVoteAccount)).toBe(true);
    expect(hasOnChainValidatorIdentity(null)).toBe(false);
    expect(hasOnChainValidatorIdentity(undefined)).toBe(false);
  });
});
