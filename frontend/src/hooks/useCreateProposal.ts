import { createSolanaRpc } from "@solana/kit";
import { fetchGlobalConfig, findGlobalConfigPda } from "@solana/svmgov";
import { useMutation } from "@tanstack/react-query";
import { useEndpoint } from "@/contexts/EndpointContext";
import { assertValidProposalDocument, assertValidProposalUrl } from "@/lib/github";
import { buildCreateProposalInstruction } from "@/lib/transactions";
import { useResolveSignerVoteAccount } from "./useResolveSignerVoteAccount";
import { useSignTransaction } from "./useSignTransaction";

export function useCreateProposal() {
  const { endpointUrl } = useEndpoint();
  const { signAndSend } = useSignTransaction();
  const resolveSignerVoteAccount = useResolveSignerVoteAccount();
  return useMutation({
    mutationKey: ["create-proposal"],
    mutationFn: async ({ title, description, publicKey, skipDocumentCheck }: {
      title: string;
      description: string;
      publicKey?: string;
      /** Explicitly bypasses off-chain reachability/frontmatter verification. */
      skipDocumentCheck?: boolean;
    }) => {
      if (!publicKey) throw new Error("Wallet not connected");
      const rpc = createSolanaRpc(endpointUrl);
      const [globalConfig] = await findGlobalConfigPda();
      const [config, splVoteAccount] = await Promise.all([
        fetchGlobalConfig(rpc, globalConfig),
        // The wallet is either the validator identity or its accepted vote delegate.
        resolveSignerVoteAccount(publicKey),
      ]);
      const normalizedDescription = assertValidProposalUrl(description, config.data.maxDescriptionLength);
      if (!skipDocumentCheck) await assertValidProposalDocument(normalizedDescription);
      const signature = await signAndSend(({ signer }) => Promise.all([
        buildCreateProposalInstruction({ description: normalizedDescription, signer, splVoteAccount, title }),
      ]));
      return { signature, success: true };
    },
  });
}
