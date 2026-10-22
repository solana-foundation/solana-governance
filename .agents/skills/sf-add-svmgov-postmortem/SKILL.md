---
name: sf-add-svmgov-postmortem
description: Add a completed SVMGov proposal vote postmortem to this repository using the svmgov CLI, finalized Solana mainnet program accounts, and exact stake-weighted accounting. Use when the user asks to "add a proposal postmortem", "write an SVMGov vote postmortem", "analyze a completed governance proposal", or extend `vote-postmortem/` with another proposal.
metadata:
  author: Solana Foundation
  version: 1.1.0
---

# Add an SVMGov vote postmortem

Create one Markdown postmortem in `vote-postmortem/` from finalized mainnet data. Keep the workflow read-only: do not sign or send transactions.

Requires the `svmgov` CLI with the `vote-breakdown` command and Solana mainnet RPC access.

## Inputs and defaults

- Require a proposal public key.
- Query Solana mainnet with `finalized` commitment.
- Prefix `svmgov` commands with `NO_DNA=1`.
- Use the proposal title for the filename slug. Prefix it with an SGP number only when the title or user supplies one; do not infer an SGP number from the proposal account index.
- Preserve the exact status reported by `svmgov`. Treat `Ended (awaiting finalization)` as completed voting. If voting is still active, report that state and ask whether the user wants a point-in-time report instead of labeling it a postmortem.

## Workflow

1. Fetch the exact vote breakdown:

   ```bash
   NO_DNA=1 svmgov --rpc-url <MAINNET_RPC_URL> vote-breakdown <PROPOSAL_ID>
   ```

   For machine-readable data, append `--json`. Lamport amounts in JSON are decimal strings; preserve them as integers when processing the result.

   The command derives account sizes and proposal filters from its bundled IDL, decodes accounts with Anchor-generated types, and queries sequentially at finalized commitment. It reports validator and staker category amounts, original validator stake, overridden stake, account counts, rounding differences, proposal status, and query time. It checks category totals and account counts against the proposal before printing a report. Unsupported account layouts and failed reconciliations return an error; investigate rather than publishing inconsistent totals. If the command is unavailable, build the current CLI from `svmgov/cli/`.

2. Record the title, exact status, query date, vote breakdown, validator vote overridden, and account reconciliation from the output. Check that voting has ended before writing a postmortem. A finalized RPC commitment describes the data's confirmation level; it does not mean the proposal's voting period has ended.

3. Preserve all nine decimal places in SOL amounts and four decimal places in percentages. Each source row uses its own category sum as the denominator. Rows with no votes show `0.0000%` for every category.

4. Run the validation gates below. The command performs category and account-count reconciliation; review the report's status, formatting, and rounding differences before publishing.

5. Add `vote-postmortem/<proposal-slug>.md` using the output format below. Do not modify existing postmortems unless the user asks for refreshed data or format changes.

## Accounting methodology

The proposal account stores combined totals. A validator `Vote` stores only the validator's effective category amounts after stake overrides. Each `VoteOverride` stores the staker's replacement category amounts.

For total, validator, and staker-override rows, calculate each category percentage against the sum of the for, against, and abstain amounts in that same row. This makes every row an independent vote-share breakdown.

For every category:

```text
proposal total = effective validator amount + staker override amount
```

When a staker overrides a validator that has voted, the program subtracts the validator's prior vote, adds the staker vote, and re-adds the validator vote at reduced stake. `Vote.override_lamports` records stake removed from validators that cast a vote.

The validator vote overridden percentage therefore includes only overridden stake attached to validators that cast a vote. A staker override targeting a validator that never voted remains part of the staker totals, but there is no cast validator vote to include in the overridden-validator numerator.

Integer basis-point distribution can leave a few lamports between raw stake and the sum of category amounts. Record the difference when validating, but do not force category amounts to equal raw stake. The existing reports observed differences of 1–27 lamports, equal to 0.000000001–0.000000027 SOL.

## Output format

```markdown
# <TITLE> — Vote Postmortem

- Proposal: `<PROPOSAL_ID>`
- Network: Solana mainnet
- Data commitment: Finalized
- Queried: <MONTH DAY, YEAR>
- Status: <SVMGOV_STATUS>

## Vote breakdown

Amounts are in SOL. Percentages show the for/against/abstain share within each source row. See the shared [methodology](../.agents/skills/sf-add-svmgov-postmortem/SKILL.md#accounting-methodology).

| Source | For (SOL) | Against (SOL) | Abstain (SOL) |
| --- | ---: | ---: | ---: |
| Total | <TOTAL_FOR> (<PERCENT>%) | <TOTAL_AGAINST> (<PERCENT>%) | <TOTAL_ABSTAIN> (<PERCENT>%) |
| Validators | <VALIDATOR_FOR> (<PERCENT>%) | <VALIDATOR_AGAINST> (<PERCENT>%) | <VALIDATOR_ABSTAIN> (<PERCENT>%) |
| Staker overrides | <STAKER_FOR> (<PERCENT>%) | <STAKER_AGAINST> (<PERCENT>%) | <STAKER_ABSTAIN> (<PERCENT>%) |

Validator vote overridden: **<OVERRIDDEN_SOL> SOL / <ORIGINAL_VALIDATOR_STAKE_SOL> SOL = <PERCENT>%**

Account reconciliation: <VALIDATOR_COUNT> validator votes + <STAKER_COUNT> staker overrides = <TOTAL_COUNT> votes.
```

## Validation gates

- Confirm each proposal total exactly equals validator plus staker override lamports for the same category.
- Confirm validator account count plus staker override account count equals the proposal vote count.
- Confirm the Total, Validators, and Staker overrides rows each have three displayed percentages that sum to 100% within four-decimal rounding tolerance, or all zero when the row has no votes.
- Confirm every SOL value round-trips to the original lamport integer.
- Confirm the proposal status and query date are explicit.
- Run `git diff --check` on the new postmortem.

## Existing examples

- [SGP-0001](../../../vote-postmortem/sgp-0001-solana-constitution.md)
- [SGP-0002](../../../vote-postmortem/sgp-0002-double-disinflation.md)
- [SGP-0003](../../../vote-postmortem/sgp-0003-resource-and-inclusion-fee.md)

## Sources

- [`Vote` account](../../../svmgov/program/programs/svmgov_program/src/state/vote.rs)
- [`VoteOverride` account](../../../svmgov/program/programs/svmgov_program/src/state/vote_override.rs)
- [`cast_vote_override` accounting](../../../svmgov/program/programs/svmgov_program/src/instructions/cast_vote_override.rs)
- [Solana `getProgramAccounts` RPC documentation](https://solana.com/docs/rpc/http/getprogramaccounts)
