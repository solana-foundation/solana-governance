import {
  parseInitMetaMerkleProofInstruction,
  NCN_SNAPSHOT_PROGRAM_ADDRESS,
} from "@solana/ncn-snapshot";
import { address, type TransactionModifyingSigner } from "@solana/kit";
import { webcrypto } from "node:crypto";
import { buildInitializeMetaMerkleProofInstruction } from "../transactions";

const SIGNER = {
  address: address("11111111111111111111111111111111"),
} as unknown as TransactionModifyingSigner;

describe("MetaMerkleProof initialization", () => {
  beforeAll(() => {
    Object.defineProperty(globalThis, "crypto", {
      configurable: true,
      value: webcrypto,
    });
    Object.defineProperty(globalThis, "isSecureContext", {
      configurable: true,
      value: true,
    });
  });

  it("uses the generated NCN instruction and preserves every proof field", async () => {
    const instruction = await buildInitializeMetaMerkleProofInstruction({
      closeTimestamp: 1234n,
      consensusResult: address("SysvarC1ock11111111111111111111111111111111"),
      proof: {
        meta_merkle_leaf: {
          active_stake: 42,
          stake_merkle_root: "SysvarRent111111111111111111111111111111111",
          vote_account: "SysvarC1ock11111111111111111111111111111111",
          voting_wallet: "11111111111111111111111111111111",
        },
        meta_merkle_proof: ["SysvarRent111111111111111111111111111111111"],
      },
      signer: SIGNER,
    });
    const parsed = parseInitMetaMerkleProofInstruction(instruction);

    expect(instruction.programAddress).toBe(NCN_SNAPSHOT_PROGRAM_ADDRESS);
    expect(parsed.data.closeTimestamp).toBe(1234n);
    expect(parsed.data.metaMerkleLeaf.activeStake).toBe(42n);
    expect(parsed.data.metaMerkleLeaf.voteAccount).toBe(
      "SysvarC1ock11111111111111111111111111111111",
    );
    expect(parsed.data.metaMerkleProof).toHaveLength(1);
  });
});
