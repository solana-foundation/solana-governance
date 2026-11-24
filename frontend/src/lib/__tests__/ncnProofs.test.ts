import {
  assertOverrideProofLineage,
  deserializeNetworkMeta,
  getStakeAccountProof,
  getVoteAccountProof,
  requireProposalSnapshotSlot,
} from "../ncnProofs";
import { isNcnProofNotFound } from "../ncnApi";

describe("proposal snapshot proof invariants", () => {
  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
  });

  it("uses the proposal's committed snapshot slot in both proof requests", async () => {
    const slot = 123_456_789n;
    const fetchMock = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () => "{}",
    } as Response);
    global.fetch = fetchMock as unknown as typeof fetch;

    await getVoteAccountProof("validator-vote", "mainnet", slot);
    await getStakeAccountProof("delegator-stake", "mainnet", slot);

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      expect.stringContaining("/proof/vote_account/validator-vote?network=mainnet&slot=123456789"),
      expect.any(Object),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      expect.stringContaining("/proof/stake_account/delegator-stake?network=mainnet&slot=123456789"),
      expect.any(Object),
    );
  });

  it("rejects a proposal that has not committed a snapshot", () => {
    expect(() => requireProposalSnapshotSlot(0n)).toThrow("no snapshot slot");
    expect(requireProposalSnapshotSlot(987n)).toBe(987n);
  });

  it("accepts different delegator and validator wallets when the vote account matches", () => {
    expect(() =>
      assertOverrideProofLineage(
        { vote_account: "validator-vote" },
        {
          meta_merkle_leaf: {
            active_stake: "10",
            stake_merkle_root: "root",
            vote_account: "validator-vote",
            voting_wallet: "validator-identity-wallet",
          },
        },
      ),
    ).not.toThrow();
  });

  it("rejects proofs for different validator vote accounts", () => {
    expect(() =>
      assertOverrideProofLineage(
        { vote_account: "validator-vote-a" },
        {
          meta_merkle_leaf: {
            active_stake: "10",
            stake_merkle_root: "root",
            vote_account: "validator-vote-b",
            voting_wallet: "validator-identity-wallet",
          },
        },
      ),
    ).toThrow("different snapshots");
  });
});

describe("proof responses", () => {
  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
  });

  it("keeps a leaf's active stake exact above the safe integer range", async () => {
    // Regression: 12345678901234567 parsed as a double rounds to ...568, and
    // the program rejects a leaf whose hash no longer matches the snapshot.
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () =>
        '{"meta_merkle_leaf":{"active_stake":12345678901234567},"meta_merkle_proof":[]}',
    } as Response) as unknown as typeof fetch;

    const proof = await getVoteAccountProof("validator-vote", "mainnet", 1n);

    expect(proof.meta_merkle_leaf.active_stake).toBe("12345678901234567");
  });

  it.each([
    ["vote account", () => getVoteAccountProof("validator-vote", "mainnet", 1n)],
    ["stake account", () => getStakeAccountProof("delegator-stake", "mainnet", 1n)],
  ])("reports a %s missing from the snapshot as a proof miss", async (_name, request) => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 404,
      statusText: "Not Found",
      text: async () => "",
      url: "https://ncn.example/proof",
    } as Response) as unknown as typeof fetch;

    const error = await request().catch((caught: unknown) => caught);

    expect(isNcnProofNotFound(error)).toBe(true);
  });
});

describe("deserializeNetworkMeta", () => {
  it("reads the numeric slot and stake a verifier sends", () => {
    expect(
      deserializeNetworkMeta({
        network: "mainnet",
        slot: 440_208_000,
        merkle_root: "root",
        snapshot_hash: "hash",
        created_at: "2026-08-25T00:00:00Z",
        total_active_stake: 1_605_407_797_433_492,
      }),
    ).toMatchObject({
      slot: 440_208_000n,
      total_active_stake: 1_605_407_797_433_492n,
    });
  });

  it("preserves u64 slot and stake values as bigint", () => {
    expect(
      deserializeNetworkMeta({
        network: "mainnet",
        slot: "18446744073709551615",
        merkle_root: "root",
        snapshot_hash: "hash",
        created_at: "2026-08-25T00:00:00Z",
        total_active_stake: "16054077974334921",
      }),
    ).toMatchObject({
      slot: 18_446_744_073_709_551_615n,
      total_active_stake: 16_054_077_974_334_921n,
    });
  });
});
