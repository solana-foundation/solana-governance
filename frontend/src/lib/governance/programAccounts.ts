import {
  getAddressEncoder,
  getBase64Decoder,
  parseBase64RpcAccount,
  type Account,
  type Address,
  type Base64EncodedBytes,
  type GetProgramAccountsApi,
  type ReadonlyUint8Array,
  type Rpc,
} from "@solana/kit";
import {
  decodeProposal,
  decodeSupport,
  decodeVote,
  decodeVoteOverride,
  PROPOSAL_DISCRIMINATOR,
  SUPPORT_DISCRIMINATOR,
  SVMGOV_PROGRAM_PROGRAM_ADDRESS,
  VOTE_DISCRIMINATOR,
  VOTE_OVERRIDE_DISCRIMINATOR,
  type Proposal,
  type Support,
  type Vote,
  type VoteOverride,
} from "@solana/svmgov";

const bytes = (value: ReadonlyUint8Array): Base64EncodedBytes =>
  getBase64Decoder().decode(value) as Base64EncodedBytes;
const memcmp = (offset: bigint, value: ReadonlyUint8Array) => ({
  memcmp: { bytes: bytes(value), encoding: "base64" as const, offset },
});
const addressFilter = (offset: bigint, value: Address) =>
  memcmp(offset, getAddressEncoder().encode(value));

export async function fetchProposals(rpc: Rpc<GetProgramAccountsApi>): Promise<Account<Proposal>[]> {
  const result = await rpc.getProgramAccounts(SVMGOV_PROGRAM_PROGRAM_ADDRESS, {
    encoding: "base64",
    filters: [memcmp(0n, PROPOSAL_DISCRIMINATOR)],
  }).send();
  return result.map(({ account, pubkey }) => decodeProposal(parseBase64RpcAccount(pubkey, account)));
}

export async function fetchSupports(rpc: Rpc<GetProgramAccountsApi>, filters: { proposal?: Address; validator?: Address }): Promise<Account<Support>[]> {
  const result = await rpc.getProgramAccounts(SVMGOV_PROGRAM_PROGRAM_ADDRESS, {
    encoding: "base64",
    filters: [memcmp(0n, SUPPORT_DISCRIMINATOR), ...(filters.proposal ? [addressFilter(8n, filters.proposal)] : []), ...(filters.validator ? [addressFilter(40n, filters.validator)] : [])],
  }).send();
  return result.map(({ account, pubkey }) => decodeSupport(parseBase64RpcAccount(pubkey, account)));
}

export async function fetchVotes(rpc: Rpc<GetProgramAccountsApi>, filters: { proposal?: Address; validator?: Address }): Promise<Account<Vote>[]> {
  const result = await rpc.getProgramAccounts(SVMGOV_PROGRAM_PROGRAM_ADDRESS, {
    encoding: "base64",
    filters: [memcmp(0n, VOTE_DISCRIMINATOR), ...(filters.validator ? [addressFilter(8n, filters.validator)] : []), ...(filters.proposal ? [addressFilter(40n, filters.proposal)] : [])],
  }).send();
  return result.map(({ account, pubkey }) => decodeVote(parseBase64RpcAccount(pubkey, account)));
}

export async function fetchVoteOverrides(rpc: Rpc<GetProgramAccountsApi>, filters: { delegator?: Address; stakeAccount?: Address; validator?: Address; proposal?: Address }): Promise<Account<VoteOverride>[]> {
  const result = await rpc.getProgramAccounts(SVMGOV_PROGRAM_PROGRAM_ADDRESS, {
    encoding: "base64",
    filters: [memcmp(0n, VOTE_OVERRIDE_DISCRIMINATOR), ...(filters.delegator ? [addressFilter(8n, filters.delegator)] : []), ...(filters.stakeAccount ? [addressFilter(40n, filters.stakeAccount)] : []), ...(filters.validator ? [addressFilter(72n, filters.validator)] : []), ...(filters.proposal ? [addressFilter(104n, filters.proposal)] : [])],
  }).send();
  return result.map(({ account, pubkey }) => decodeVoteOverride(parseBase64RpcAccount(pubkey, account)));
}

export type { Proposal, Support, Vote, VoteOverride };
