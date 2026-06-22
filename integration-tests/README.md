# Cross-program integration tests

Deploys **both** on-chain programs onto an in-process
[Surfpool](https://github.com/solana-foundation/surfpool) ephemeral network
("surfnet") and drives the full governance lifecycle end to end, including the
cross-program invocation and merkle-snapshot voting that tie the two together:

| Program                                        | Role in the flow                                                                                                                                      |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`svmgov_program`](../svmgov/program)          | proposal creation, support, voting, finalize                                                                                                          |
| [`ncn_snapshot`](../ncn/programs/ncn-snapshot) | operator ballot consensus + merkle-proof verification; `svmgov` CPIs into it during `support_proposal`, and reads its `ConsensusResult` during voting |

## The flow (`src/full_flow.rs`)

A single `#[tokio::test]` drives:

1. Deploy svmgov + ncn (admin = upgrade authority).
2. Configure svmgov (`initialize_config` + `initialize_index`).
3. Configure ncn (`init_program_config` linking svmgov, `update_program_config`, `update_operator_whitelist`).
4. Generate validator / staker / ncn-operator keypairs.
5. Set vote accounts + delegated stake via surfpool cheatcodes.
6. `create_proposal` (validator 1).
7. Support past the threshold (validators 1 then 2) → CPI into `ncn_snapshot::init_ballot_box`.
8. Operators upload a fake merkle snapshot → consensus → `ConsensusResult`; publish per-validator `MetaMerkleProof`s; time-travel into the voting epoch.
9. Validators `cast_vote` and a staker `cast_vote_override` — **stake is sourced from the merkle snapshot** (verified via CPI against `ConsensusResult.meta_merkle_root`).
10. Time-travel past the voting window and `finalize_proposal`.

The on-chain epoch stake (read by create/support) and the fake snapshot (read by
voting) are built from the **same** keypairs/amounts, so the test proves the
snapshot is the voting stake source. Example tally asserted: FOR=60, AGAINST=100,
ABSTAIN=40 SOL (V1 100 FOR − 40 staker override → 60; staker 40 ABSTAIN; V2 100 AGAINST).

## Layout

```
integration-tests/
├── Cargo.toml                  # standalone workspace; surfpool-sdk + raw-encoding deps; litesvm patch
└── src/
    ├── lib.rs
    ├── harness.rs              # setup_scenario + Scenario types, deploy, raw anchor encoding,
    │                           #   PDAs, sorted merkle, cheatcode wrappers, account decoders, send/try_send
    ├── full_flow.rs            # the end-to-end happy-path #[tokio::test]
    └── proposal_validation.rs  # create_proposal input-validation failures
```

`harness::setup_scenario(&Scenario)` is the shared setup: given a `Scenario`
(admin + validators with stakes + operators + `GovConfigArgs` + fund amount) it
returns a `Surfnet` with both programs deployed, signers funded, both programs
configured, the operator whitelist set, and all vote/stake accounts created.
Tests drive the actual flow from there (and `cheats.time_travel_to_epoch(..)` as
needed). `try_send` returns the error (with program logs) for asserting failures.

We do **not** depend on the on-chain program crates: `svmgov_program` pulls
`solana-vote-interface 6.0`, which conflicts with surfpool's `litesvm`
(`solana-instruction =3.2.0`). So instructions are encoded raw (anchor
discriminator `sha256("global:<name>")[..8]` + borsh args) and accounts decoded
with borsh mirror structs — a single solana (3.x) version, no bridging.

## Running

```sh
make build-programs                              # from repo root: builds both .so + IDLs
cd integration-tests && cargo test -- --nocapture
```

The surfnet runs in-process; there is no external validator to manage. Tests:

- `full_flow::full_governance_flow` — the end-to-end lifecycle above (stake modeled
  via `SetStakeAccount`, aggregated into epoch stake automatically).
- `proposal_validation::*` — `create_proposal` input-validation failures
  (title too long, description too long, description not a github link).

## Required dependency patches (important)

Two supporting changes outside this crate are required for the flow to run, and
they are wired via path/patch from this workspace:

- **`surfpool-sdk`** is consumed from a local checkout (`../../surfpool`) for the
  vote/stake cheatcodes.
- **`litesvm` is patched to a local fork** (`../../litesvm`, via
  `[patch.crates-io]` in `Cargo.toml`). Stock litesvm 0.13 has no epoch-stake
  state, so the on-chain `sol_get_epoch_stake` / `get_epoch_stake_for_vote_account`
  syscalls — which `create_proposal`/`support_proposal` rely on — always return 0
  (→ division-by-zero `ArithmeticOverflow`). The fork adds settable epoch stakes
  to `LiteSVM` and overrides the `InvokeContextCallback` epoch-stake methods;
  surfpool-core populates them from its stake-delegation index whenever a stake
  account changes. See the surfpool/litesvm changes for details.

  This patch must live in **this** crate's `Cargo.toml` (not only surfpool's),
  because `integration-tests` is its own Cargo workspace and therefore the
  `[patch]` root for its own builds.

## Notes

- Programs are deployed via a raw `surfnet_writeProgram` call passing the admin as
  the upgrade authority (the SDK's `deploy` leaves it as the system program, which
  `initialize_config`'s `upgrade_authority == admin` check rejects).
- The vote account created by `SetVoteAccount` is padded to `VoteState::size_of()`
  (3762) via `set_account`, since the program enforces that exact data length.
- `Cargo.lock` and `.surfpool/` are gitignored.
