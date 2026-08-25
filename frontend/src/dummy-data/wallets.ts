import { RawStakeAccountData } from "@/types/stakeAccounts";

export interface ProposalStats {
  total: number;
  active: number;
  history: number;
}

export interface WalletData {
  snapshot_slot: number;
  voting_wallet: string;
  vote_accounts: unknown[];
  stake_accounts: RawStakeAccountData[];
  proposalStats: ProposalStats;
}

// Dummy Proposal Stats
export const dummyProposalStats: ProposalStats = {
  total: 12,
  active: 3,
  history: 9,
};

// Validator Only Wallet
// export const validatorOnlyWallet: WalletData = {
//   snapshot_slot: 245789456,
//   voting_wallet: "CG4tRANBKDoM7dN9LssTdVkFhDykeq7A8CZurA5AQSFJH",
//   vote_accounts: [
//     {
//       vote_account: "4fjd8k7so9jK5kFGMwBXdudvWHYwtNgNhvLu",
//       active_stake: 1020450000000000,
//       identity: "CG4tRANBKDoM7dN9LssTdVkFhDykeq7A8CZurA5AQSFJH",
//       commission: 5,
//       lastVote: 245789455,
//       credits: 950000,
//       epochCredits: 8500,
//       activated_stake: 1020450000000000,
//     },
//     {
//       vote_account: "8hgr2l2pd5kFGMwBXdudvWHYwtNgNhvLuCertusDeBmq",
//       active_stake: 850300000000000,
//       identity: "CG4tRANBKDoM7dN9LssTdVkFhDykeq7A8CZurA5AQSFJH",
//       commission: 5,
//       lastVote: 245789456,
//       credits: 890000,
//       epochCredits: 8200,
//       activated_stake: 850300000000000,
//     },
//     {
//       vote_account: "b2k1m9as5kFGMwBXdudvWHYwtNgNhvLuCertusDeTest",
//       active_stake: 560000000000000,
//       identity: "CG4tRANBKDoM7dN9LssTdVkFhDykeq7A8CZurA5AQSFJH",
//       commission: 5,
//       lastVote: 245789454,
//       credits: 560000,
//       epochCredits: 5600,
//       activated_stake: 560000000000000,
//     },
//     {
//       vote_account: "c3lmp8qw7kFGMwBXdudvWHYwtNgNhvLuValidatorX",
//       active_stake: 100273000000000,
//       identity: "CG4tRANBKDoM7dN9LssTdVkFhDykeq7A8CZurA5AQSFJH",
//       commission: 5,
//       lastVote: 245789456,
//       credits: 100000,
//       epochCredits: 1000,
//       activated_stake: 100273000000000,
//     },
//     {
//       vote_account: "d4mnq9rx5kFGMwBXdudvWHYwtNgNhvLuSmallVal",
//       active_stake: 17100000000000,
//       identity: "CG4tRANBKDoM7dN9LssTdVkFhDykeq7A8CZurA5AQSFJH",
//       commission: 5,
//       lastVote: 245789450,
//       credits: 17000,
//       epochCredits: 170,
//       activated_stake: 17100000000000,
//     },
//   ],
//   stake_accounts: [],
//   proposalStats: dummyProposalStats,
// };

// Staker Only Wallet
export const stakerOnlyWallet: WalletData = {
  snapshot_slot: 245789456,
  voting_wallet: "3vyuweP6mwGxK27NNPRRPdE7j3BvMdJvP2PeVQ3mXzEd",
  vote_accounts: [],
  stake_accounts: [
    {
      stake_account: "3StELsSKCT4K27Np41oeYqPefeNQEHSv1UDhYrehxin3",
      active_stake: 1500000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorA",
      state: "initialized",
    },
    {
      stake_account: "8ZawdkxK5kFGMwBXdudvWHYwtNgNhvLuCertusDeBmq5",
      active_stake: 2250000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorB",
      state: "initialized",
    },
    {
      stake_account: "9YbwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct3",
      active_stake: 500000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorC",
      state: "inactive",
    },
    {
      stake_account: "AZcwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct4",
      active_stake: 750000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorA",
      state: "deactivating",
    },
    {
      stake_account: "BYdwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct5",
      active_stake: 300000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorD",
      state: "initialized",
    },
    {
      stake_account: "CXewdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct6",
      active_stake: 1800000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorE",
      state: "initialized",
    },
    {
      stake_account: "DZfwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct7",
      active_stake: 950000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorF",
      state: "initialized",
    },
    {
      stake_account: "EAgwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct8",
      active_stake: 420000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorG",
      state: "inactive",
    },
    {
      stake_account: "FBhwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct9",
      active_stake: 1350000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorH",
      state: "deactivating",
    },
    {
      stake_account: "GCiwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct10",
      active_stake: 680000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorI",
      state: "initialized",
    },
    {
      stake_account: "HDjwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct11",
      active_stake: 2100000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorJ",
      state: "initialized",
    },
    {
      stake_account: "IEkwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct12",
      active_stake: 890000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorK",
      state: "initialized",
    },
    {
      stake_account: "JFlwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct13",
      active_stake: 340000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorL",
      state: "inactive",
    },
    {
      stake_account: "KGmwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct14",
      active_stake: 1650000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorM",
      state: "deactivating",
    },
    {
      stake_account: "LHnwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct15",
      active_stake: 720000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorN",
      state: "initialized",
    },
    {
      stake_account: "MIowdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct16",
      active_stake: 1920000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorO",
      state: "initialized",
    },
    {
      stake_account: "NJpwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct17",
      active_stake: 580000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorP",
      state: "initialized",
    },
    {
      stake_account: "OKqwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct18",
      active_stake: 1240000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorQ",
      state: "inactive",
    },
    {
      stake_account: "PLrwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct19",
      active_stake: 460000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorR",
      state: "deactivating",
    },
    {
      stake_account: "QMswdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct20",
      active_stake: 1780000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorS",
      state: "initialized",
    },
    {
      stake_account: "RNtwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct21",
      active_stake: 820000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorT",
      state: "initialized",
    },
    {
      stake_account: "SOuwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct22",
      active_stake: 1450000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorU",
      state: "initialized",
    },
    {
      stake_account: "TPvwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct23",
      active_stake: 630000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorV",
      state: "inactive",
    },
    {
      stake_account: "UQwwdkxK5kFGMwBXdudvWHYwtNgNhvLuStakeAcct24",
      active_stake: 2400000000000000,
      vote_account: "vspdrGthy5kFGMwBXdudvWHYwtNgNhvLuValidatorW",
      state: "initialized",
    },
  ],
  proposalStats: dummyProposalStats,
};

// Both Validator and Staker Wallet
export const validatorAndStakerWallet: WalletData = {
  snapshot_slot: 245789456,
  voting_wallet: "7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2",
  vote_accounts: [
    // {
    //   vote_account: "4fjd8k7so9jK5kFGMwBXdudvWHYwtNgNhvLu",
    //   active_stake: 1020450000000000,
    //   identity: "7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2",
    //   commission: 5,
    //   lastVote: 245789455,
    //   credits: 950000,
    //   epochCredits: 8500,
    //   activated_stake: 1020450000000000,
    // },
    // {
    //   vote_account: "8hgr2l2pd5kFGMwBXdudvWHYwtNgNhvLuCertusDeBmq",
    //   active_stake: 850300000000000,
    //   identity: "7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2",
    //   commission: 5,
    //   lastVote: 245789456,
    //   credits: 890000,
    //   epochCredits: 8200,
    //   activated_stake: 850300000000000,
    // },
  ],
  stake_accounts: [
    {
      stake_account: "3StELsSKCT4K27Np41oeYqPefeNQEHSv1UDhYrehxin3",
      active_stake: 1500000000000000,
      vote_account: "4fjd8k7so9jK5kFGMwBXdudvWHYwtNgNhvLu", // Own validator
      state: "initialized",
    },
    {
      stake_account: "8ZawdkxK5kFGMwBXdudvWHYwtNgNhvLuCertusDeBmq5",
      active_stake: 2250000000000000,
      vote_account: "ExternalValidator5kFGMwBXdudvWHYwtNgNhvLu", // External validator
      state: "inactive",
    },
  ],
  proposalStats: dummyProposalStats,
};

// No Role Wallet (neither validator nor staker)
export const noRoleWallet: WalletData = {
  snapshot_slot: 245789456,
  voting_wallet: "EmptyWallet5kFGMwBXdudvWHYwtNgNhvLuNoRole",
  vote_accounts: [],
  stake_accounts: [],
  proposalStats: dummyProposalStats,
};

// Export all wallet scenarios
export const dummyWallets = {
  // validatorOnly: validatorOnlyWallet,
  validatorOnly: [],
  stakerOnly: stakerOnlyWallet,
  both: validatorAndStakerWallet,
  noRole: noRoleWallet,
};
