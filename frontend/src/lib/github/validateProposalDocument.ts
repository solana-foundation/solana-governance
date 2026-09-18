import { fetchProposalDocument } from "./fetchProposalDocument";

export type ProposalDocumentValidationCode =
  | "not-found"
  | "missing-frontmatter"
  | "unreachable";

/** A preflight failure that the UI may explicitly allow a user to bypass. */
export class ProposalDocumentValidationError extends Error {
  readonly name = "ProposalDocumentValidationError";

  constructor(
    readonly code: ProposalDocumentValidationCode,
    message: string,
  ) {
    super(message);
  }
}

/**
 * Verifies the off-chain properties that cannot be enforced by the program.
 * Callers must perform URL structure validation before invoking this helper.
 */
export async function assertValidProposalDocument(url: string): Promise<void> {
  try {
    const result = await fetchProposalDocument(url);
    if (result.status === "unsupported") {
      throw new ProposalDocumentValidationError(
        "not-found",
        `Proposal document could not be fetched: ${result.reason}`,
      );
    }
    if (!result.document.hasFrontmatter) {
      throw new ProposalDocumentValidationError(
        "missing-frontmatter",
        "Proposal document must begin with a non-empty frontmatter block delimited by ---.",
      );
    }
  } catch (error) {
    if (error instanceof ProposalDocumentValidationError) throw error;
    throw new ProposalDocumentValidationError(
      "unreachable",
      "Could not verify the proposal document. Check GitHub connectivity or submit without document verification.",
    );
  }
}
