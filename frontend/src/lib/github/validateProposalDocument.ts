import { fetchProposalDocument } from "./fetchProposalDocument";

const REQUEST_TIMEOUT_MS = 5_000;

export type ProposalDocumentValidationCode =
  "not-found" | "missing-frontmatter" | "timeout" | "unreachable";

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
 * Fetching and reading the document must complete within five seconds.
 * Callers must perform URL structure validation before invoking this helper.
 * Rejects with ProposalDocumentValidationError if the document cannot be verified.
 */
export async function assertValidProposalDocument(url: string): Promise<void> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

  try {
    const result = await fetchProposalDocument(url, {
      signal: controller.signal,
    });
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
    if (controller.signal.aborted) {
      throw new ProposalDocumentValidationError(
        "timeout",
        "Proposal document verification timed out. Check GitHub connectivity or submit without document verification.",
      );
    }
    throw new ProposalDocumentValidationError(
      "unreachable",
      "Could not verify the proposal document. Check GitHub connectivity or submit without document verification.",
    );
  } finally {
    clearTimeout(timeout);
  }
}
