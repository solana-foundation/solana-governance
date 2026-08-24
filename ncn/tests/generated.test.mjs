import assert from "node:assert/strict";
import test from "node:test";

import { findMetaMerkleProofPda } from "../generated/clients/ts/pdas/metaMerkleProof.ts";

test("derives MetaMerkleProof using the flattened vote-account seed", async () => {
  const [metaMerkleProof] = await findMetaMerkleProofPda({
    consensusResult: "11111111111111111111111111111111",
    voteAccount: "SysvarC1ock11111111111111111111111111111111",
  });

  assert.equal(
    metaMerkleProof,
    "DnsVqtooAMhTrcpKPpCGzJtWYAaiihz4mVnwsCTgakbM",
  );
});
