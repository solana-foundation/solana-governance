import { address, createSolanaRpc } from "@solana/kit";
import {
  fetchGlobalConfig,
  fetchMaybeVoteDelegation,
  fetchProposal,
  findGlobalConfigPda,
  findVoteDelegationPda,
} from "@solana/svmgov";
import { useMutation } from "@tanstack/react-query";
import { useEndpoint } from "@/contexts/EndpointContext";
import { assertValidProposalDocument, assertValidProposalUrl } from "@/lib/github";
import { buildUpdateProposalDescriptionInstruction } from "@/lib/transactions";
import { useSignTransaction } from "./useSignTransaction";

/** Creates a mutation that replaces an eligible proposal's commit-pinned document URL. */
export function useUpdateProposalDescription() {
  const { endpointUrl } = useEndpoint();
  const { signAndSend } = useSignTransaction();
  return useMutation({
    mutationKey: ["update-proposal-description"],
    mutationFn: async ({ proposalId, description, publicKey, skipDocumentCheck }: {
      proposalId: string;
      description: string;
      publicKey?: string;
      /** Explicitly bypasses off-chain reachability/frontmatter verification. */
      skipDocumentCheck?: boolean;
    }) => {
      if (!publicKey) throw new Error("Wallet not connected");
      const rpc = createSolanaRpc(endpointUrl);
      const proposalAddress = address(proposalId);
      const [globalConfig] = await findGlobalConfigPda();
      const [config, proposal] = await Promise.all([
        fetchGlobalConfig(rpc, globalConfig),
        fetchProposal(rpc, proposalAddress),
      ]);
      const normalizedDescription = assertValidProposalUrl(description, config.data.maxDescriptionLength);
      if (!skipDocumentCheck) await assertValidProposalDocument(normalizedDescription);

      const [voteDelegation] = await findVoteDelegationPda({ splVoteAccount: proposal.data.voteAccountPubkey });
      const delegation = await fetchMaybeVoteDelegation(rpc, voteDelegation);
      // With an accepted delegation on the proposal's vote account only the
      // delegate may update it; otherwise only the author (the identity) may.
      if (delegation.exists && delegation.data.delegationAccepted) {
        if (delegation.data.delegate !== publicKey) {
          throw new Error("Only the vote account's accepted vote delegate can update the description");
        }
      } else if (proposal.data.author !== publicKey) {
        throw new Error("Only the proposal author can update the description");
      }

      const signature = await signAndSend(({ signer }) => Promise.all([
        buildUpdateProposalDescriptionInstruction({
          description: normalizedDescription,
          proposal: proposalAddress,
          signer,
          voteAccount: proposal.data.voteAccountPubkey,
        }),
      ]));
      return { signature, success: true };
    },
  });
}
