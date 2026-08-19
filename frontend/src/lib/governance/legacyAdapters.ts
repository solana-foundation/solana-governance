import BN from "bn.js";
import { PublicKey } from "@solana/web3.js";

export const toLegacyPublicKey = (address: string) => new PublicKey(address);
export const toLegacyBn = (value: bigint) => new BN(value.toString());
