import { PublicKey } from "@solana/web3.js";
import { getProposalStatus } from "../proposals";
import type { EpochConstants, GetProposalStatusParams } from "../proposals";

// Mock the SUPPORT_THRESHOLD_PERCENT constant
jest.mock("@/components/proposals/detail/support-phase-progress", () => ({
  SUPPORT_THRESHOLD_PERCENT: 10,
}));

describe("getProposalStatus", () => {
  const creationEpoch = 800;
  const totalStakedLamports = 100_000_000_000; // 100M SOL in lamports
  const requiredThresholdLamports = totalStakedLamports * 0.1; // 10% = 10M SOL
  const mockConsensusResult = new PublicKey("11111111111111111111111111111111");

  // Epoch lengths aligned with former testnet defaults used by these tests
  const epochs: EpochConstants = {
    SUPPORT_EPOCHS: 1,
    DISCUSSION_EPOCHS: 2,
    SNAPSHOT_EPOCHS: 1,
    VOTING_EPOCHS: 4,
  };

  // When support is reached at supportEndEpoch, backend sets startEpoch as:
  // startEpoch = supportEndEpoch + DISCUSSION_EPOCHS + SNAPSHOT_EPOCHS
  const supportEndEpoch = creationEpoch + epochs.SUPPORT_EPOCHS + 1; // epoch 802
  const startEpochWhenSupportReached =
    supportEndEpoch + epochs.DISCUSSION_EPOCHS + epochs.SNAPSHOT_EPOCHS; // epoch 805
  const endEpochWhenSupportReached =
    startEpochWhenSupportReached + epochs.VOTING_EPOCHS; // epoch 809

  // Default startEpoch: when voting is false, this doesn't matter
  // When voting is true, this should be the calculated startEpoch when support was reached
  const defaultStartEpoch = startEpochWhenSupportReached; // epoch 805
  const defaultEndEpoch = endEpochWhenSupportReached; // epoch 809

  const baseParams: Omit<
    GetProposalStatusParams,
    "currentEpoch" | "clusterSupportLamports"
  > = {
    creationEpoch,
    startEpoch: defaultStartEpoch,
    endEpoch: defaultEndEpoch,
    totalStakedLamports,
    consensusResult: undefined,
    finalized: false,
    voting: false,
    epochConstants: epochs,
  };

  const testSuites = [
    {
      describe: "finalized proposals",
      testCases: [
        {
          description:
            "should return 'finalized' when finalized is true, regardless of other params",
          params: {
            currentEpoch: 900,
            clusterSupportLamports: 0,
            finalized: true,
            voting: false,
          },
          expected: "finalized" as const,
        },
        {
          description: "should return 'finalized' even if voting flag is true",
          params: {
            currentEpoch: 900,
            clusterSupportLamports: 0,
            finalized: true,
            voting: true,
          },
          expected: "finalized" as const,
        },
      ],
    },
    {
      describe: "during support phase",
      testCases: [
        {
          description:
            "should return 'supporting' when currentEpoch equals support start epoch (creationEpoch)",
          params: {
            currentEpoch: creationEpoch, // epoch 800, support phase starts
            clusterSupportLamports: 0,
          },
          expected: "supporting" as const,
        },
        {
          description:
            "should return 'supporting' during support phase (epoch 801)",
          params: {
            currentEpoch: creationEpoch + 1, // epoch 801, still in support phase
            clusterSupportLamports: 0,
          },
          expected: "supporting" as const,
        },
      ],
    },
    {
      describe: "at support end epoch (threshold check)",
      testCases: [
        {
          description:
            "should return 'failed' when threshold is not met at support end epoch",
          params: {
            currentEpoch: creationEpoch + 2, // epoch 802
            clusterSupportLamports: requiredThresholdLamports - 1, // Just below threshold
          },
          expected: "failed" as const,
        },
        {
          description:
            "should return 'discussion' when threshold is met at support end epoch",
          params: {
            currentEpoch: creationEpoch + 2, // epoch 802
            clusterSupportLamports: requiredThresholdLamports, // Exactly at threshold
          },
          expected: "discussion" as const,
        },
        {
          description:
            "should return 'discussion' when threshold is exceeded at support end epoch",
          params: {
            currentEpoch: creationEpoch + 2, // epoch 802
            clusterSupportLamports: requiredThresholdLamports + 1_000_000, // Above threshold
          },
          expected: "discussion" as const,
        },
      ],
    },
    {
      describe: "during discussion phase",
      testCases: [
        {
          description:
            "should return 'discussion' at discussion start epoch (epoch 802) when threshold was met",
          params: {
            currentEpoch: creationEpoch + 2, // epoch 802 - threshold check returns discussion if met
            clusterSupportLamports: requiredThresholdLamports, // Threshold met
          },
          expected: "discussion" as const,
        },
        {
          description:
            "should return 'discussion' during middle of discussion phase (epoch 803)",
          params: {
            currentEpoch: creationEpoch + 3, // epoch 803
            clusterSupportLamports: requiredThresholdLamports, // Threshold was met
            voting: true, // Threshold was met, so voting flag should be true
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
          },
          expected: "discussion" as const,
        },
        {
          description:
            "should return 'discussion' at discussion end epoch (epoch 804)",
          params: {
            currentEpoch: creationEpoch + 4, // epoch 804
            clusterSupportLamports: requiredThresholdLamports, // Threshold was met
            voting: true, // Threshold was met, so voting flag should be true
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
          },
          expected: "discussion" as const,
        },
        {
          description:
            "should return 'failed' during discussion phase (epoch 803) if threshold was not met (voting flag is false)",
          params: {
            currentEpoch: creationEpoch + 3, // epoch 803 - in discussion phase range
            clusterSupportLamports: requiredThresholdLamports - 1, // Threshold not met
            voting: false, // On-chain flag indicates threshold was not met
          },
          expected: "failed" as const,
        },
        {
          description:
            "should return 'failed' during discussion phase (epoch 804) if threshold was not met (voting flag is false)",
          params: {
            currentEpoch: creationEpoch + 4, // epoch 804 - in discussion phase range
            clusterSupportLamports: requiredThresholdLamports - 1, // Threshold not met
            voting: false, // On-chain flag indicates threshold was not met
          },
          expected: "failed" as const,
        },
      ],
    },
    {
      describe: "snapshot phase",
      testCases: [
        {
          description:
            "should return 'discussion' at snapshot epoch (epoch 805) when threshold was met",
          params: {
            currentEpoch: creationEpoch + 5, // epoch 805
            clusterSupportLamports: requiredThresholdLamports,
            voting: true, // Threshold was met
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
          },
          expected: "discussion" as const,
        },
      ],
    },
    {
      describe: "past discussion phase - using voting flag",
      testCases: [
        {
          description:
            "should return 'failed' when voting flag is false after discussion phase (support threshold was not met)",
          params: {
            currentEpoch: creationEpoch + 6, // epoch 806
            clusterSupportLamports: requiredThresholdLamports - 1, // Threshold not met
            voting: false, // On-chain flag indicates threshold was not met
          },
          expected: "failed" as const,
        },
        {
          description:
            "should return 'failed' when voting flag is false at epoch 807 (support threshold was not met)",
          params: {
            currentEpoch: creationEpoch + 7,
            clusterSupportLamports: requiredThresholdLamports - 1, // Threshold not met
            voting: false, // On-chain flag indicates threshold was not met
          },
          expected: "failed" as const,
        },
        {
          description:
            "should return 'failed' when voting flag is false at snapshot epoch (epoch 805) if threshold was not met",
          params: {
            currentEpoch: creationEpoch + 5, // epoch 805
            clusterSupportLamports: requiredThresholdLamports - 1, // Threshold not met
            voting: false, // On-chain flag indicates threshold was not met
          },
          expected: "failed" as const,
        },
        {
          description:
            "should return 'failed' when voting flag is false even if currentEpoch >= endEpoch (failed proposals should show as failed, not finalized)",
          params: {
            currentEpoch: defaultEndEpoch, // epoch 809 - equals endEpoch
            clusterSupportLamports: requiredThresholdLamports - 1, // Threshold not met
            voting: false, // On-chain flag indicates threshold was not met
          },
          expected: "failed" as const,
        },
        {
          description:
            "should return 'failed' when voting flag is false even if currentEpoch > endEpoch (failed proposals should show as failed, not finalized)",
          params: {
            currentEpoch: defaultEndEpoch + 10, // epoch 819 - past endEpoch
            clusterSupportLamports: requiredThresholdLamports - 1, // Threshold not met
            voting: false, // On-chain flag indicates threshold was not met
          },
          expected: "failed" as const,
        },
      ],
    },
    {
      describe: "voting phase - with consensusResult",
      testCases: [
        {
          description:
            "should return 'voting' when voting flag is true, consensusResult exists, and at voting start epoch",
          params: {
            currentEpoch: startEpochWhenSupportReached, // epoch 805 - voting start epoch
            clusterSupportLamports: requiredThresholdLamports,
            consensusResult: mockConsensusResult,
            voting: true,
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
          },
          expected: "voting" as const,
        },
        {
          description:
            "should return 'voting' when voting flag is true, consensusResult exists, and past voting start epoch but before end epoch",
          params: {
            currentEpoch: endEpochWhenSupportReached - 1, // epoch 808 - before end epoch
            clusterSupportLamports: requiredThresholdLamports,
            consensusResult: mockConsensusResult,
            voting: true,
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
          },
          expected: "voting" as const,
        },
        {
          description:
            "should return 'finalized' when currentEpoch equals endEpoch (voting has ended)",
          params: {
            currentEpoch: endEpochWhenSupportReached, // epoch 809 - equals endEpoch
            clusterSupportLamports: requiredThresholdLamports,
            consensusResult: mockConsensusResult,
            voting: true,
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
            finalized: false,
          },
          expected: "finalized" as const,
        },
        {
          description:
            "should return 'finalized' when currentEpoch exceeds endEpoch (voting has ended)",
          params: {
            currentEpoch: endEpochWhenSupportReached + 1, // epoch 810 - past endEpoch
            clusterSupportLamports: requiredThresholdLamports,
            consensusResult: mockConsensusResult,
            voting: true,
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
            finalized: false,
          },
          expected: "finalized" as const,
        },
      ],
    },
    {
      describe: "voting phase - without consensusResult (snapshot not ready)",
      testCases: [
        {
          description:
            "should return 'discussion' when voting flag is true but consensusResult is undefined at voting start epoch",
          params: {
            currentEpoch: startEpochWhenSupportReached, // epoch 805 - voting start epoch
            clusterSupportLamports: requiredThresholdLamports,
            consensusResult: undefined,
            voting: true,
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
          },
          expected: "discussion" as const,
        },
        {
          description:
            "should return 'discussion' when voting flag is true but consensusResult is undefined past voting start epoch but before end epoch",
          params: {
            currentEpoch: endEpochWhenSupportReached - 1, // epoch 808 - before end epoch
            clusterSupportLamports: requiredThresholdLamports,
            consensusResult: undefined,
            voting: true,
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
          },
          expected: "discussion" as const,
        },
      ],
    },
    {
      describe: "edge cases",
      testCases: [
        {
          description: "should handle zero total staked lamports",
          params: {
            currentEpoch: creationEpoch + 2, // epoch 802
            totalStakedLamports: 0,
            clusterSupportLamports: 0,
          },
          expected: "discussion" as const,
          note: "With zero total stake, threshold is 0, so any support should pass",
        },
        {
          description:
            "should return 'finalized' for very large epoch numbers (past endEpoch)",
          params: {
            currentEpoch: creationEpoch + 1000,
            clusterSupportLamports: requiredThresholdLamports,
            consensusResult: mockConsensusResult,
            voting: true, // Threshold was met
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
            finalized: false,
          },
          expected: "finalized" as const,
        },
        {
          description:
            "should return 'supporting' as fallback for epoch 801 (during support phase)",
          params: {
            currentEpoch: creationEpoch + 1, // epoch 801
            clusterSupportLamports: requiredThresholdLamports,
            voting: false,
          },
          expected: "supporting" as const,
          note: "Epoch 801 is during support phase, falls through to fallback",
        },
      ],
    },
    {
      describe: "threshold calculation",
      testCases: [
        {
          description:
            "should correctly calculate threshold as 10% of total staked - just below threshold",
          params: {
            currentEpoch: creationEpoch + 2,
            totalStakedLamports: 1_000_000_000, // 1M SOL
            clusterSupportLamports: 100_000_000 - 1, // Just below 10% = 100k SOL
          },
          expected: "failed" as const,
        },
        {
          description:
            "should correctly calculate threshold as 10% of total staked - exactly at threshold",
          params: {
            currentEpoch: creationEpoch + 2,
            totalStakedLamports: 1_000_000_000, // 1M SOL
            clusterSupportLamports: 100_000_000, // Exactly 10% = 100k SOL
          },
          expected: "discussion" as const,
        },
      ],
    },
  ];

  // Iterate over test suites to create describe blocks and tests
  testSuites.forEach(({ describe: describeTitle, testCases }) => {
    describe(describeTitle, () => {
      testCases.forEach(({ description, params, expected }) => {
        it(description, () => {
          const result = getProposalStatus({
            ...baseParams,
            ...params,
          });
          expect(result).toBe(expected);
        });
      });
    });
  });

  // Test that verifies startEpoch calculation when support is reached
  describe("startEpoch calculation when support is reached", () => {
    it("should use startEpoch = supportEndEpoch + DISCUSSION_EPOCHS + SNAPSHOT_EPOCHS when voting is true", () => {
      // When support is reached at supportEndEpoch (epoch 802), backend sets:
      // startEpoch = supportEndEpoch + DISCUSSION_EPOCHS + SNAPSHOT_EPOCHS
      // For testnet: 802 + 2 + 1 = 805
      const expectedStartEpoch =
        supportEndEpoch + epochs.DISCUSSION_EPOCHS + epochs.SNAPSHOT_EPOCHS;
      expect(expectedStartEpoch).toBe(startEpochWhenSupportReached);
      expect(expectedStartEpoch).toBe(805); // Verify the calculation

      const expectedEndEpoch = expectedStartEpoch + epochs.VOTING_EPOCHS;

      // Test that before startEpoch, proposal is in discussion phase
      const beforeVotingStart = getProposalStatus({
        ...baseParams,
        currentEpoch: expectedStartEpoch - 1, // epoch 804
        clusterSupportLamports: requiredThresholdLamports,
        voting: true,
        startEpoch: expectedStartEpoch,
        endEpoch: expectedEndEpoch,
      });
      expect(beforeVotingStart).toBe("discussion");

      // Test that at startEpoch with consensusResult, proposal is in voting phase
      const atVotingStart = getProposalStatus({
        ...baseParams,
        currentEpoch: expectedStartEpoch, // epoch 805
        clusterSupportLamports: requiredThresholdLamports,
        consensusResult: mockConsensusResult,
        voting: true,
        startEpoch: expectedStartEpoch,
        endEpoch: expectedEndEpoch,
      });
      expect(atVotingStart).toBe("voting");
    });
  });

  // Phase transitions tests (these need multiple assertions per test)
  describe("phase transitions", () => {
    it("should transition correctly from supporting to discussion when threshold is met", () => {
      const testCases = [
        {
          description: "support phase",
          params: {
            currentEpoch: creationEpoch, // epoch 800 - support phase
            clusterSupportLamports: requiredThresholdLamports,
          },
          expected: "supporting" as const,
        },
        {
          description: "threshold check",
          params: {
            currentEpoch: creationEpoch + 2, // epoch 802 - threshold check
            clusterSupportLamports: requiredThresholdLamports,
          },
          expected: "discussion" as const,
        },
      ];

      testCases.forEach(({ params, expected }) => {
        expect(
          getProposalStatus({
            ...baseParams,
            ...params,
          }),
        ).toBe(expected);
      });
    });

    it("should transition correctly from supporting to failed when threshold is not met", () => {
      const testCases = [
        {
          description: "support phase",
          params: {
            currentEpoch: creationEpoch, // epoch 800 - support phase
            clusterSupportLamports: requiredThresholdLamports - 1,
          },
          expected: "supporting" as const,
        },
        {
          description: "threshold check",
          params: {
            currentEpoch: creationEpoch + 2, // epoch 802 - threshold check
            clusterSupportLamports: requiredThresholdLamports - 1,
          },
          expected: "failed" as const,
        },
      ];

      testCases.forEach(({ params, expected }) => {
        expect(
          getProposalStatus({
            ...baseParams,
            ...params,
          }),
        ).toBe(expected);
      });
    });

    it("should remain failed after support phase ends if threshold was not met", () => {
      const testCases = [
        {
          description: "threshold check",
          params: {
            currentEpoch: creationEpoch + 2, // epoch 802 - threshold check
            clusterSupportLamports: requiredThresholdLamports - 1,
          },
          expected: "failed" as const,
        },
        {
          description: "after support phase",
          params: {
            currentEpoch: creationEpoch + 6, // epoch 806 - past discussion phase
            clusterSupportLamports: requiredThresholdLamports - 1,
            voting: false, // Threshold was not met
          },
          expected: "failed" as const,
        },
      ];

      testCases.forEach(({ params, expected }) => {
        expect(
          getProposalStatus({
            ...baseParams,
            ...params,
          }),
        ).toBe(expected);
      });
    });

    it("should progress to discussion and voting when threshold was met", () => {
      const testCases = [
        {
          description: "threshold check",
          params: {
            currentEpoch: creationEpoch + 2, // epoch 802 - threshold check
            clusterSupportLamports: requiredThresholdLamports,
          },
          expected: "discussion" as const,
        },
        {
          description: "discussion phase",
          params: {
            currentEpoch: creationEpoch + 3, // epoch 803 - discussion phase
            clusterSupportLamports: requiredThresholdLamports,
            voting: true, // Threshold was met
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
          },
          expected: "discussion" as const,
        },
        {
          description: "voting phase",
          params: {
            currentEpoch: startEpochWhenSupportReached, // epoch 805 - voting phase starts
            clusterSupportLamports: requiredThresholdLamports,
            consensusResult: mockConsensusResult,
            voting: true, // Threshold was met
            startEpoch: startEpochWhenSupportReached, // epoch 805 - when support reached at epoch 802
            endEpoch: endEpochWhenSupportReached, // epoch 809
          },
          expected: "voting" as const,
        },
      ];

      testCases.forEach(({ params, expected }) => {
        expect(
          getProposalStatus({
            ...baseParams,
            ...params,
          }),
        ).toBe(expected);
      });
    });
  });
});
