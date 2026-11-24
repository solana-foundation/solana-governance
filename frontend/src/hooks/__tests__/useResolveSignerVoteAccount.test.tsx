import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { fetchRawVoteAccounts } from "@/lib/rpcVoteAccounts";
import { useRawVoteAccounts } from "../useRawVoteAccounts";
import { useResolveSignerVoteAccount } from "../useResolveSignerVoteAccount";

jest.mock("@/contexts/EndpointContext", () => ({
  useEndpoint: () => ({ endpointUrl: "https://rpc.example" }),
}));
jest.mock("@/lib/rpcVoteAccounts", () => ({ fetchRawVoteAccounts: jest.fn() }));

const IDENTITY = "SysvarC1ock11111111111111111111111111111111";
const VOTE_ACCOUNT = "Vote111111111111111111111111111111111111111";

describe("useResolveSignerVoteAccount", () => {
  it("downloads the cluster's vote accounts once across repeated actions and the page's own query", async () => {
    // Every Support click used to download the full vote-account list again,
    // although the page already holds it in the query cache.
    jest.mocked(fetchRawVoteAccounts).mockResolvedValue({
      current: [{ nodePubkey: IDENTITY, votePubkey: VOTE_ACCOUNT }],
      delinquent: [],
    } as unknown as Awaited<ReturnType<typeof fetchRawVoteAccounts>>);
    const queryClient = new QueryClient();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(
      () => ({ resolve: useResolveSignerVoteAccount(), page: useRawVoteAccounts() }),
      { wrapper },
    );

    await expect(result.current.resolve(IDENTITY)).resolves.toBe(VOTE_ACCOUNT);
    await expect(result.current.resolve(IDENTITY)).resolves.toBe(VOTE_ACCOUNT);
    expect(fetchRawVoteAccounts).toHaveBeenCalledTimes(1);
  });
});
