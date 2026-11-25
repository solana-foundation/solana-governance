import {
  getSolanaErrorFromJsonRpcError,
  getSolanaErrorFromTransactionError,
  SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED,
  SolanaError,
} from "@solana/kit";
import {
  describeTransactionError,
  rethrowAsTransactionFailure,
  TransactionFailedError,
} from "../transactionErrors";

/** The error Kit throws when `sendTransaction` fails preflight simulation. */
function preflightFailure(logs: string[], err: unknown) {
  return getSolanaErrorFromJsonRpcError({
    code: -32002,
    data: { accounts: null, err, logs, unitsConsumed: 0 },
    message: "Transaction simulation failed",
  });
}

// Kit's production bundle replaces every error message with "Solana error
// #<code>; Decode this error by running `npx @solana/errors decode ...`", which
// the vote and support modals used to show verbatim.
describe("describeTransactionError", () => {
  it("reports the Anchor error message from the simulation logs", () => {
    const error = preflightFailure(
      [
        "Program govYkyQ3ePtGULAtY6V75qjWE8UH4vCUVQ1W4HdCAZU invoke [1]",
        "Program log: Instruction: CastVote",
        "Program log: AnchorError thrown in programs/svmgov/src/instructions/cast_vote.rs:42. Error Code: VotingNotStarted. Error Number: 6007. Error Message: Voting on proposal not yet started.",
        "Program govYkyQ3ePtGULAtY6V75qjWE8UH4vCUVQ1W4HdCAZU failed: custom program error: 0x1777",
      ],
      { InstructionError: [0, { Custom: 6007 }] },
    );

    expect(describeTransactionError(error)).toBe(
      "Transaction failed: Voting on proposal not yet started",
    );
  });

  it("reports the runtime's reason when no program logged an error", () => {
    const error = preflightFailure(
      [
        "Program govYkyQ3ePtGULAtY6V75qjWE8UH4vCUVQ1W4HdCAZU invoke [1]",
        "Program govYkyQ3ePtGULAtY6V75qjWE8UH4vCUVQ1W4HdCAZU failed: exceeded CUs meter at BPF instruction",
      ],
      { InstructionError: [0, "ComputationalBudgetExceeded"] },
    );

    expect(describeTransactionError(error)).toBe(
      "Transaction failed: exceeded CUs meter at BPF instruction",
    );
  });

  it("names the program error code when a confirmed transaction failed without logs", () => {
    const error = getSolanaErrorFromTransactionError({
      InstructionError: [0, { Custom: 6007 }],
    });

    expect(describeTransactionError(error)).toBe(
      "Transaction failed with program error 6007 (0x1777)",
    );
  });

  it("explains an unfunded fee payer", () => {
    const error = preflightFailure([], "AccountNotFound");

    expect(describeTransactionError(error)).toMatch(/enough SOL/);
  });

  it("says an expired transaction was not processed", () => {
    const error = new SolanaError(SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED, {
      currentBlockHeight: 2n,
      lastValidBlockHeight: 1n,
    });

    expect(describeTransactionError(error)).toMatch(/expired.*not processed/);
  });

  it("leaves wallet errors to speak for themselves", () => {
    expect(describeTransactionError(new Error("User rejected the request."))).toBeUndefined();
  });
});

describe("rethrowAsTransactionFailure", () => {
  it("keeps the Solana error as the cause of the readable one", () => {
    const solanaError = getSolanaErrorFromTransactionError({
      InstructionError: [0, { Custom: 6000 }],
    });

    let thrown: unknown;
    try {
      rethrowAsTransactionFailure(solanaError);
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(TransactionFailedError);
    expect((thrown as Error).cause).toBe(solanaError);
  });

  it("rethrows a wallet rejection untouched so it is still recognised as a cancellation", () => {
    const rejection = new Error("User rejected the request.");

    expect(() => rethrowAsTransactionFailure(rejection)).toThrow(rejection);
  });
});
