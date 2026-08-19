import assert from "node:assert/strict";
import test from "node:test";

import { findGlobalConfigPda } from "../clients/ts/pdas/globalConfig.ts";
import { findProposalPda } from "../clients/ts/pdas/proposal.ts";
import {
  getVoteOverrideCacheDecoder,
  getVoteOverrideCacheEncoder,
  getVoteOverrideCacheSize,
} from "../clients/ts/accounts/voteOverrideCache.ts";
import { getStakeMerkleLeafEncoder } from "../clients/ts/types/stakeMerkleLeaf.ts";

test("derives the governance PDAs from the deployed program and canonical seeds", async () => {
  const [globalConfig] = await findGlobalConfigPda();
  const [proposal] = await findProposalPda({
    seed: 42n,
    splVoteAccount: "11111111111111111111111111111111",
  });

  assert.equal(globalConfig, "4V8PnDoVa7BSUWj16uhUkgHhXPoBEvMSk4EFhKA9wyFA");
  assert.equal(proposal, "G3UaSmcVuDXweprGDbfi4APQsvom9jjuZuZ3cbS2gNPb");
});

test("serializes a stake merkle leaf with fixed account and u64 layouts", () => {
  const bytes = getStakeMerkleLeafEncoder().encode({
    votingWallet: "11111111111111111111111111111111",
    stakeAccount: "SysvarC1ock11111111111111111111111111111111",
    activeStake: 42n,
  });

  assert.equal(
    Buffer.from(bytes).toString("hex"),
    "000000000000000000000000000000000000000000000000000000000000000006a7d51718c774c928566398691d5eb68b5eb8a39b4b6d5c73555b21000000002a00000000000000",
  );
});

test("serializes the current main VoteOverrideCache layout", () => {
  const cache = {
    validator: "11111111111111111111111111111111",
    proposal: "11111111111111111111111111111111",
    voteAccountValidator: "11111111111111111111111111111111",
    forVotesLamports: 1n,
    againstVotesLamports: 2n,
    abstainVotesLamports: 3n,
    totalStake: 4n,
    bump: 255,
  };
  const bytes = getVoteOverrideCacheEncoder().encode(cache);

  assert.equal(getVoteOverrideCacheSize(), 137);
  assert.equal(bytes.length, 137);
  assert.deepEqual(getVoteOverrideCacheDecoder().decode(bytes), {
    discriminator: new Uint8Array([195, 82, 50, 219, 140, 34, 108, 57]),
    ...cache,
  });
});
