import { render, screen } from "@testing-library/react";
import QuorumDonut from "../QuorumDonut";

const props = {
  forLamports: 30_000_000_000,
  againstLamports: 10_000_000_000,
  abstainLamports: 10_000_000_000,
};

describe("QuorumDonut", () => {
  it("renders the total when network stake is known", () => {
    render(<QuorumDonut {...props} totalLamports={100_000_000_000} />);
    expect(screen.getByText("Total SOL")).toBeInTheDocument();
    expect(screen.getByText("100.00")).toBeInTheDocument();
  });

  it("renders a dash, not 0.00, when network stake is unknown", () => {
    // Regression (PR #121 review): the caller passed `?? 0`, so the donut
    // showed "0.00 Total SOL" — a real-looking figure — while the vote
    // breakdown beside it correctly showed "—".
    render(<QuorumDonut {...props} totalLamports={undefined} />);
    expect(screen.getByText("—")).toBeInTheDocument();
    expect(screen.queryByText("0.00")).not.toBeInTheDocument();
  });
});
