// export type VoteOutcome = "for" | "against" | "abstain";

import { accentColors, TopVoterRecord } from "@/types";

const getColorFromString = (str: string): string => {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash = hash & hash;
  }
  const index = Math.abs(hash) % accentColors.length;
  return accentColors[index];
};

export const topVoters: TopVoterRecord[] = [
  {
    id: "shinobi",
    validatorName: "Shinobi",
    validatorIdentity: "GU9GEdjq1YHDWFmb44fGSBen6iTbRuPYg8hBsrTA7J8c",
    stakedLamports: 1_200_000n,
    // voteOutcome: "for",
    votePercentage: 6.81,
    voteTimestamp: "2023-11-15T14:23:00Z",
    voteData: {
      forVotesBp: 6000n,
      againstVotesBp: 2000n,
      abstainVotesBp: 2000n,
    },
    accentColor: getColorFromString("shinobi"),
    walletType: "validator",
  },
  {
    id: "stakin",
    validatorName: "Stakin",
    validatorIdentity: "JKvMiwT4qbQVXnvezFtyLiG5QqY4TPFRJPE6dm8idiN8",
    stakedLamports: 987_000n,
    // voteOutcome: "for",
    votePercentage: 5.23,
    voteTimestamp: "2023-11-14T09:10:00Z",
    voteData: {
      forVotesBp: 6000n,
      againstVotesBp: 2000n,
      abstainVotesBp: 2000n,
    },
    accentColor: getColorFromString("stakin"),
    walletType: "validator",
  },
  {
    id: "everstake",
    validatorName: "Everstake",
    validatorIdentity: "QbhGZV8cYJJBgSUE9us4HwLAiWw9TG8R6mcMjmYYB2kA",
    stakedLamports: 765_000n,
    // voteOutcome: "against",
    votePercentage: 4.05,
    voteTimestamp: "2023-11-13T18:45:00Z",
    voteData: {
      forVotesBp: 2000n,
      againstVotesBp: 6000n,
      abstainVotesBp: 2000n,
    },
    accentColor: getColorFromString("everstake"),
    walletType: "validator",
  },
  {
    id: "figment",
    validatorName: "Figment",
    validatorIdentity: "92jGF5FXGTo8LgHSAj4oWX8iNM83fF7pKTojk1BP9LpK",
    stakedLamports: 543_000n,
    // voteOutcome: "abstain",
    votePercentage: 2.88,
    voteTimestamp: "2023-11-12T22:15:00Z",
    voteData: {
      forVotesBp: 1000n,
      againstVotesBp: 1000n,
      abstainVotesBp: 8000n,
    },
    accentColor: getColorFromString("figment"),
    walletType: "validator",
  },
  {
    id: "chorus-one",
    validatorName: "Chorus One",
    validatorIdentity: "TXQ1frfDyNj6nBWDe2be2ZY7GfQDRHWBjbMebpM1VWXq",
    stakedLamports: 1_450_000n,
    // voteOutcome: "for",
    votePercentage: 7.96,
    voteTimestamp: "2023-11-15T03:40:00Z",
    voteData: {
      forVotesBp: 9000n,
      againstVotesBp: 500n,
      abstainVotesBp: 500n,
    },
    accentColor: getColorFromString("chorus-one"),
    walletType: "validator",
  },
  {
    id: "p2p",
    validatorName: "P2P.org",
    validatorIdentity: "qyJYnnGhXmLXwYPkFk6qdXGW3Dxx2AzqGWUJfMi1pmoQ",
    stakedLamports: 1_015_000n,
    // voteOutcome: "for",
    votePercentage: 6.12,
    voteTimestamp: "2023-11-14T21:05:00Z",
    voteData: {
      forVotesBp: 10000n,
      againstVotesBp: 0n,
      abstainVotesBp: 0n,
    },
    accentColor: getColorFromString("p2p"),
    walletType: "validator",
  },
  {
    id: "rockawayx",
    validatorName: "RockawayX",
    validatorIdentity: "WaNaomugZdRUNib91sGF2ujij47GoeLwdCvbZBXgyUqB",
    stakedLamports: 812_000n,
    // voteOutcome: "against",
    votePercentage: 4.61,
    voteTimestamp: "2023-11-13T11:30:00Z",
    voteData: {
      forVotesBp: 1000n,
      againstVotesBp: 8000n,
      abstainVotesBp: 1000n,
    },
    accentColor: getColorFromString("rockawayx"),
    walletType: "validator",
  },
  {
    id: "coinbase",
    validatorName: "Coinbase Cloud",
    validatorIdentity: "2Y4PfXNQm6Kdj8Ggr8QXkXNRX7ytzEhdRya6fF33dVSs",
    stakedLamports: 1_110_000n,
    // voteOutcome: "for",
    votePercentage: 6.71,
    voteTimestamp: "2023-11-15T05:50:00Z",
    voteData: {
      forVotesBp: 8000n,
      againstVotesBp: 1000n,
      abstainVotesBp: 1000n,
    },
    accentColor: getColorFromString("coinbase"),
    walletType: "validator",
  },
  {
    id: "blockdaemon",
    validatorName: "Blockdaemon",
    validatorIdentity: "DpNRt3EphMTNaV5mqXVnWgY9tPWWsNjHGfQWU2CdGr1U",
    stakedLamports: 698_000n,
    // voteOutcome: "abstain",
    votePercentage: 3.28,
    voteTimestamp: "2023-11-12T17:05:00Z",
    voteData: {
      forVotesBp: 8000n,
      againstVotesBp: 1000n,
      abstainVotesBp: 1000n,
    },
    accentColor: getColorFromString("blockdaemon"),
    walletType: "validator",
  },
  {
    id: "solana-foundation",
    validatorName: "Solana Foundation",
    validatorIdentity: "cfuneLZZZH1Vw4MQvKnRkcd4QdichNcNCf4Chui4e55q",
    stakedLamports: 1_980_000n,
    // voteOutcome: "for",
    votePercentage: 10.45,
    voteTimestamp: "2023-11-15T01:25:00Z",
    voteData: {
      forVotesBp: 8000n,
      againstVotesBp: 1000n,
      abstainVotesBp: 1000n,
    },
    accentColor: getColorFromString("solana-foundation"),
    walletType: "validator",
  },
  {
    id: "kiln",
    validatorName: "Kiln",
    validatorIdentity: "DauLfwEpetmr92P4H3RPaKEo5hjdzfDhLdcebxK6P9ez",
    stakedLamports: 605_000n,
    // voteOutcome: "for",
    votePercentage: 3.96,
    voteTimestamp: "2023-11-14T16:40:00Z",
    voteData: {
      forVotesBp: 8000n,
      againstVotesBp: 1000n,
      abstainVotesBp: 1000n,
    },
    accentColor: getColorFromString("kiln"),
    walletType: "validator",
  },
  {
    id: "stakefish",
    validatorName: "stakefish",
    validatorIdentity: "uRjXvFeibDqzxDGbXhJX17uKkbvDnnxCpbn65Cey2G8c",
    stakedLamports: 889_000n,
    // voteOutcome: "against",
    votePercentage: 5.02,
    voteTimestamp: "2023-11-13T08:20:00Z",
    voteData: {
      forVotesBp: 10000n,
      againstVotesBp: 8000n,
      abstainVotesBp: 1000n,
    },
    accentColor: getColorFromString("stakefish"),
    walletType: "validator",
  },
];
