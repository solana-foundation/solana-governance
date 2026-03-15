/**
 * @deprecated old program types
 */
export type SvmgovProgramOLD = {
  address: "GoVpHPV3EY89hwKJjfw19jTdgMsGKG4UFSE2SfJqTuhc";
  metadata: {
    name: "svmgov_program";
    version: "0.1.0";
    spec: "0.1.0";
    description: "Created with Anchor";
  };
  instructions: [
    {
      name: "castVote";
      discriminator: [20, 212, 15, 189, 69, 180, 69, 151];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "splVoteAccount";
        },
        {
          name: "proposal";
          writable: true;
        },
        {
          name: "vote";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [118, 111, 116, 101];
              },
              {
                kind: "account";
                path: "proposal";
              },
              {
                kind: "account";
                path: "signer";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "forVotesBp";
          type: "u64";
        },
        {
          name: "againstVotesBp";
          type: "u64";
        },
        {
          name: "abstainVotesBp";
          type: "u64";
        }
      ];
    },
    {
      name: "createProposal";
      discriminator: [132, 116, 68, 174, 216, 160, 198, 22];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "splVoteAccount";
        },
        {
          name: "proposal";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [112, 114, 111, 112, 111, 115, 97, 108];
              },
              {
                kind: "arg";
                path: "seed";
              },
              {
                kind: "account";
                path: "signer";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "seed";
          type: "u64";
        },
        {
          name: "title";
          type: "string";
        },
        {
          name: "description";
          type: "string";
        },
        {
          name: "startEpoch";
          type: "u64";
        },
        {
          name: "votingLengthEpochs";
          type: "u64";
        }
      ];
    },
    {
      name: "modifyVote";
      discriminator: [116, 52, 102, 0, 121, 145, 27, 139];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "splVoteAccount";
        },
        {
          name: "proposal";
          writable: true;
        },
        {
          name: "vote";
          pda: {
            seeds: [
              {
                kind: "const";
                value: [118, 111, 116, 101];
              },
              {
                kind: "account";
                path: "proposal";
              },
              {
                kind: "account";
                path: "signer";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "forVotesBp";
          type: "u64";
        },
        {
          name: "againstVotesBp";
          type: "u64";
        },
        {
          name: "abstainVotesBp";
          type: "u64";
        }
      ];
    },
    {
      name: "supportProposal";
      discriminator: [95, 239, 233, 199, 201, 62, 90, 27];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "splVoteAccount";
        },
        {
          name: "proposal";
          writable: true;
        },
        {
          name: "support";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [115, 117, 112, 112, 111, 114, 116];
              },
              {
                kind: "account";
                path: "proposal";
              },
              {
                kind: "account";
                path: "signer";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [];
    },
    {
      name: "tallyVotes";
      discriminator: [144, 82, 0, 72, 160, 132, 35, 121];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "splVoteAccount";
        },
        {
          name: "proposal";
          writable: true;
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "finalize";
          type: "bool";
        }
      ];
    }
  ];
  accounts: [
    {
      name: "proposal";
      discriminator: [26, 94, 189, 187, 116, 136, 53, 33];
    },
    {
      name: "support";
      discriminator: [247, 108, 3, 111, 84, 51, 217, 107];
    },
    {
      name: "vote";
      discriminator: [96, 91, 104, 57, 145, 35, 172, 155];
    }
  ];
  errors: [
    {
      code: 6000;
      name: "notEnoughStake";
      msg: "Minimum stake required to create proposal is 40k";
    },
    {
      code: 6001;
      name: "titleTooLong";
      msg: "The title of the proposal is too long, max 50 char";
    },
    {
      code: 6002;
      name: "descriptionTooLong";
      msg: "The description of the proposal is too long, max 250 char";
    },
    {
      code: 6003;
      name: "descriptionInvalid";
      msg: "The description of the proposal must point to a github link";
    },
    {
      code: 6004;
      name: "invalidProposalId";
      msg: "Invalid proposal ID";
    },
    {
      code: 6005;
      name: "votingNotStarted";
      msg: "Voting on proposal not yet started";
    },
    {
      code: 6006;
      name: "proposalClosed";
      msg: "Proposal closed";
    },
    {
      code: 6007;
      name: "proposalFinalized";
      msg: "Proposal finalized";
    },
    {
      code: 6008;
      name: "invalidVoteDistribution";
      msg: "Vote distribution must add up to 100% in Basis Points";
    },
    {
      code: 6009;
      name: "votingPeriodNotEnded";
      msg: "Voting period not yet ended";
    },
    {
      code: 6010;
      name: "invalidVoteAccount";
      msg: "Invalid vote account, proposal id mismatch";
    },
    {
      code: 6011;
      name: "failedDeserializeNodePubkey";
      msg: "Failed to deserialize node_pubkey from Vote account";
    },
    {
      code: 6012;
      name: "voteNodePubkeyMismatch";
      msg: "Deserialized node_pubkey from Vote accounts does not match";
    },
    {
      code: 6013;
      name: "notEnoughAccounts";
      msg: "Not enough accounts for tally";
    }
  ];
  types: [
    {
      name: "proposal";
      type: {
        kind: "struct";
        fields: [
          {
            name: "author";
            type: "pubkey";
          },
          {
            name: "title";
            type: "string";
          },
          {
            name: "description";
            type: "string";
          },
          {
            name: "creationEpoch";
            type: "u64";
          },
          {
            name: "startEpoch";
            type: "u64";
          },
          {
            name: "endEpoch";
            type: "u64";
          },
          {
            name: "proposerStakeWeightBp";
            type: "u64";
          },
          {
            name: "clusterSupportBp";
            type: "u64";
          },
          {
            name: "forVotesBp";
            type: "u64";
          },
          {
            name: "againstVotesBp";
            type: "u64";
          },
          {
            name: "abstainVotesBp";
            type: "u64";
          },
          {
            name: "voting";
            type: "bool";
          },
          {
            name: "finalized";
            type: "bool";
          },
          {
            name: "proposalBump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "support";
      type: {
        kind: "struct";
        fields: [
          {
            name: "proposal";
            type: "pubkey";
          },
          {
            name: "validator";
            type: "pubkey";
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "vote";
      type: {
        kind: "struct";
        fields: [
          {
            name: "validator";
            type: "pubkey";
          },
          {
            name: "proposal";
            type: "pubkey";
          },
          {
            name: "forVotesBp";
            type: "u64";
          },
          {
            name: "againstVotesBp";
            type: "u64";
          },
          {
            name: "abstainVotesBp";
            type: "u64";
          },
          {
            name: "voteTimestamp";
            type: "i64";
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    }
  ];
};
