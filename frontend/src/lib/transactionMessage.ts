import {
  appendTransactionMessageInstructions,
  createTransactionMessage,
  estimateAndSetResourceLimitsFactory,
  estimateResourceLimitsFactory,
  fillTransactionMessageProvisoryResourceLimits,
  pipe,
  setTransactionMessageComputeUnitLimit,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  type Blockhash,
  type Instruction,
  type Rpc,
  type SimulateTransactionApi,
  type TransactionSigner,
} from "@solana/kit";

/** Largest compute-unit limit a transaction may request. */
const MAX_COMPUTE_UNIT_LIMIT = 1_400_000;
/** Largest loaded-accounts data size a transaction may request (64 MiB). */
const MAX_LOADED_ACCOUNTS_DATA_SIZE_LIMIT = 64 * 1024 * 1024;
/**
 * The block cost model charges loaded account data in pages of this size, so
 * headroom up to the next page boundary costs nothing.
 */
const LOADED_ACCOUNTS_DATA_PAGE = 32 * 1024;
/**
 * Margin over the simulated compute units, covering state that changes between
 * simulation and execution. Keep in sync with `svmgov/cli/src/utils/transaction.rs`.
 */
const COMPUTE_UNIT_HEADROOM_PERCENT = 10;

type ResourceLimitsEstimator = Parameters<typeof estimateAndSetResourceLimitsFactory>[0];

/**
 * Wraps `estimate` so its limits leave room for what can change before the
 * transaction executes: compute units get headroom, and loaded data is rounded
 * up past the next page boundary, since an account created after simulation
 * adds to it.
 */
export function withResourceLimitHeadroom(estimate: ResourceLimitsEstimator): ResourceLimitsEstimator {
  return (async (message, config) => {
    const { computeUnitLimit, loadedAccountsDataSizeLimit } = await estimate(message, config);
    return {
      computeUnitLimit: Math.min(
        Math.ceil((computeUnitLimit * (100 + COMPUTE_UNIT_HEADROOM_PERCENT)) / 100),
        MAX_COMPUTE_UNIT_LIMIT,
      ),
      loadedAccountsDataSizeLimit:
        loadedAccountsDataSizeLimit === undefined
          ? undefined
          : Math.min(
              (Math.floor(loadedAccountsDataSizeLimit / LOADED_ACCOUNTS_DATA_PAGE) + 1) *
                LOADED_ACCOUNTS_DATA_PAGE,
              MAX_LOADED_ACCOUNTS_DATA_SIZE_LIMIT,
            ),
    };
  }) as ResourceLimitsEstimator;
}

/**
 * Builds a transaction message of `version` that runs `instructions`, paid for
 * by `feePayer`.
 *
 * A v1 message carries its resource limits in its config, where an unset limit
 * budgets zero, so they are sized by simulating it against `rpc`.
 * `computeUnitLimit`, when given, replaces the simulated compute-unit limit; on
 * v0 it is the only limit set, as a Compute Budget instruction.
 */
export async function buildTransactionMessage({
  computeUnitLimit,
  feePayer,
  instructions,
  latestBlockhash,
  rpc,
  version,
}: {
  computeUnitLimit?: number;
  feePayer: TransactionSigner;
  instructions: readonly Instruction[];
  latestBlockhash: Readonly<{ blockhash: Blockhash; lastValidBlockHeight: bigint }>;
  rpc: Rpc<SimulateTransactionApi>;
  version: 0 | 1;
}) {
  if (version === 0) {
    const message = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(feePayer, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
      (m) => appendTransactionMessageInstructions([...instructions], m),
    );
    return computeUnitLimit === undefined
      ? message
      : setTransactionMessageComputeUnitLimit(computeUnitLimit, message);
  }
  const message = pipe(
    createTransactionMessage({ version: 1 }),
    (m) => setTransactionMessageFeePayerSigner(feePayer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    (m) => appendTransactionMessageInstructions([...instructions], m),
    (m) => setTransactionMessageComputeUnitLimit(computeUnitLimit, m),
    // Reserves the config fields so the message simulates at its final size.
    (m) => fillTransactionMessageProvisoryResourceLimits(m),
  );
  return estimateAndSetResourceLimitsFactory(
    withResourceLimitHeadroom(estimateResourceLimitsFactory({ rpc })),
  )(message);
}
