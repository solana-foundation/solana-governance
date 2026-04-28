/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/gov_v1.json`.
 */
export type GovV1 = {
  address: "HsiPGfvF441TrU8eCLLerQDcEMtyzYszBfKxqoSuaD9R";
  metadata: {
    name: "govV1";
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
          name: "operator";
          signer: true;
        },
        {
          name: "ballotBox";
          writable: true;
        },
      ];
      args: [
        {
          name: "ballot";
          type: {
            defined: {
              name: "ballot";
            };
          };
        },
      ];
    },
    {
      name: "closeMetaMerkleProof";
      discriminator: [248, 239, 182, 146, 23, 215, 172, 3];
      accounts: [
        {
          name: "payer";
          docs: ["Account to receive the reclaimed rent from StakingRecord"];
          writable: true;
          relations: ["metaMerkleProof"];
        },
        {
          name: "metaMerkleProof";
          writable: true;
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
      ];
      args: [];
    },
    {
      name: "createConsensusResult";
      discriminator: [21, 90, 183, 65, 163, 107, 168, 89];
      accounts: [
        {
          name: "payer";
          writable: true;
          signer: true;
        },
        {
          name: "consensusResult";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  67,
                  111,
                  110,
                  115,
                  101,
                  110,
                  115,
                  117,
                  115,
                  82,
                  101,
                  115,
                  117,
                  108,
                  116,
                ];
              },
              {
                kind: "arg";
                path: "snapshotSlot";
              },
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
      ];
      args: [
        {
          name: "snapshotSlot";
          type: "u64";
        },
        {
          name: "metaMerkleRoot";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "snapshotHash";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "tieBreakerConsensus";
          type: "bool";
        },
      ];
    },
    {
      name: "finalizeBallot";
      discriminator: [212, 43, 85, 58, 158, 34, 41, 42];
      accounts: [
        {
          name: "payer";
          writable: true;
          signer: true;
        },
        {
          name: "ballotBox";
        },
        {
          name: "consensusResult";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  67,
                  111,
                  110,
                  115,
                  101,
                  110,
                  115,
                  117,
                  115,
                  82,
                  101,
                  115,
                  117,
                  108,
                  116,
                ];
              },
              {
                kind: "account";
                path: "ballot_box.snapshot_slot";
                account: "ballotBox";
              },
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
      ];
      args: [];
    },
    {
      name: "finalizeProposedAuthority";
      discriminator: [89, 96, 108, 130, 223, 223, 213, 102];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "programConfig";
          writable: true;
        },
      ];
      args: [];
    },
    {
      name: "initBallotBox";
      discriminator: [164, 20, 45, 213, 67, 43, 193, 212];
      accounts: [
        {
          name: "payer";
          writable: true;
          signer: true;
        },
        {
          name: "proposal";
          docs: [
            "Verifies that signer is a Proposal PDA from the governance program.",
            "When `skip-pda-check` feature is enabled, this check is disabled to allow local testing without CPI.",
          ];
          signer: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [112, 114, 111, 112, 111, 115, 97, 108];
              },
              {
                kind: "arg";
                path: "proposalSeed";
              },
              {
                kind: "arg";
                path: "splVoteAccount";
              },
            ];
            program: {
              kind: "const";
              value: [
                79,
                139,
                255,
                234,
                216,
                23,
                1,
                144,
                122,
                105,
                236,
                235,
                161,
                220,
                75,
                80,
                111,
                167,
                231,
                208,
                4,
                240,
                180,
                181,
                58,
                174,
                189,
                60,
                177,
                123,
                49,
                181,
              ];
            };
          };
        },
        {
          name: "ballotBox";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [66, 97, 108, 108, 111, 116, 66, 111, 120];
              },
              {
                kind: "arg";
                path: "snapshotSlot";
              },
            ];
          };
        },
        {
          name: "programConfig";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
      ];
      args: [
        {
          name: "snapshotSlot";
          type: "u64";
        },
        {
          name: "proposalSeed";
          type: "u64";
        },
        {
          name: "splVoteAccount";
          type: "pubkey";
        },
      ];
    },
    {
      name: "initMetaMerkleProof";
      discriminator: [190, 210, 132, 165, 204, 88, 110, 84];
      accounts: [
        {
          name: "payer";
          writable: true;
          signer: true;
        },
        {
          name: "merkleProof";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  77,
                  101,
                  116,
                  97,
                  77,
                  101,
                  114,
                  107,
                  108,
                  101,
                  80,
                  114,
                  111,
                  111,
                  102,
                ];
              },
              {
                kind: "account";
                path: "consensusResult";
              },
              {
                kind: "arg";
                path: "meta_merkle_leaf.vote_account";
              },
            ];
          };
        },
        {
          name: "consensusResult";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
      ];
      args: [
        {
          name: "metaMerkleLeaf";
          type: {
            defined: {
              name: "metaMerkleLeaf";
            };
          };
        },
        {
          name: "metaMerkleProof";
          type: {
            vec: {
              array: ["u8", 32];
            };
          };
        },
        {
          name: "closeTimestamp";
          type: "i64";
        },
      ];
    },
    {
      name: "initProgramConfig";
      discriminator: [185, 54, 237, 229, 219, 179, 109, 20];
      accounts: [
        {
          name: "payer";
          writable: true;
          signer: true;
        },
        {
          name: "authority";
          signer: true;
        },
        {
          name: "programConfig";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
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
                  103,
                ];
              },
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
      ];
      args: [];
    },
    {
      name: "removeVote";
      discriminator: [32, 187, 23, 3, 156, 232, 55, 177];
      accounts: [
        {
          name: "operator";
          signer: true;
        },
        {
          name: "ballotBox";
          writable: true;
        },
      ];
      args: [];
    },
    {
      name: "resetBallotBox";
      discriminator: [108, 127, 131, 171, 226, 219, 67, 163];
      accounts: [
        {
          name: "tieBreakerAdmin";
          signer: true;
          relations: ["programConfig"];
        },
        {
          name: "ballotBox";
          writable: true;
        },
        {
          name: "programConfig";
        },
      ];
      args: [];
    },
    {
      name: "setTieBreaker";
      discriminator: [228, 10, 240, 130, 193, 58, 103, 181];
      accounts: [
        {
          name: "tieBreakerAdmin";
          signer: true;
          relations: ["programConfig"];
        },
        {
          name: "ballotBox";
          writable: true;
        },
        {
          name: "programConfig";
        },
      ];
      args: [
        {
          name: "ballot";
          type: {
            defined: {
              name: "ballot";
            };
          };
        },
      ];
    },
    {
      name: "updateOperatorWhitelist";
      discriminator: [25, 65, 144, 150, 200, 245, 156, 92];
      accounts: [
        {
          name: "authority";
          signer: true;
          relations: ["programConfig"];
        },
        {
          name: "programConfig";
          writable: true;
        },
      ];
      args: [
        {
          name: "operatorsToAdd";
          type: {
            option: {
              vec: "pubkey";
            };
          };
        },
        {
          name: "operatorsToRemove";
          type: {
            option: {
              vec: "pubkey";
            };
          };
        },
      ];
    },
    {
      name: "updateProgramConfig";
      discriminator: [214, 3, 187, 98, 170, 106, 33, 45];
      accounts: [
        {
          name: "authority";
          signer: true;
          relations: ["programConfig"];
        },
        {
          name: "programConfig";
          writable: true;
        },
      ];
      args: [
        {
          name: "proposedAuthority";
          type: {
            option: "pubkey";
          };
        },
        {
          name: "minConsensusThresholdBps";
          type: {
            option: "u16";
          };
        },
        {
          name: "tieBreakerAdmin";
          type: {
            option: "pubkey";
          };
        },
        {
          name: "voteDuration";
          type: {
            option: "i64";
          };
        },
      ];
    },
    {
      name: "verifyMerkleProof";
      discriminator: [51, 191, 37, 169, 74, 207, 201, 102];
      accounts: [
        {
          name: "metaMerkleProof";
        },
        {
          name: "consensusResult";
          relations: ["metaMerkleProof"];
        },
      ];
      args: [
        {
          name: "stakeMerkleProof";
          type: {
            option: {
              vec: {
                array: ["u8", 32];
              };
            };
          };
        },
        {
          name: "stakeMerkleLeaf";
          type: {
            option: {
              defined: {
                name: "stakeMerkleLeaf";
              };
            };
          };
        },
      ];
    },
  ];
  accounts: [
    {
      name: "ballotBox";
      discriminator: [155, 169, 156, 8, 92, 14, 24, 101];
    },
    {
      name: "consensusResult";
      discriminator: [105, 121, 122, 243, 100, 58, 93, 161];
    },
    {
      name: "metaMerkleProof";
      discriminator: [130, 55, 141, 26, 195, 58, 18, 178];
    },
    {
      name: "programConfig";
      discriminator: [196, 210, 90, 231, 144, 149, 140, 63];
    },
  ];
  errors: [
    {
      code: 6000;
      name: "operatorNotWhitelisted";
      msg: "Operator not whitelisted";
    },
    {
      code: 6001;
      name: "operatorHasVoted";
      msg: "Operator has voted";
    },
    {
      code: 6002;
      name: "operatorHasNotVoted";
      msg: "Operator has not voted";
    },
    {
      code: 6003;
      name: "votingExpired";
      msg: "Voting has expired";
    },
    {
      code: 6004;
      name: "votingNotExpired";
      msg: "Voting not expired";
    },
    {
      code: 6005;
      name: "consensusReached";
      msg: "Consensus has reached";
    },
    {
      code: 6006;
      name: "consensusNotReached";
      msg: "Consensus not reached";
    },
    {
      code: 6007;
      name: "invalidBallot";
      msg: "Invalid ballot";
    },
    {
      code: 6008;
      name: "invalidMerkleInputs";
      msg: "Invalid merkle inputs";
    },
    {
      code: 6009;
      name: "invalidMerkleProof";
      msg: "Invalid merkle proof";
    },
    {
      code: 6010;
      name: "vecFull";
      msg: "Vector size exceeded";
    },
    {
      code: 6011;
      name: "overlappingWhitelistEntries";
      msg: "Overlapping operators in add and remove lists";
    },
    {
      code: 6012;
      name: "invalidBallotIndex";
      msg: "Invalid ballot index";
    },
    {
      code: 6013;
      name: "invalidSnapshotSlot";
      msg: "Snapshot slot must be greater than current slot";
    },
    {
      code: 6014;
      name: "ballotTalliesNotMaxLength";
      msg: "Ballot tallies not at max length";
    },
    {
      code: 6015;
      name: "invalidProposal";
      msg: "Invalid proposal";
    },
  ];
  types: [
    {
      name: "ballot";
      docs: ["Inner struct of BallotBox"];
      type: {
        kind: "struct";
        fields: [
          {
            name: "metaMerkleRoot";
            docs: ["The merkle root of the meta merkle tree"];
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "snapshotHash";
            docs: ["SHA256 hash of borsh serialized snapshot. Optional."];
            type: {
              array: ["u8", 32];
            };
          },
        ];
      };
    },
    {
      name: "ballotBox";
      type: {
        kind: "struct";
        fields: [
          {
            name: "bump";
            docs: ["Bump seed for the PDA"];
            type: "u8";
          },
          {
            name: "epoch";
            docs: ["The epoch this ballot box is for"];
            type: "u64";
          },
          {
            name: "slotCreated";
            docs: ["Slot when this ballot box was created"];
            type: "u64";
          },
          {
            name: "slotConsensusReached";
            docs: ["Slot when consensus was reached"];
            type: "u64";
          },
          {
            name: "minConsensusThresholdBps";
            docs: [
              "Min. percentage of votes required to finalize for this ballot box.",
            ];
            type: "u16";
          },
          {
            name: "winningBallot";
            docs: [
              "The ballot that got at least min_consensus_threshold of votes",
            ];
            type: {
              defined: {
                name: "ballot";
              };
            };
          },
          {
            name: "operatorVotes";
            docs: ["Operator votes"];
            type: {
              vec: {
                defined: {
                  name: "operatorVote";
                };
              };
            };
          },
          {
            name: "ballotTallies";
            docs: ["Mapping of ballots votes to stake weight"];
            type: {
              vec: {
                defined: {
                  name: "ballotTally";
                };
              };
            };
          },
          {
            name: "voteExpiryTimestamp";
            docs: [
              "Timestamp when voting ends. Tie breaker admin will decide the results",
              "if no consensus is reached by then.",
            ];
            type: "i64";
          },
          {
            name: "snapshotSlot";
            docs: ["Slot for which the snapshot is taken"];
            type: "u64";
          },
          {
            name: "voterList";
            docs: ["Snapshot of whitelisted operators at BallotBox creation"];
            type: {
              vec: "pubkey";
            };
          },
          {
            name: "tieBreakerConsensus";
            docs: ["Whether consensus was reached via tie breaker"];
            type: "bool";
          },
        ];
      };
    },
    {
      name: "ballotTally";
      docs: ["Inner struct of BallotBox"];
      type: {
        kind: "struct";
        fields: [
          {
            name: "index";
            docs: ["Index of the tally within the ballot_tallies"];
            type: "u8";
          },
          {
            name: "ballot";
            docs: ["The ballot being tallied"];
            type: {
              defined: {
                name: "ballot";
              };
            };
          },
          {
            name: "tally";
            docs: [
              "The number of votes for this ballot. Each vote is equally weighted.",
            ];
            type: "u8";
          },
        ];
      };
    },
    {
      name: "consensusResult";
      type: {
        kind: "struct";
        fields: [
          {
            name: "snapshotSlot";
            docs: ["Snapshot slot used for the ballot box"];
            type: "u64";
          },
          {
            name: "ballot";
            docs: ["ballot"];
            type: {
              defined: {
                name: "ballot";
              };
            };
          },
          {
            name: "tieBreakerConsensus";
            docs: ["Whether consensus was reached via tie breaker"];
            type: "bool";
          },
        ];
      };
    },
    {
      name: "metaMerkleLeaf";
      type: {
        kind: "struct";
        fields: [
          {
            name: "votingWallet";
            docs: [
              "Wallet designated for governance voting for the vote account.",
            ];
            type: "pubkey";
          },
          {
            name: "voteAccount";
            docs: ["Validator's vote account."];
            type: "pubkey";
          },
          {
            name: "stakeMerkleRoot";
            docs: [
              "Root hash of the StakeMerkleTree, representing all active stake accounts",
              "delegated to the current vote account.",
            ];
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "activeStake";
            docs: ["Total active delegated stake under this vote account."];
            type: "u64";
          },
        ];
      };
    },
    {
      name: "metaMerkleProof";
      type: {
        kind: "struct";
        fields: [
          {
            name: "payer";
            docs: ["Payer wallet"];
            type: "pubkey";
          },
          {
            name: "consensusResult";
            docs: ["ConsensusResult proof is created for."];
            type: "pubkey";
          },
          {
            name: "metaMerkleLeaf";
            docs: ["Meta merkle leaf"];
            type: {
              defined: {
                name: "metaMerkleLeaf";
              };
            };
          },
          {
            name: "metaMerkleProof";
            docs: ["Meta merkle proof"];
            type: {
              vec: {
                array: ["u8", 32];
              };
            };
          },
          {
            name: "closeTimestamp";
            docs: [
              "Timestamp after which MetaMerkleProof can be closed permissionlessly.",
              "This is selected by the payer but our recommendation is to set to vote expiry time.",
            ];
            type: "i64";
          },
        ];
      };
    },
    {
      name: "operatorVote";
      docs: ["Inner struct of BallotBox"];
      type: {
        kind: "struct";
        fields: [
          {
            name: "operator";
            docs: ["The operator that cast the vote"];
            type: "pubkey";
          },
          {
            name: "slotVoted";
            docs: ["The slot the operator voted"];
            type: "u64";
          },
          {
            name: "ballotIndex";
            docs: ["The index of the ballot in the ballot_tallies"];
            type: "u8";
          },
        ];
      };
    },
    {
      name: "programConfig";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authority";
            docs: ["Authority allowed to update the config."];
            type: "pubkey";
          },
          {
            name: "proposedAuthority";
            docs: ["Authority to be set to upon finalization of proposal."];
            type: {
              option: "pubkey";
            };
          },
          {
            name: "whitelistedOperators";
            docs: [
              "Operators whitelisted to participate in voting.",
              "A snapshot of this list will be taken at the time of BallotBox creation.",
            ];
            type: {
              vec: "pubkey";
            };
          },
          {
            name: "minConsensusThresholdBps";
            docs: [
              "Min. percentage of votes required to finalize a ballot. Used during BallotBox creation.",
            ];
            type: "u16";
          },
          {
            name: "tieBreakerAdmin";
            docs: [
              "Admin allowed to decide the winning ballot if vote expires before consensus.",
            ];
            type: "pubkey";
          },
          {
            name: "voteDuration";
            docs: ["Duration for which ballot box will be opened for voting."];
            type: "i64";
          },
        ];
      };
    },
    {
      name: "stakeMerkleLeaf";
      type: {
        kind: "struct";
        fields: [
          {
            name: "votingWallet";
            docs: [
              "Wallet designated for governance voting for the stake account.",
            ];
            type: "pubkey";
          },
          {
            name: "stakeAccount";
            docs: ["The stake account address."];
            type: "pubkey";
          },
          {
            name: "activeStake";
            docs: ["Active delegated stake amount."];
            type: "u64";
          },
        ];
      };
    },
  ];
};
