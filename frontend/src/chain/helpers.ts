import { SvmgovProgramOLD } from "@/chain";
import { AnchorProvider, Program } from "@coral-xyz/anchor";
import { Connection, Keypair } from "@solana/web3.js";

import idl from "@/chain/idl/svmgov_program.json";

const RPC_URL = "https://api.devnet.solana.com";

export const connection = new Connection(RPC_URL, "confirmed");

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const dummyWallet: any = {
  publicKey: Keypair.generate().publicKey,
  signAllTransactions: async () => {},
  signTransaction: async () => {},
};

const provider = new AnchorProvider(connection, dummyWallet, {
  commitment: "confirmed",
});

/**
 * @deprecated old program
 */
export const program = new Program(idl, provider) as Program<SvmgovProgramOLD>;
import { BN, Program as AnchorProgram } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import type { SvmgovProgram } from "@/chain";

// Corrected version: removed broken imports, replaced anchor.Program and anchor.BN with correct imports
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
