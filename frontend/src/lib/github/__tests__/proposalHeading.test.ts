import { formatProposalHeading, titleNamesProposal } from "../proposalHeading";
import { makeProposalRef } from "../proposalUrl";

const SGP_3 = makeProposalRef("0003", "sgp");
const SGP_2 = makeProposalRef("0002", "sgp");
const SIMD_22 = makeProposalRef("0022", "simd");

describe("formatProposalHeading - live mainnet titles", () => {
  it("does not repeat a number the proposer already put in the title", () => {
    expect(
      formatProposalHeading(SGP_3, "SGP-0003: Resource and Inclusion Fee"),
    ).toBe("SGP-0003: Resource and Inclusion Fee");
  });

  it("prefixes a title that omits the number", () => {
    expect(formatProposalHeading(SGP_2, "Double Disinflation")).toBe(
      "SGP-0002: Double Disinflation",
    );
  });
});

describe("formatProposalHeading", () => {
  it("returns the title unchanged when no number has resolved", () => {
    expect(formatProposalHeading(undefined, "Double Disinflation")).toBe(
      "Double Disinflation",
    );
  });

  it("leaves the proposer's own spelling alone rather than normalizing it", () => {
    expect(formatProposalHeading(SGP_3, "SGP-3 - Resource Fee")).toBe(
      "SGP-3 - Resource Fee",
    );
  });

  it("prefixes when the title names a different proposal", () => {
    expect(formatProposalHeading(SGP_3, "SGP-0009: Something else")).toBe(
      "SGP-0003: SGP-0009: Something else",
    );
  });
});

describe("titleNamesProposal", () => {
  it.each([
    ["SGP-0003: Resource and Inclusion Fee"],
    ["SGP-3: Resource and Inclusion Fee"],
    ["sgp-0003: resource and inclusion fee"],
    ["SGP 0003 - Resource and Inclusion Fee"],
    ["SGP0003 Resource and Inclusion Fee"],
    ["#SGP-0003: Resource and Inclusion Fee"],
    ["  SGP-0003: Resource and Inclusion Fee"],
  ])("recognizes %j", (title) => {
    expect(titleNamesProposal(title, SGP_3)).toBe(true);
  });

  it.each([
    ["Double Disinflation"],
    ["Resource and Inclusion Fee"],
    // A different number.
    ["SGP-0009: Resource and Inclusion Fee"],
    // Right number, wrong flavour.
    ["SIMD-0003: Resource and Inclusion Fee"],
    // Names the proposal, but not at the start.
    ["Fees, see SGP-0003"],
  ])("rejects %j", (title) => {
    expect(titleNamesProposal(title, SGP_3)).toBe(false);
  });

  it("works for SIMD titles too", () => {
    expect(titleNamesProposal("SIMD-0022: Multi Stake", SIMD_22)).toBe(true);
    expect(titleNamesProposal("Multi Stake", SIMD_22)).toBe(false);
  });
});
