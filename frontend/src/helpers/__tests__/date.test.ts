import { calculateVotingEndsIn, estimateSlotTimeMs } from "../date";

describe("estimateSlotTimeMs", () => {
  it("uses the slot-weighted duration from recent performance samples", () => {
    expect(
      estimateSlotTimeMs([
        { numSlots: 170n, samplePeriodSecs: 60 },
        { numSlots: 160n, samplePeriodSecs: 60 },
      ]),
    ).toBeCloseTo(120_000 / 330);
  });

  it("ignores empty samples", () => {
    expect(
      estimateSlotTimeMs([
        { numSlots: 0n, samplePeriodSecs: 60 },
        { numSlots: 165n, samplePeriodSecs: 60 },
      ]),
    ).toBeCloseTo(60_000 / 165);
  });

  it("falls back to 350ms for missing or implausible samples", () => {
    expect(estimateSlotTimeMs([])).toBe(350);
    expect(
      estimateSlotTimeMs([{ numSlots: 0n, samplePeriodSecs: 0 }]),
    ).toBe(350);
    expect(
      estimateSlotTimeMs([{ numSlots: 1n, samplePeriodSecs: 60 }]),
    ).toBe(350);
  });
});

describe("calculateVotingEndsIn", () => {
  it("uses the supplied clock value so countdown renders can update deterministically", () => {
    const endTime = "2026-08-25T00:00:00.000Z";

    expect(
      calculateVotingEndsIn(endTime, Date.parse("2026-08-24T21:29:00.000Z")),
    ).toBe("2h 31m");
    expect(
      calculateVotingEndsIn(endTime, Date.parse("2026-08-24T21:30:00.000Z")),
    ).toBe("2h 30m");
  });
});
