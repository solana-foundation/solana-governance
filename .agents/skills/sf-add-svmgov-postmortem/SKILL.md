---
name: sf-add-svmgov-postmortem
description: Add a completed SVMGov proposal vote postmortem to this repository using the svmgov CLI, finalized Solana mainnet program accounts, and exact stake-weighted accounting. Use when the user asks to "add a proposal postmortem", "write an SVMGov vote postmortem", "analyze a completed governance proposal", or extend `vote-postmortem/` with another proposal.
metadata:
  author: Solana Foundation
  version: 1.0.0
---

# Add an SVMGov vote postmortem

Create one Markdown postmortem in `vote-postmortem/` from finalized mainnet data. Keep the workflow read-only: do not sign or send transactions.

Requires the `svmgov` CLI, Solana mainnet RPC access, and the Solana MCP server.

## Inputs and defaults

- Require a proposal public key.
- Query Solana mainnet with `finalized` commitment.
- Prefix `svmgov` commands with `NO_DNA=1`.
- Use the proposal title for the filename slug. Prefix it with an SGP number only when the title or user supplies one; do not infer an SGP number from the proposal account index.
- Preserve the exact status reported by `svmgov`. Treat `Ended (awaiting finalization)` as completed voting. If voting is still active, report that state and ask whether the user wants a point-in-time report instead of labeling it a postmortem.

## Workflow

1. Inspect the current SVMGov IDL and account structs before querying:
   - `svmgov/cli/idls/svmgov_program.json`
   - `svmgov/program/programs/svmgov_program/src/state/vote.rs`
   - `svmgov/program/programs/svmgov_program/src/state/vote_override.rs`
   - `svmgov/program/programs/svmgov_program/src/instructions/cast_vote_override.rs`

   If their layouts differ from the account layout reference below, derive the new offsets, use those values for the query, and update this skill. Do not decode live data with stale offsets.

2. Use the Solana MCP for current RPC documentation. Call `list_sections` first, then use a narrow documentation search when `getProgramAccounts`, filter, data-slice, or commitment semantics need confirmation.

3. Fetch the proposal summary:

   ```bash
   NO_DNA=1 svmgov --rpc-url <MAINNET_RPC_URL> proposal <PROPOSAL_ID>
   ```

   Record the title, status, total for/against/abstain lamports, vote count, and query date.

4. Fetch the proposal's `Vote` and `VoteOverride` accounts with finalized `getProgramAccounts` calls. Apply the program owner, discriminator, data-size, and proposal filters in the account layout reference. Use compact data slices to reduce public RPC load. Retry HTTP 429 responses with bounded backoff and issue sequential queries rather than a large batch.

5. Decode little-endian `u64` fields and aggregate with integer arithmetic:
   - Validator for, against, abstain, original stake, and overridden stake.
   - Staker-override for, against, abstain, and stake.
   - Validator and staker account counts.

6. Calculate category percentages within each source row:
   - Total category percentage = category proposal lamports / total proposal vote lamports.
   - Validator category percentage = category validator lamports / total effective validator vote lamports.
   - Staker category percentage = category override lamports / total staker override vote lamports.
   - Validator vote overridden = total `Vote.override_lamports` / total `Vote.stake`.

   Use integer or arbitrary-precision decimal arithmetic. Round displayed percentages to four decimal places; do not convert raw lamport totals to an IEEE-754 number before calculating.

7. Convert lamports to SOL without floating-point arithmetic. Divide by `1_000_000_000`, zero-pad the remainder to nine digits, and retain all nine decimal places. This preserves the exact atomic value.

8. Run the validation gates below. Stop and investigate any failed reconciliation instead of publishing inconsistent totals.

9. Add `vote-postmortem/<proposal-slug>.md` using the output format below. Do not modify existing postmortems unless the user asks for refreshed data or format changes.

## Current account layout reference

Program ID: `govYkyQ3ePtGULAtY6V75qjWE8UH4vCUVQ1W4HdCAZU`

### Validator `Vote`

- Account size: 145 bytes
- Discriminator filter: offset 0, base58 `H7nUxx34RXx`
- Proposal filter: offset 40, proposal public key
- Compact data slice: offset 96, length 40

| Slice offset | Field |
| ---: | --- |
| 0 | For vote lamports |
| 8 | Against vote lamports |
| 16 | Abstain vote lamports |
| 24 | Original validator stake lamports |
| 32 | Overridden validator stake lamports |

### Staker `VoteOverride`

- Account size: 233 bytes
- Discriminator filter: offset 0, base58 `NoiLFMSoFVm`
- Proposal filter: offset 104, proposal public key
- Compact data slice: offset 192, length 32

| Slice offset | Field |
| ---: | --- |
| 0 | For override lamports |
| 8 | Against override lamports |
| 16 | Abstain override lamports |
| 24 | Override stake lamports |

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
- Confirm the Total, Validators, and Staker overrides rows each have three displayed percentages that sum to 100% within four-decimal rounding tolerance.
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
