import type { LegacyPublicKey as PublicKey } from "@/lib/governance/legacyAdapters";

export const shortenPublicKey = (pk: PublicKey | string) => {
  const stringPk = typeof pk === "string" ? pk : pk.toString();

  return `${stringPk.substring(0, 6)}...${stringPk.substring(
    stringPk.length - 6,
    stringPk.length
  )}`;
};
export const shortenMediumPublickKey = (pk: PublicKey | string) => {
  const stringPk = typeof pk === "string" ? pk : pk.toString();

  return `${stringPk.substring(0, 10)}...${stringPk.substring(
    stringPk.length - 10,
    stringPk.length
  )}`;
};
