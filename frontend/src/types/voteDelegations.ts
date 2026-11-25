import type { Address } from "@solana/kit";

/**
 * On-chain `VoteDelegation`. The subject is an SPL vote account (a validator's
 * vote) or a voting wallet (a staker's override vote).
 */
export interface VoteDelegationData {
  subject: Address;
  delegate: Address;
  delegationAccepted: boolean;
}
