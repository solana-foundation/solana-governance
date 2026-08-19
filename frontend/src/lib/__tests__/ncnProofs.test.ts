import {
  assertOverrideProofLineage,
  getStakeAccountProof,
  getVoteAccountProof,
  requireProposalSnapshotSlot,
} from "../ncnProofs";

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
      json: async () => ({}),
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
            active_stake: 10,
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
            active_stake: 10,
            stake_merkle_root: "root",
            vote_account: "validator-vote-b",
            voting_wallet: "validator-identity-wallet",
          },
        },
      ),
    ).toThrow("different snapshots");
  });
});
