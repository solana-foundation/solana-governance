import {
  isSolanaError,
  SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED,
  SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM,
  SOLANA_ERROR__TRANSACTION_ERROR__ACCOUNT_NOT_FOUND,
  SOLANA_ERROR__TRANSACTION_ERROR__BLOCKHASH_NOT_FOUND,
  SOLANA_ERROR__TRANSACTION_ERROR__INSUFFICIENT_FUNDS_FOR_FEE,
  type SolanaError,
} from "@solana/kit";

const MAX_CAUSE_DEPTH = 5;

/**
 * A failed transaction, with a `message` fit to show a user. The underlying
 * `SolanaError` is kept as `cause`.
 */
export class TransactionFailedError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "TransactionFailedError";
  }
}

function solanaErrorChain(error: unknown): SolanaError[] {
  const chain: SolanaError[] = [];
  let current = error;
  for (let depth = 0; depth < MAX_CAUSE_DEPTH && current; depth += 1) {
    if (isSolanaError(current)) chain.push(current);
    current = current instanceof Error ? current.cause : undefined;
  }
  return chain;
}

function simulationLogs(chain: SolanaError[]): string[] {
  for (const error of chain) {
    const logs = (error.context as { logs?: unknown }).logs;
    if (Array.isArray(logs)) {
      return logs.filter((line): line is string => typeof line === "string");
    }
  }
  return [];
}

/**
 * Picks the line of a simulation log that says why the transaction failed:
 * the Anchor error message, else a program's own `Error:` log, else the
 * runtime's `failed:` reason.
 */
function failureReasonFromLogs(logs: string[]): string | undefined {
  const patterns = [
    /Error Message: (.+?)\.?$/,
    /^Program log: Error: (.+)$/,
    /^Program \S+ failed: (.+)$/,
  ];
  for (const pattern of patterns) {
    for (let index = logs.length - 1; index >= 0; index -= 1) {
      const match = pattern.exec(logs[index]);
      if (match) return match[1];
    }
  }
  return undefined;
}

/**
 * Describes why a transaction failed in words a user can act on. Kit strips
 * its error messages from production bundles, leaving only a numeric code, so
 * the reason is taken from the simulation logs and the error's code instead.
 *
 * Returns `undefined` when `error` is not a Solana error, such as a wallet
 * rejection, whose own message should be shown.
 */
export function describeTransactionError(error: unknown): string | undefined {
  const chain = solanaErrorChain(error);
  if (chain.length === 0) return undefined;

  const reason = failureReasonFromLogs(simulationLogs(chain));
  if (reason) return `Transaction failed: ${reason}`;

  for (const solanaError of chain) {
    if (isSolanaError(solanaError, SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM)) {
      const { code } = solanaError.context;
      return `Transaction failed with program error ${code} (0x${code.toString(16)})`;
    }
    if (
      isSolanaError(solanaError, SOLANA_ERROR__TRANSACTION_ERROR__ACCOUNT_NOT_FOUND) ||
      isSolanaError(solanaError, SOLANA_ERROR__TRANSACTION_ERROR__INSUFFICIENT_FUNDS_FOR_FEE)
    ) {
      return "The wallet does not hold enough SOL on this network to pay the transaction fee";
    }
    if (
      isSolanaError(solanaError, SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED) ||
      isSolanaError(solanaError, SOLANA_ERROR__TRANSACTION_ERROR__BLOCKHASH_NOT_FOUND)
    ) {
      return "The transaction expired before it was confirmed and was not processed. Please try again.";
    }
  }

  const [outermost] = chain;
  // Production builds of Kit reduce every message to a decode command.
  return outermost.message.startsWith("Solana error #")
    ? `Transaction failed (Solana error #${outermost.context.__code})`
    : outermost.message;
}

/**
 * Rethrows a transaction failure as a `TransactionFailedError` carrying a
 * readable message. Errors that are not Solana errors are rethrown unchanged.
 */
export function rethrowAsTransactionFailure(error: unknown): never {
  const message = describeTransactionError(error);
  if (message === undefined) throw error;
  throw new TransactionFailedError(message, { cause: error });
}
