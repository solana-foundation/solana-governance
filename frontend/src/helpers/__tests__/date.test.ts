import { calculateVotingEndsIn, getLiveSlotTimeMs } from "../date";

const FEATURE_OWNER = "Feature111111111111111111111111111111111111";

function featureAccount(activatedAt: bigint | null) {
  const data = new Uint8Array(9);
  if (activatedAt !== null) {
    data[0] = 1;
    new DataView(data.buffer).setBigUint64(1, activatedAt, true);
  }
  return { data, owner: FEATURE_OWNER };
}

describe("getLiveSlotTimeMs", () => {
  it("uses the active 350ms gate after its one-epoch warmup", () => {
    const activationSlot = 440_208_000n;
    expect(
      getLiveSlotTimeMs(activationSlot + 432_000n, [
        featureAccount(activationSlot),
        featureAccount(null),
        featureAccount(null),
        featureAccount(null),
      ]),
    ).toBe(350);
  });

  it("keeps the 400ms baseline during a later gate's warmup epoch", () => {
    const activationSlot = 440_208_000n;
    expect(
      getLiveSlotTimeMs(activationSlot + 431_999n, [
        featureAccount(null),
        featureAccount(activationSlot),
        featureAccount(null),
        featureAccount(null),
      ]),
    ).toBe(400);
  });

  it("rejects invalid feature accounts", () => {
    expect(() =>
      getLiveSlotTimeMs(1_000_000n, [
        featureAccount(1n),
        featureAccount(1n),
        { data: featureAccount(1n).data, owner: "NotAFeatureProgram" },
        featureAccount(null),
      ]),
    ).toThrow("Invalid feature gate account");
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
