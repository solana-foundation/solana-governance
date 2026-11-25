import {
  SOLANA_ERROR__RPC__TRANSPORT_HTTP_ERROR,
  SolanaError,
  type Address,
} from "@solana/kit";
import { fetchMaybeMetaMerkleProof } from "@solana/ncn-snapshot";
import { fetchProposal } from "@solana/svmgov";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook } from "@testing-library/react";
import { track } from "@vercel/analytics";
import type { ReactNode } from "react";
import { getStakeAccountProof, getVoteAccountProof } from "@/lib/ncnProofs";
import { NcnApiHttpError, isNcnProofNotFound } from "@/lib/ncnApi";
import {
  useCastVoteOverrideTransaction,
  useCastVoteTransaction,
  useModifyVoteTransaction,
} from "../useVoteTransaction";

const WALLET = "SysvarC1ock11111111111111111111111111111111" as Address;
const PROPOSAL = "SysvarRent111111111111111111111111111111111" as Address;
const CONSENSUS = "Config1111111111111111111111111111111111111" as Address;
const VOTE_ACCOUNT = "Vote111111111111111111111111111111111111111" as Address;
const STAKE_ACCOUNT = "Stake11111111111111111111111111111111111111" as Address;

const mockSignAndSend = jest.fn();

jest.mock("../useSignTransaction", () => ({
  useSignTransaction: () => ({ signAndSend: mockSignAndSend }),
}));
jest.mock("../useResolveSignerVoteAccount", () => ({
  useResolveSignerVoteAccount: () => () => Promise.resolve(VOTE_ACCOUNT),
}));
jest.mock("@/contexts/EndpointContext", () => ({
  useEndpoint: () => ({ endpointUrl: "https://rpc.example", network: "testnet" }),
}));
jest.mock("@/contexts/NcnApiContext", () => ({
  useNcnApi: () => ({ ncnApiUrl: "https://ncn.example" }),
}));
jest.mock("@vercel/analytics", () => ({ track: jest.fn() }));
jest.mock("@solana/svmgov", () => ({ fetchProposal: jest.fn() }));
jest.mock("@solana/ncn-snapshot", () => ({
  fetchMaybeMetaMerkleProof: jest.fn(),
  findMetaMerkleProofPda: () => Promise.resolve(["MetaProofPda"]),
}));
jest.mock("@/lib/ncnProofs", () => ({
  ...jest.requireActual("@/lib/ncnProofs"),
  assertOverrideProofLineage: jest.fn(),
  getStakeAccountProof: jest.fn(),
  getVoteAccountProof: jest.fn(),
}));
jest.mock("@/lib/metaMerkleProofSetup", () => ({
  ...jest.requireActual("@/lib/metaMerkleProofSetup"),
  computeProofCloseTimestamp: () => Promise.resolve(1_800_000_000n),
}));
// Each builder returns its own name, so a sent transaction reads as the list
// of instructions it carried.
jest.mock("@/lib/transactions", () => ({
  buildCastVoteInstruction: () => Promise.resolve("castVote"),
  buildCastVoteOverrideInstruction: () => Promise.resolve("castVoteOverride"),
  buildInitializeMetaMerkleProofInstruction: () => Promise.resolve("initMetaMerkleProof"),
  buildModifyVoteInstruction: () => Promise.resolve("modifyVote"),
  buildModifyVoteOverrideInstruction: () => Promise.resolve("modifyVoteOverride"),
}));

const VOTE = {
  abstainVotesBp: 0,
  againstVotesBp: 0,
  consensusResult: CONSENSUS,
  forVotesBp: 10_000,
  proposalId: PROPOSAL,
  publicKey: WALLET,
};

/** The instructions of every transaction the hook asked the wallet to sign, in order. */
let sentTransactions: unknown[][];
/** Whether the shared meta merkle proof account exists on chain. */
let proofExists: boolean;

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>;
}
const mutation = <T,>(hook: () => T) => renderHook(hook, { wrapper }).result.current;

beforeEach(() => {
  jest.clearAllMocks();
  sentTransactions = [];
  proofExists = false;
  mockSignAndSend.mockImplementation(async (build) => {
    sentTransactions.push(await build({ signer: { address: WALLET } }));
    return `signature-${sentTransactions.length}`;
  });
  jest.mocked(fetchProposal).mockResolvedValue({
    data: { endEpoch: 905n, snapshotSlot: 1_000n },
  } as unknown as Awaited<ReturnType<typeof fetchProposal>>);
  jest.mocked(fetchMaybeMetaMerkleProof).mockImplementation(
    () => Promise.resolve({ exists: proofExists }) as ReturnType<typeof fetchMaybeMetaMerkleProof>,
  );
  const metaProof = { meta_merkle_leaf: { vote_account: VOTE_ACCOUNT } };
  jest.mocked(getVoteAccountProof).mockResolvedValue(metaProof as never);
  jest.mocked(getStakeAccountProof).mockResolvedValue({
    stake_merkle_leaf: { active_stake: "5000000000", stake_account: STAKE_ACCOUNT, voting_wallet: WALLET },
    stake_merkle_proof: [],
    vote_account: VOTE_ACCOUNT,
  } as never);
});

describe("validator votes", () => {
  it("creates the missing proof account in the same transaction as the vote", async () => {
    await mutation(useCastVoteTransaction).mutateAsync(VOTE);

    expect(sentTransactions).toEqual([["initMetaMerkleProof", "castVote"]]);
  });

  it("sends only the vote when the proof account already exists", async () => {
    proofExists = true;

    await mutation(useModifyVoteTransaction).mutateAsync(VOTE);

    expect(sentTransactions).toEqual([["modifyVote"]]);
  });

  it("reports init, then success, to analytics", async () => {
    await mutation(useCastVoteTransaction).mutateAsync(VOTE);

    expect(jest.mocked(track).mock.calls).toEqual([
      ["Cast Vote init", { proposalId: PROPOSAL }],
      ["Cast Vote success", { proposalId: PROPOSAL }],
    ]);
  });

  it("explains an RPC failure that happens before anything is signed", async () => {
    // Kit's production message for this is only "Solana error #8100002; Decode
    // this error by running `npx @solana/errors decode ...`", which the vote
    // modal displayed verbatim.
    jest.mocked(fetchProposal).mockRejectedValue(
      new SolanaError(SOLANA_ERROR__RPC__TRANSPORT_HTTP_ERROR, {
        headers: new Headers(),
        message: "Too Many Requests",
        statusCode: 429,
      }),
    );

    await expect(mutation(useCastVoteTransaction).mutateAsync(VOTE)).rejects.toThrow(
      "The RPC is rate limiting requests",
    );
    expect(mockSignAndSend).not.toHaveBeenCalled();
    expect(track).toHaveBeenLastCalledWith("Cast Vote error", expect.objectContaining({ proposalId: PROPOSAL }));
  });

  it("passes a missing snapshot proof through so the modal can explain it", async () => {
    const missing = new NcnApiHttpError("vote account proof", 404, {
      resource: "vote-account-proof",
      url: "https://ncn.example/proof",
    });
    jest.mocked(getVoteAccountProof).mockRejectedValue(missing);

    const error = await mutation(useCastVoteTransaction).mutateAsync(VOTE).catch((e: unknown) => e);

    expect(error).toBe(missing);
    expect(isNcnProofNotFound(error)).toBe(true);
  });
});

describe("stake override votes", () => {
  const OVERRIDE = { ...VOTE, stakeAccount: STAKE_ACCOUNT };

  it("creates the shared proof account in its own transaction before voting", async () => {
    // The meta proof and the stake proof together exceed the transaction size
    // limit, so they cannot share one.
    mockSignAndSend.mockImplementation(async (build) => {
      sentTransactions.push(await build({ signer: { address: WALLET } }));
      proofExists = true;
      return "signature";
    });

    await mutation(useCastVoteOverrideTransaction).mutateAsync(OVERRIDE);

    expect(sentTransactions).toEqual([["initMetaMerkleProof"], ["castVoteOverride"]]);
  });

  it("asks for one signature when the proof account already exists", async () => {
    // An existing proof used to produce an empty setup transaction that the
    // delegator still signed and paid a fee for.
    proofExists = true;

    await mutation(useCastVoteOverrideTransaction).mutateAsync(OVERRIDE);

    expect(sentTransactions).toEqual([["castVoteOverride"]]);
  });

  it("still votes when another delegator creates the proof account first", async () => {
    mockSignAndSend
      .mockImplementationOnce(() => {
        proofExists = true;
        return Promise.reject(new Error("Transaction failed: account already in use"));
      })
      .mockImplementationOnce(async (build) => {
        sentTransactions.push(await build({ signer: { address: WALLET } }));
        return "vote-signature";
      });

    await expect(
      mutation(useCastVoteOverrideTransaction).mutateAsync(OVERRIDE),
    ).resolves.toEqual({ signature: "vote-signature", success: true });
    expect(sentTransactions).toEqual([["castVoteOverride"]]);
  });

  it("does not ask for the vote signature after the proof setup is rejected", async () => {
    const rejection = new Error("User rejected the request.");
    mockSignAndSend.mockRejectedValue(rejection);

    await expect(
      mutation(useCastVoteOverrideTransaction).mutateAsync(OVERRIDE),
    ).rejects.toBe(rejection);
    expect(mockSignAndSend).toHaveBeenCalledTimes(1);
  });
});
