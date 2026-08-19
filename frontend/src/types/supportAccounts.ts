import type { LegacyPublicKey } from "@/lib/governance/legacyAdapters";

export interface SupportAccountData {
  publicKey: LegacyPublicKey;
  proposal: LegacyPublicKey;
  validator: LegacyPublicKey;
  bump: number;
}
