// The support_proposal handler re-tallies the whole supporter list on every
// call, so its cost is linear in `num_supporters`. Requesting a flat worst case
// would overshoot a typical call by ~10x, and priority fees price the
// *requested* limit rather than what is consumed, so the request is modelled
// instead.
//
// Keep in sync with the mirror in `svmgov/cli/src/constants.rs`. The program's
// `tests/support_compute_budget.rs` asserts the model covers measured cost.

/** Fixed cost before the per-supporter re-tally. Measured at 22,434 CU. */
const SUPPORT_CU_BASE = 22_500;
/** Per existing supporter: a `sol_get_epoch_stake` syscall plus loop overhead. */
const SUPPORT_CU_PER_SUPPORTER = 132;
/**
 * Extra units for the call that crosses the threshold: it activates voting and,
 * unless the ballot box exists, creates it via the `init_ballot_box` CPI.
 * Measured at ~26.8k above a non-activating call, at the 64-operator whitelist
 * maximum (init_ballot_box clones the whitelist, so a longer list costs more).
 * Always included — a caller cannot know whether its own call will cross.
 */
const SUPPORT_CU_ACTIVATION = 28_000;
/**
 * Allowance for the PDAs the handler derives by bump search from keys that
 * differ per proposal: the support and delegation records, the proposal signer
 * inside `init_ballot_box`, the consensus result, and the ballot box twice.
 * Each failed attempt costs 1,500 CU and the count depends on the keys; 18
 * failed attempts leaves about one proposal in 300 needing more.
 */
export const SUPPORT_CU_BUMP_SEARCH = 27_000;
/** Covers supporters landing between reading the count and executing. */
const SUPPORT_CU_HEADROOM_PERCENT = 15;
/** Per-transaction maximum a client may request. */
const MAX_COMPUTE_UNIT_LIMIT = 1_400_000;
/**
 * The program's `MAX_SUPPORTERS_LIMIT`. Used as the supporter count when the
 * real one cannot be read, so the request still covers the largest list the
 * program permits.
 */
export const MAX_SUPPORTERS = 2_000;

/**
 * Compute-unit limit to request for a support against a proposal that currently
 * has `numSupporters` supporters.
 */
export function supportComputeUnitLimit(numSupporters: number): number {
  const modelled =
    SUPPORT_CU_BASE +
    SUPPORT_CU_PER_SUPPORTER * Math.max(0, numSupporters) +
    SUPPORT_CU_ACTIVATION +
    SUPPORT_CU_BUMP_SEARCH;
  const withHeadroom = Math.ceil(
    (modelled * (100 + SUPPORT_CU_HEADROOM_PERCENT)) / 100,
  );
  return Math.min(withHeadroom, MAX_COMPUTE_UNIT_LIMIT);
}
