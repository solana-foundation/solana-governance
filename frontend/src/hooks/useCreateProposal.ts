import { createSolanaRpc } from "@solana/kit";
import { useMutation } from "@tanstack/react-query";
import { useEndpoint } from "@/contexts/EndpointContext";
import { assertValidProposalUrl } from "@/lib/github";
import { buildCreateProposalInstruction } from "@/lib/transactions";
import { useSignTransaction } from "./useSignTransaction";

export function useCreateProposal() {
  const { endpointUrl } = useEndpoint();
  const { signAndSend } = useSignTransaction();
  return useMutation({
    mutationKey: ["create-proposal"],
    mutationFn: async ({ title, description, publicKey }: { title: string; description: string; publicKey?: string }) => {
      if (!publicKey) throw new Error("Wallet not connected");
      const voteAccounts = await createSolanaRpc(endpointUrl).getVoteAccounts().send();
      const vote = [...voteAccounts.current, ...voteAccounts.delinquent].find((account) => account.nodePubkey === publicKey);
      if (!vote) throw new Error(`No SPL vote account found for validator identity ${publicKey}`);
      const normalizedDescription = assertValidProposalUrl(description);
      const signature = await signAndSend(({ signer }) => Promise.all([
        buildCreateProposalInstruction({ description: normalizedDescription, signer, splVoteAccount: vote.votePubkey, title }),
      ]));
      return { signature, success: true };
    },
  });
}
