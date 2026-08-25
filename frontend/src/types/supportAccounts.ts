import type { Address } from "@solana/kit";

export interface SupportAccountData {
  publicKey: Address;
  proposal: Address;
  validator: Address;
  bump: number;
}
