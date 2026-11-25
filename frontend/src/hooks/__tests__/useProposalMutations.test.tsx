import type { Address } from "@solana/kit";
import {
  fetchGlobalConfig,
  fetchMaybeVoteDelegation,
  fetchProposal,
} from "@solana/svmgov";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { assertValidProposalDocument } from "@/lib/github";
import { useCreateProposal } from "../useCreateProposal";
import { useSupportProposal } from "../useSupportProposal";
import { useUpdateProposalDescription } from "../useUpdateProposalDescription";

const AUTHOR = "SysvarC1ock11111111111111111111111111111111" as Address;
const DELEGATE = "SysvarRent111111111111111111111111111111111" as Address;
const PROPOSAL = "Config1111111111111111111111111111111111111" as Address;
const VOTE_ACCOUNT = "Vote111111111111111111111111111111111111111" as Address;
const DOCUMENT =
  "https://github.com/solana-foundation/solana-governance-proposals/blob/27bca51e5c0fc34ddbea6904faf86f5098225316/proposals/sgp-0001-title.md";

const mockSignAndSend = jest.fn();
const mockRpc = {
  getEpochInfo: () => ({ send: () => Promise.resolve({ epoch: 900n }) }),
  getEpochSchedule: () => ({
    send: () =>
      Promise.resolve({
        firstNormalEpoch: 0n,
        firstNormalSlot: 0n,
        slotsPerEpoch: 432_000n,
        warmup: false,
      }),
  }),
};

jest.mock("../useSignTransaction", () => ({
  useSignTransaction: () => ({ signAndSend: mockSignAndSend }),
}));
jest.mock("../useResolveSignerVoteAccount", () => ({
  useResolveSignerVoteAccount: () => () => Promise.resolve(VOTE_ACCOUNT),
}));
jest.mock("@/contexts/EndpointContext", () => ({
  useEndpoint: () => ({ endpointUrl: "https://rpc.example" }),
}));
jest.mock("@/contexts/GovernanceConfigContext", () => ({
  useGovernanceConfigContext: () => ({
    data: { discussionEpochs: 2, snapshotEpochExtension: 1, snapshotSlotOffset: 1_000 },
    dataUpdatedAt: 1,
  }),
}));
jest.mock("@solana/kit", () => ({
  ...jest.requireActual("@solana/kit"),
  createSolanaRpc: () => mockRpc,
}));
jest.mock("@solana/svmgov", () => ({
  fetchGlobalConfig: jest.fn(),
  fetchMaybeVoteDelegation: jest.fn(),
  fetchProposal: jest.fn(),
  findGlobalConfigPda: () => Promise.resolve(["GlobalConfigPda"]),
  findVoteDelegationPda: () => Promise.resolve(["VoteDelegationPda"]),
}));
jest.mock("@/lib/github", () => ({
  ...jest.requireActual("@/lib/github"),
  assertValidProposalDocument: jest.fn(),
}));
// Each builder returns its name and input, so a sent transaction can be read back.
jest.mock("@/lib/transactions", () => ({
  buildCreateProposalInstruction: (input: object) => Promise.resolve({ create: input }),
  buildSupportProposalInstruction: (input: object) => Promise.resolve({ support: input }),
  buildUpdateProposalDescriptionInstruction: (input: object) => Promise.resolve({ update: input }),
}));

let sent: unknown[];
let sentOptions: { computeUnitLimit?: number } | undefined;

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>;
}
const mutation = <T,>(hook: () => T) => renderHook(hook, { wrapper }).result.current;

/** Sets whether the proposal's vote account has an accepted vote delegate. */
function voteDelegation(delegation: { accepted: boolean } | null) {
  jest.mocked(fetchMaybeVoteDelegation).mockResolvedValue(
    (delegation
      ? { data: { delegate: DELEGATE, delegationAccepted: delegation.accepted }, exists: true }
      : { exists: false }) as never,
  );
}

beforeEach(() => {
  jest.clearAllMocks();
  sent = [];
  mockSignAndSend.mockImplementation(async (build, options) => {
    sent = await build({ signer: { address: AUTHOR } });
    sentOptions = options;
    return "signature";
  });
  jest.mocked(fetchGlobalConfig).mockResolvedValue({
    data: { maxDescriptionLength: 500 },
  } as never);
  jest.mocked(fetchProposal).mockResolvedValue({
    data: { author: AUTHOR, numSupporters: 10, voteAccountPubkey: VOTE_ACCOUNT },
  } as never);
  jest.mocked(assertValidProposalDocument).mockResolvedValue(undefined as never);
  voteDelegation(null);
});

describe("useCreateProposal", () => {
  const input = { description: DOCUMENT, publicKey: AUTHOR, title: "SIMD-0001" };

  it("does not ask for a signature when the proposal document fails verification", async () => {
    jest.mocked(assertValidProposalDocument).mockRejectedValue(
      new Error("The proposal document could not be fetched."),
    );

    await expect(mutation(useCreateProposal).mutateAsync(input)).rejects.toThrow(
      "could not be fetched",
    );
    expect(mockSignAndSend).not.toHaveBeenCalled();
  });

  it("creates the proposal without verifying the document when the author opts out", async () => {
    await mutation(useCreateProposal).mutateAsync({ ...input, skipDocumentCheck: true });

    expect(assertValidProposalDocument).not.toHaveBeenCalled();
    expect(sent).toEqual([
      { create: expect.objectContaining({ description: DOCUMENT, splVoteAccount: VOTE_ACCOUNT }) },
    ]);
  });
});

describe("useUpdateProposalDescription", () => {
  const update = (publicKey: Address) =>
    mutation(useUpdateProposalDescription).mutateAsync({
      description: DOCUMENT,
      proposalId: PROPOSAL,
      publicKey,
    });

  it("lets the author update a proposal whose vote account has no delegate", async () => {
    await expect(update(AUTHOR)).resolves.toEqual({ signature: "signature", success: true });
  });

  it("still lets the author update while a delegation is only pending", async () => {
    voteDelegation({ accepted: false });

    await expect(update(AUTHOR)).resolves.toEqual({ signature: "signature", success: true });
  });

  it("hands the update to the delegate once the delegation is accepted", async () => {
    // The program rejects the author's signature in this state, so the wallet
    // is told why instead of being asked to sign a transaction that will fail.
    voteDelegation({ accepted: true });

    await expect(update(AUTHOR)).rejects.toThrow("accepted vote delegate");
    await expect(update(DELEGATE)).resolves.toEqual({ signature: "signature", success: true });
    expect(mockSignAndSend).toHaveBeenCalledTimes(1);
  });

  it("refuses a wallet that is neither the author nor the delegate", async () => {
    await expect(update(DELEGATE)).rejects.toThrow("Only the proposal author");
    expect(mockSignAndSend).not.toHaveBeenCalled();
  });
});

describe("useSupportProposal", () => {
  const support = () =>
    mutation(() => useSupportProposal(AUTHOR)).mutateAsync({
      proposalId: PROPOSAL,
      publicKey: AUTHOR,
    });

  it("targets the snapshot slot the governance config schedules", async () => {
    await support();

    // Epoch 900 + 2 discussion epochs + 1 extension, then the configured offset.
    expect(sent[0]).toEqual({
      support: expect.objectContaining({
        snapshotSlot: 903n * 432_000n + 1_000n,
        splVoteAccount: VOTE_ACCOUNT,
      }),
    });
  });

  it("requests more compute as the supporter list grows", async () => {
    // The handler re-tallies every supporter; without a raised limit the
    // default 200,000 units run out once a proposal has many of them.
    const limitFor = async (numSupporters: number) => {
      jest.mocked(fetchProposal).mockResolvedValue({ data: { numSupporters } } as never);
      await support();
      return sentOptions?.computeUnitLimit ?? 0;
    };

    const few = await limitFor(10);
    expect(await limitFor(1_500)).toBeGreaterThan(few);
    expect(await limitFor(1_500)).toBeGreaterThan(200_000);
  });

  it("budgets for a full supporter list when the count cannot be read", async () => {
    jest.spyOn(console, "warn").mockImplementation(() => {});
    jest.mocked(fetchProposal).mockRejectedValue(new Error("rpc down"));

    await expect(support()).resolves.toEqual({ signature: "signature", success: true });
  });
});
