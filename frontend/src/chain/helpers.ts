import { BN, Program as AnchorProgram } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import type { SvmgovProgram } from "@/chain";

export function deriveProposalAccount(
  program: AnchorProgram<SvmgovProgram>,
  seed: BN,
  splVoteAccount: PublicKey,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("proposal"),
      seed.toArrayLike(Buffer, "le", 8),
      splVoteAccount.toBuffer(),
    ],
    program.programId,
  )[0];
}
