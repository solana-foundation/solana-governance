/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/svmgov_program.json`.
 */
export type SvmgovProgram = {
  address: '94rLyg6fBA231a9UUoKrVH3nuXqNY7nnqyMkDrd55Ydu';
  metadata: {
    name: 'svmgov_program';
    version: '0.1.0';
    spec: '0.1.0';
    description: 'Created with Anchor';
  };
  instructions: [
    {
      name: 'adjustProposalTiming';
      discriminator: [82, 66, 219, 217, 123, 150, 223, 224];
      accounts: [
        {
          name: 'signer';
          writable: true;
          signer: true;
        },
        {
          name: 'proposal';
          writable: true;
        }
      ];
      args: [
        {
          name: 'creationTimestamp';
          type: {
            option: 'i64';
          };
        },
        {
          name: 'creationEpoch';
          type: {
            option: 'u64';
          };
        },
        {
          name: 'startEpoch';
          type: {
            option: 'u64';
          };
        },
        {
          name: 'endEpoch';
          type: {
            option: 'u64';
          };
        },
        {
          name: 'snapshotSlot';
          type: {
            option: 'u64';
          };
        }
      ];
    },
    {
      name: 'castVote';
      discriminator: [20, 212, 15, 189, 69, 180, 69, 151];
      accounts: [
        {
          name: 'signer';
          writable: true;
          signer: true;
        },
        {
          name: 'proposal';
          writable: true;
        },
        {
          name: 'vote';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [118, 111, 116, 101];
              },
              {
                kind: 'account';
                path: 'proposal';
              },
              {
                kind: 'account';
                path: 'splVoteAccount';
              }
            ];
          };
        },
        {
          name: 'splVoteAccount';
        },
        {
          name: 'voteOverrideCache';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [
                  118,
                  111,
                  116,
                  101,
                  95,
                  111,
                  118,
                  101,
                  114,
                  114,
                  105,
                  100,
                  101,
                  95,
                  99,
                  97,
                  99,
                  104,
                  101
                ];
              },
              {
                kind: 'account';
                path: 'proposal';
              },
              {
                kind: 'account';
                path: 'vote';
              }
            ];
          };
        },
        {
          name: 'snapshotProgram';
        },
        {
          name: 'consensusResult';
        },
        {
          name: 'metaMerkleProof';
        },
        {
          name: 'systemProgram';
          address: '11111111111111111111111111111111';
        }
      ];
      args: [
        {
          name: 'forVotesBp';
          type: 'u64';
        },
        {
          name: 'againstVotesBp';
          type: 'u64';
        },
        {
          name: 'abstainVotesBp';
          type: 'u64';
        }
      ];
    },
    {
      name: 'castVoteOverride';
      discriminator: [225, 8, 137, 98, 214, 156, 183, 62];
      accounts: [
        {
          name: 'signer';
          writable: true;
          signer: true;
        },
        {
          name: 'proposal';
          writable: true;
        },
        {
          name: 'validatorVote';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [118, 111, 116, 101];
              },
              {
                kind: 'account';
                path: 'proposal';
              },
              {
                kind: 'account';
                path: 'splVoteAccount';
              }
            ];
          };
        },
        {
          name: 'splVoteAccount';
        },
        {
          name: 'voteOverride';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [
                  118,
                  111,
                  116,
                  101,
                  95,
                  111,
                  118,
                  101,
                  114,
                  114,
                  105,
                  100,
                  101
                ];
              },
              {
                kind: 'account';
                path: 'proposal';
              },
              {
                kind: 'account';
                path: 'splStakeAccount';
              },
              {
                kind: 'account';
                path: 'validatorVote';
              }
            ];
          };
        },
        {
          name: 'voteOverrideCache';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [
                  118,
                  111,
                  116,
                  101,
                  95,
                  111,
                  118,
                  101,
                  114,
                  114,
                  105,
                  100,
                  101,
                  95,
                  99,
                  97,
                  99,
                  104,
                  101
                ];
              },
              {
                kind: 'account';
                path: 'proposal';
              },
              {
                kind: 'account';
                path: 'validatorVote';
              }
            ];
          };
        },
        {
          name: 'splStakeAccount';
        },
        {
          name: 'snapshotProgram';
        },
        {
          name: 'consensusResult';
        },
        {
          name: 'metaMerkleProof';
        },
        {
          name: 'systemProgram';
          address: '11111111111111111111111111111111';
        }
      ];
      args: [
        {
          name: 'forVotesBp';
          type: 'u64';
        },
        {
          name: 'againstVotesBp';
          type: 'u64';
        },
        {
          name: 'abstainVotesBp';
          type: 'u64';
        },
        {
          name: 'stakeMerkleProof';
          type: {
            vec: {
              array: ['u8', 32];
            };
          };
        },
        {
          name: 'stakeMerkleLeaf';
          type: {
            defined: {
              name: 'stakeMerkleLeaf';
            };
          };
        }
      ];
    },
    {
      name: 'createProposal';
      discriminator: [132, 116, 68, 174, 216, 160, 198, 22];
      accounts: [
        {
          name: 'signer';
          writable: true;
          signer: true;
        },
        {
          name: 'proposal';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [112, 114, 111, 112, 111, 115, 97, 108];
              },
              {
                kind: 'arg';
                path: 'seed';
              },
              {
                kind: 'account';
                path: 'splVoteAccount';
              }
            ];
          };
        },
        {
          name: 'proposalIndex';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [105, 110, 100, 101, 120];
              }
            ];
          };
        },
        {
          name: 'splVoteAccount';
        },
        {
          name: 'systemProgram';
          address: '11111111111111111111111111111111';
        }
      ];
      args: [
        {
          name: 'seed';
          type: 'u64';
        },
        {
          name: 'title';
          type: 'string';
        },
        {
          name: 'description';
          type: 'string';
        }
      ];
    },
    {
      name: 'finalizeProposal';
      discriminator: [23, 68, 51, 167, 109, 173, 187, 164];
      accounts: [
        {
          name: 'signer';
          signer: true;
        },
        {
          name: 'proposal';
          writable: true;
        }
      ];
      args: [];
    },
    {
      name: 'flushMerkleRoot';
      discriminator: [10, 71, 17, 246, 162, 57, 144, 87];
      accounts: [
        {
          name: 'signer';
          writable: true;
          signer: true;
        },
        {
          name: 'proposal';
          writable: true;
        },
        {
          name: 'splVoteAccount';
        },
        {
          name: 'ballotBox';
        },
        {
          name: 'ballotProgram';
        },
        {
          name: 'systemProgram';
          address: '11111111111111111111111111111111';
        }
      ];
      args: [];
    },
    {
      name: 'initializeIndex';
      discriminator: [204, 67, 3, 74, 139, 139, 233, 10];
      accounts: [
        {
          name: 'signer';
          writable: true;
          signer: true;
        },
        {
          name: 'proposalIndex';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [105, 110, 100, 101, 120];
              }
            ];
          };
        },
        {
          name: 'systemProgram';
          address: '11111111111111111111111111111111';
        }
      ];
      args: [];
    },
    {
      name: 'modifyVote';
      discriminator: [116, 52, 102, 0, 121, 145, 27, 139];
      accounts: [
        {
          name: 'signer';
          signer: true;
        },
        {
          name: 'proposal';
          writable: true;
        },
        {
          name: 'vote';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [118, 111, 116, 101];
              },
              {
                kind: 'account';
                path: 'proposal';
              },
              {
                kind: 'account';
                path: 'splVoteAccount';
              }
            ];
          };
        },
        {
          name: 'splVoteAccount';
        },
        {
          name: 'snapshotProgram';
        },
        {
          name: 'consensusResult';
        },
        {
          name: 'metaMerkleProof';
        },
        {
          name: 'systemProgram';
          address: '11111111111111111111111111111111';
        }
      ];
      args: [
        {
          name: 'forVotesBp';
          type: 'u64';
        },
        {
          name: 'againstVotesBp';
          type: 'u64';
        },
        {
          name: 'abstainVotesBp';
          type: 'u64';
        }
      ];
    },
    {
      name: 'modifyVoteOverride';
      discriminator: [42, 54, 123, 87, 239, 152, 22, 186];
      accounts: [
        {
          name: 'signer';
          signer: true;
        },
        {
          name: 'proposal';
          writable: true;
        },
        {
          name: 'validatorVote';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [118, 111, 116, 101];
              },
              {
                kind: 'account';
                path: 'proposal';
              },
              {
                kind: 'account';
                path: 'splVoteAccount';
              }
            ];
          };
        },
        {
          name: 'splVoteAccount';
        },
        {
          name: 'voteOverride';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [
                  118,
                  111,
                  116,
                  101,
                  95,
                  111,
                  118,
                  101,
                  114,
                  114,
                  105,
                  100,
                  101
                ];
              },
              {
                kind: 'account';
                path: 'proposal';
              },
              {
                kind: 'account';
                path: 'splStakeAccount';
              },
              {
                kind: 'account';
                path: 'validatorVote';
              }
            ];
          };
        },
        {
          name: 'voteOverrideCache';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [
                  118,
                  111,
                  116,
                  101,
                  95,
                  111,
                  118,
                  101,
                  114,
                  114,
                  105,
                  100,
                  101,
                  95,
                  99,
                  97,
                  99,
                  104,
                  101
                ];
              },
              {
                kind: 'account';
                path: 'proposal';
              },
              {
                kind: 'account';
                path: 'validatorVote';
              }
            ];
          };
        },
        {
          name: 'splStakeAccount';
        },
        {
          name: 'snapshotProgram';
        },
        {
          name: 'consensusResult';
        },
        {
          name: 'metaMerkleProof';
        },
        {
          name: 'systemProgram';
          address: '11111111111111111111111111111111';
        }
      ];
      args: [
        {
          name: 'forVotesBp';
          type: 'u64';
        },
        {
          name: 'againstVotesBp';
          type: 'u64';
        },
        {
          name: 'abstainVotesBp';
          type: 'u64';
        },
        {
          name: 'stakeMerkleProof';
          type: {
            vec: {
              array: ['u8', 32];
            };
          };
        },
        {
          name: 'stakeMerkleLeaf';
          type: {
            defined: {
              name: 'stakeMerkleLeaf';
            };
          };
        }
      ];
    },
    {
      name: 'supportProposal';
      discriminator: [95, 239, 233, 199, 201, 62, 90, 27];
      accounts: [
        {
          name: 'signer';
          writable: true;
          signer: true;
        },
        {
          name: 'proposal';
          writable: true;
        },
        {
          name: 'support';
          writable: true;
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [115, 117, 112, 112, 111, 114, 116];
              },
              {
                kind: 'account';
                path: 'proposal';
              },
              {
                kind: 'account';
                path: 'splVoteAccount';
              }
            ];
          };
        },
        {
          name: 'splVoteAccount';
        },
        {
          name: 'ballotBox';
          writable: true;
        },
        {
          name: 'ballotProgram';
        },
        {
          name: 'programConfig';
          pda: {
            seeds: [
              {
                kind: 'const';
                value: [
                  80,
                  114,
                  111,
                  103,
                  114,
                  97,
                  109,
                  67,
                  111,
                  110,
                  102,
                  105,
                  103
                ];
              }
            ];
            program: {
              kind: 'account';
              path: 'ballotProgram';
            };
          };
        },
        {
          name: 'systemProgram';
          address: '11111111111111111111111111111111';
        }
      ];
      args: [];
    }
  ];
  accounts: [
    {
      name: 'proposal';
      discriminator: [26, 94, 189, 187, 116, 136, 53, 33];
    },
    {
      name: 'proposalIndex';
      discriminator: [83, 97, 143, 58, 176, 46, 177, 195];
    },
    {
      name: 'support';
      discriminator: [247, 108, 3, 111, 84, 51, 217, 107];
    },
    {
      name: 'vote';
      discriminator: [96, 91, 104, 57, 145, 35, 172, 155];
    },
    {
      name: 'voteOverride';
      discriminator: [130, 93, 172, 50, 168, 151, 176, 188];
    },
    {
      name: 'voteOverrideCache';
      discriminator: [195, 82, 50, 219, 140, 34, 108, 57];
    }
  ];
  events: [
    {
      name: 'merkleRootFlushed';
      discriminator: [120, 37, 53, 216, 119, 172, 17, 144];
    },
    {
      name: 'proposalCreated';
      discriminator: [186, 8, 160, 108, 81, 13, 51, 206];
    },
    {
      name: 'proposalFinalized';
      discriminator: [159, 104, 210, 220, 86, 209, 61, 51];
    },
    {
      name: 'proposalSupported';
      discriminator: [248, 220, 71, 30, 127, 209, 67, 231];
    },
    {
      name: 'proposalTimingAdjusted';
      discriminator: [151, 58, 224, 49, 142, 133, 182, 107];
    },
    {
      name: 'voteCast';
      discriminator: [39, 53, 195, 104, 188, 17, 225, 213];
    },
    {
      name: 'voteModified';
      discriminator: [192, 64, 130, 9, 210, 47, 57, 175];
    },
    {
      name: 'voteOverrideCast';
      discriminator: [111, 204, 225, 252, 254, 218, 120, 236];
    },
    {
      name: 'voteOverrideModified';
      discriminator: [235, 74, 153, 225, 242, 72, 228, 9];
    }
  ];
  errors: [
    {
      code: 6000;
      name: 'notEnoughStake';
      msg: 'Minimum stake required to create proposal is 100k';
    },
    {
      code: 6001;
      name: 'titleEmpty';
      msg: 'The title of the proposal cannot be empty';
    },
    {
      code: 6002;
      name: 'titleTooLong';
      msg: 'The title of the proposal is too long, max 50 char';
    },
    {
      code: 6003;
      name: 'descriptionEmpty';
      msg: 'The description of the proposal cannot be empty';
    },
    {
      code: 6004;
      name: 'descriptionTooLong';
      msg: 'The description of the proposal is too long, max 250 char';
    },
    {
      code: 6005;
      name: 'descriptionInvalid';
      msg: 'The description of the proposal must point to a github link';
    },
    {
      code: 6006;
      name: 'invalidProposalId';
      msg: 'Invalid proposal ID';
    },
    {
      code: 6007;
      name: 'votingNotStarted';
      msg: 'Voting on proposal not yet started';
    },
    {
      code: 6008;
      name: 'proposalClosed';
      msg: 'Proposal closed';
    },
    {
      code: 6009;
      name: 'proposalFinalized';
      msg: 'Proposal finalized';
    },
    {
      code: 6010;
      name: 'invalidVoteDistribution';
      msg: 'Vote distribution must add up to 100% in Basis Points';
    },
    {
      code: 6011;
      name: 'votingPeriodNotEnded';
      msg: 'Voting period not yet ended';
    },
    {
      code: 6012;
      name: 'invalidVoteAccount';
      msg: 'Invalid vote account, proposal id mismatch';
    },
    {
      code: 6013;
      name: 'failedDeserializeNodePubkey';
      msg: 'Failed to deserialize node_pubkey from Vote account';
    },
    {
      code: 6014;
      name: 'voteNodePubkeyMismatch';
      msg: 'Deserialized node_pubkey from Vote accounts does not match';
    },
    {
      code: 6015;
      name: 'notEnoughAccounts';
      msg: 'Not enough accounts for tally';
    },
    {
      code: 6016;
      name: 'invalidClusterStake';
      msg: 'Cluster stake cannot be zero';
    },
    {
      code: 6017;
      name: 'invalidStartEpoch';
      msg: 'Start epoch must be current or future epoch';
    },
    {
      code: 6018;
      name: 'invalidVotingLength';
      msg: 'Voting length must be bigger than 0';
    },
    {
      code: 6019;
      name: 'invalidVoteAccountVersion';
      msg: 'Invalid Vote account version';
    },
    {
      code: 6020;
      name: 'invalidVoteAccountSize';
      msg: 'Invalid Vote account size';
    },
    {
      code: 6021;
      name: 'invalidStakeAccount';
      msg: 'Stake account invalid';
    },
    {
      code: 6022;
      name: 'invalidStakeState';
      msg: 'Stake account invalid';
    },
    {
      code: 6023;
      name: 'invalidStakeAccountSize';
      msg: 'Invalid Stake account size';
    },
    {
      code: 6024;
      name: 'invalidSnapshotProgram';
      msg: 'Invalid Snapshot program: provided program ID does not match the expected Merkle Verifier Service program';
    },
    {
      code: 6025;
      name: 'unauthorizedMerkleRootUpdate';
      msg: 'Only the original proposal author can add the merkle root hash';
    },
    {
      code: 6026;
      name: 'merkleRootAlreadySet';
      msg: 'Merkle root hash is already set for this proposal';
    },
    {
      code: 6027;
      name: 'invalidMerkleRoot';
      msg: 'Merkle root hash cannot be all zeros';
    },
    {
      code: 6028;
      name: 'invalidSnapshotSlot';
      msg: 'Invalid snapshot slot: snapshot slot must be less past or current slot';
    },
    {
      code: 6029;
      name: 'mustBeOwnedBySnapshotProgram';
      msg: 'Account must be owned by Snapshot program';
    },
    {
      code: 6030;
      name: 'invalidConsensusResultPda';
      msg: 'Invalid consensus result PDA';
    },
    {
      code: 6031;
      name: 'cantDeserializeMmppda';
      msg: "Can't deserialize MetaMerkleProof PDA";
    },
    {
      code: 6032;
      name: 'cantDeserializeConsensusResult';
      msg: "Can't deserialize ConsensusResult";
    },
    {
      code: 6033;
      name: 'cannotModifyAfterStart';
      msg: 'Cannot modify proposal after voting has started';
    },
    {
      code: 6034;
      name: 'votingLengthTooLong';
      msg: 'Voting length exceeds maximum allowed epochs';
    },
    {
      code: 6035;
      name: 'arithmeticOverflow';
      msg: 'Arithmetic overflow occurred';
    },
    {
      code: 6036;
      name: 'snapshotProgramUpgraded';
      msg: 'Snapshot program has been upgraded, update protection triggered';
    },
    {
      code: 6037;
      name: 'merkleRootNotSet';
      msg: 'Merkle root hash has not been set for this proposal';
    },
    {
      code: 6038;
      name: 'supportPeriodExpired';
      msg: 'Support period has expired for this proposal';
    },
    {
      code: 6039;
      name: 'consensusResultNotSet';
      msg: 'Consensus result has not been set for this proposal';
    },
    {
      code: 6040;
      name: 'unauthorized';
      msg: 'Unauthorized: caller is not authorized to perform this action';
    }
  ];
  types: [
    {
      name: 'merkleRootFlushed';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proposalId';
            type: 'pubkey';
          },
          {
            name: 'author';
            type: 'pubkey';
          },
          {
            name: 'newSnapshotSlot';
            type: 'u64';
          },
          {
            name: 'flushTimestamp';
            type: 'i64';
          }
        ];
      };
    },
    {
      name: 'proposal';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'author';
            docs: ['The public key of the validator who created this proposal'];
            type: 'pubkey';
          },
          {
            name: 'title';
            type: 'string';
          },
          {
            name: 'description';
            type: 'string';
          },
          {
            name: 'creationEpoch';
            type: 'u64';
          },
          {
            name: 'startEpoch';
            type: 'u64';
          },
          {
            name: 'endEpoch';
            type: 'u64';
          },
          {
            name: 'proposerStakeWeightBp';
            type: 'u64';
          },
          {
            name: 'clusterSupportLamports';
            type: 'u64';
          },
          {
            name: 'forVotesLamports';
            docs: ['Total lamports voted in favor of this proposal'];
            type: 'u64';
          },
          {
            name: 'againstVotesLamports';
            docs: ['Total lamports voted against this proposal'];
            type: 'u64';
          },
          {
            name: 'abstainVotesLamports';
            docs: [
              'Total lamports that abstained from voting on this proposal'
            ];
            type: 'u64';
          },
          {
            name: 'voting';
            type: 'bool';
          },
          {
            name: 'finalized';
            type: 'bool';
          },
          {
            name: 'proposalBump';
            type: 'u8';
          },
          {
            name: 'creationTimestamp';
            type: 'i64';
          },
          {
            name: 'voteCount';
            type: 'u32';
          },
          {
            name: 'index';
            type: 'u32';
          },
          {
            name: 'consensusResult';
            type: {
              option: 'pubkey';
            };
          },
          {
            name: 'snapshotSlot';
            docs: ['Slot number when the validator stake snapshot was taken'];
            type: 'u64';
          },
          {
            name: 'proposalSeed';
            type: 'u64';
          },
          {
            name: 'voteAccountPubkey';
            type: 'pubkey';
          }
        ];
      };
    },
    {
      name: 'proposalCreated';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proposalId';
            type: 'pubkey';
          },
          {
            name: 'author';
            type: 'pubkey';
          },
          {
            name: 'title';
            type: 'string';
          },
          {
            name: 'description';
            type: 'string';
          },
          {
            name: 'startEpoch';
            type: 'u64';
          },
          {
            name: 'endEpoch';
            type: 'u64';
          },
          {
            name: 'snapshotSlot';
            type: 'u64';
          },
          {
            name: 'creationTimestamp';
            type: 'i64';
          }
        ];
      };
    },
    {
      name: 'proposalFinalized';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proposalId';
            type: 'pubkey';
          },
          {
            name: 'finalizer';
            type: 'pubkey';
          },
          {
            name: 'totalForVotes';
            type: 'u64';
          },
          {
            name: 'totalAgainstVotes';
            type: 'u64';
          },
          {
            name: 'totalAbstainVotes';
            type: 'u64';
          },
          {
            name: 'totalVotesCount';
            type: 'u32';
          },
          {
            name: 'finalizationTimestamp';
            type: 'i64';
          }
        ];
      };
    },
    {
      name: 'proposalIndex';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'currentIndex';
            type: 'u32';
          },
          {
            name: 'bump';
            type: 'u8';
          }
        ];
      };
    },
    {
      name: 'proposalSupported';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proposalId';
            type: 'pubkey';
          },
          {
            name: 'supporter';
            type: 'pubkey';
          },
          {
            name: 'clusterSupportLamports';
            type: 'u64';
          },
          {
            name: 'votingActivated';
            type: 'bool';
          },
          {
            name: 'snapshotSlot';
            type: 'u64';
          }
        ];
      };
    },
    {
      name: 'proposalTimingAdjusted';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proposalId';
            type: 'pubkey';
          },
          {
            name: 'author';
            type: 'pubkey';
          },
          {
            name: 'newCreationTimestamp';
            type: 'i64';
          },
          {
            name: 'newCreationEpoch';
            type: 'u64';
          },
          {
            name: 'newStartEpoch';
            type: 'u64';
          },
          {
            name: 'newEndEpoch';
            type: 'u64';
          },
          {
            name: 'newSnapshotSlot';
            type: 'u64';
          },
          {
            name: 'adjustmentTimestamp';
            type: 'i64';
          }
        ];
      };
    },
    {
      name: 'stakeMerkleLeaf';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'votingWallet';
            docs: [
              'Wallet designated for governance voting for the stake account.'
            ];
            type: 'pubkey';
          },
          {
            name: 'stakeAccount';
            docs: ['The stake account address.'];
            type: 'pubkey';
          },
          {
            name: 'activeStake';
            docs: ['Active delegated stake amount.'];
            type: 'u64';
          }
        ];
      };
    },
    {
      name: 'support';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proposal';
            type: 'pubkey';
          },
          {
            name: 'validator';
            type: 'pubkey';
          },
          {
            name: 'bump';
            type: 'u8';
          }
        ];
      };
    },
    {
      name: 'vote';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'validator';
            type: 'pubkey';
          },
          {
            name: 'proposal';
            type: 'pubkey';
          },
          {
            name: 'forVotesBp';
            type: 'u64';
          },
          {
            name: 'againstVotesBp';
            type: 'u64';
          },
          {
            name: 'abstainVotesBp';
            type: 'u64';
          },
          {
            name: 'forVotesLamports';
            type: 'u64';
          },
          {
            name: 'againstVotesLamports';
            type: 'u64';
          },
          {
            name: 'abstainVotesLamports';
            type: 'u64';
          },
          {
            name: 'stake';
            type: 'u64';
          },
          {
            name: 'overrideLamports';
            type: 'u64';
          },
          {
            name: 'voteTimestamp';
            type: 'i64';
          },
          {
            name: 'bump';
            type: 'u8';
          }
        ];
      };
    },
    {
      name: 'voteCast';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proposalId';
            type: 'pubkey';
          },
          {
            name: 'voter';
            type: 'pubkey';
          },
          {
            name: 'voteAccount';
            type: 'pubkey';
          },
          {
            name: 'forVotesBp';
            type: 'u64';
          },
          {
            name: 'againstVotesBp';
            type: 'u64';
          },
          {
            name: 'abstainVotesBp';
            type: 'u64';
          },
          {
            name: 'forVotesLamports';
            type: 'u64';
          },
          {
            name: 'againstVotesLamports';
            type: 'u64';
          },
          {
            name: 'abstainVotesLamports';
            type: 'u64';
          },
          {
            name: 'voteTimestamp';
            type: 'i64';
          }
        ];
      };
    },
    {
      name: 'voteModified';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proposalId';
            type: 'pubkey';
          },
          {
            name: 'voter';
            type: 'pubkey';
          },
          {
            name: 'voteAccount';
            type: 'pubkey';
          },
          {
            name: 'oldForVotesBp';
            type: 'u64';
          },
          {
            name: 'oldAgainstVotesBp';
            type: 'u64';
          },
          {
            name: 'oldAbstainVotesBp';
            type: 'u64';
          },
          {
            name: 'newForVotesBp';
            type: 'u64';
          },
          {
            name: 'newAgainstVotesBp';
            type: 'u64';
          },
          {
            name: 'newAbstainVotesBp';
            type: 'u64';
          },
          {
            name: 'forVotesLamports';
            type: 'u64';
          },
          {
            name: 'againstVotesLamports';
            type: 'u64';
          },
          {
            name: 'abstainVotesLamports';
            type: 'u64';
          },
          {
            name: 'modificationTimestamp';
            type: 'i64';
          }
        ];
      };
    },
    {
      name: 'voteOverride';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'delegator';
            type: 'pubkey';
          },
          {
            name: 'stakeAccount';
            type: 'pubkey';
          },
          {
            name: 'validator';
            type: 'pubkey';
          },
          {
            name: 'proposal';
            type: 'pubkey';
          },
          {
            name: 'voteAccountValidator';
            type: 'pubkey';
          },
          {
            name: 'forVotesBp';
            type: 'u64';
          },
          {
            name: 'againstVotesBp';
            type: 'u64';
          },
          {
            name: 'abstainVotesBp';
            type: 'u64';
          },
          {
            name: 'forVotesLamports';
            type: 'u64';
          },
          {
            name: 'againstVotesLamports';
            type: 'u64';
          },
          {
            name: 'abstainVotesLamports';
            type: 'u64';
          },
          {
            name: 'stakeAmount';
            type: 'u64';
          },
          {
            name: 'voteOverrideTimestamp';
            type: 'i64';
          },
          {
            name: 'bump';
            type: 'u8';
          }
        ];
      };
    },
    {
      name: 'voteOverrideCache';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'validator';
            type: 'pubkey';
          },
          {
            name: 'proposal';
            type: 'pubkey';
          },
          {
            name: 'voteAccountValidator';
            type: 'pubkey';
          },
          {
            name: 'forVotesBp';
            type: 'u64';
          },
          {
            name: 'againstVotesBp';
            type: 'u64';
          },
          {
            name: 'abstainVotesBp';
            type: 'u64';
          },
          {
            name: 'forVotesLamports';
            type: 'u64';
          },
          {
            name: 'againstVotesLamports';
            type: 'u64';
          },
          {
            name: 'abstainVotesLamports';
            type: 'u64';
          },
          {
            name: 'totalStake';
            type: 'u64';
          },
          {
            name: 'bump';
            type: 'u8';
          }
        ];
      };
    },
    {
      name: 'voteOverrideCast';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proposalId';
            type: 'pubkey';
          },
          {
            name: 'delegator';
            type: 'pubkey';
          },
          {
            name: 'stakeAccount';
            type: 'pubkey';
          },
          {
            name: 'validator';
            type: 'pubkey';
          },
          {
            name: 'forVotesBp';
            type: 'u64';
          },
          {
            name: 'againstVotesBp';
            type: 'u64';
          },
          {
            name: 'abstainVotesBp';
            type: 'u64';
          },
          {
            name: 'forVotesLamports';
            type: 'u64';
          },
          {
            name: 'againstVotesLamports';
            type: 'u64';
          },
          {
            name: 'abstainVotesLamports';
            type: 'u64';
          },
          {
            name: 'stakeAmount';
            type: 'u64';
          },
          {
            name: 'voteTimestamp';
            type: 'i64';
          }
        ];
      };
    },
    {
      name: 'voteOverrideModified';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proposalId';
            type: 'pubkey';
          },
          {
            name: 'delegator';
            type: 'pubkey';
          },
          {
            name: 'stakeAccount';
            type: 'pubkey';
          },
          {
            name: 'validator';
            type: 'pubkey';
          },
          {
            name: 'oldForVotesBp';
            type: 'u64';
          },
          {
            name: 'oldAgainstVotesBp';
            type: 'u64';
          },
          {
            name: 'oldAbstainVotesBp';
            type: 'u64';
          },
          {
            name: 'newForVotesBp';
            type: 'u64';
          },
          {
            name: 'newAgainstVotesBp';
            type: 'u64';
          },
          {
            name: 'newAbstainVotesBp';
            type: 'u64';
          },
          {
            name: 'forVotesLamports';
            type: 'u64';
          },
          {
            name: 'againstVotesLamports';
            type: 'u64';
          },
          {
            name: 'abstainVotesLamports';
            type: 'u64';
          },
          {
            name: 'stakeAmount';
            type: 'u64';
          },
          {
            name: 'modificationTimestamp';
            type: 'i64';
          }
        ];
      };
    }
  ];
};
