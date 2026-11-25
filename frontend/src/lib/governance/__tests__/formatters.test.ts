import { formatLamportsDisplay, formatSOL } from "../formatters";

const SOL = 1_000_000_000n;

describe("formatSOL", () => {
  it("groups thousands and rounds to the nearest hundredth", () => {
    // Regression: the bigint path rendered "1234567.89", dropping the
    // separators and truncating instead of rounding.
    expect(formatSOL(1_234_567_899_000_000n)).toBe("1,234,567.90");
    expect(formatSOL(0n)).toBe("0.00");
    expect(formatSOL(4_999_999n)).toBe("0.00");
    expect(formatSOL(5_000_000n)).toBe("0.01");
  });

  it("uses compact notation from 10M SOL", () => {
    expect(formatSOL(9_999_999n * SOL)).toBe("9,999,999.00");
    expect(formatSOL(12_345_678n * SOL)).toBe("12.35M");
    expect(formatSOL(1_234_567_890n * SOL)).toBe("1.23B");
  });
});

describe("formatLamportsDisplay", () => {
  it("offers the full amount when the value is compacted", () => {
    // Regression: the raw value repeated the compact string.
    expect(formatLamportsDisplay(12_345_678n * SOL + 900_000_000n)).toEqual({
      value: "12.35M SOL",
      rawValue: "12,345,678.90 SOL",
      showRaw: true,
    });
  });

  it("omits the raw value below 10M SOL and renders a dash for no value", () => {
    expect(formatLamportsDisplay(5n * SOL)).toEqual({
      value: "5.00 SOL",
      rawValue: undefined,
      showRaw: false,
    });
    expect(formatLamportsDisplay(undefined)).toEqual({ value: "-", showRaw: false });
  });
});
