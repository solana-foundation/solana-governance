# Governance Voter Snapshot Program

[← Back to Project README](../../README.md)

## Purpose

This program enables whitelisted operators to vote on stake distribution snapshots that will be used for governance voting. It provides consensus functionality for agreeing on a snapshot, along with helper instructions to verify stake weights from the agreed-upon snapshot.

---

## Program Accounts and Instructions

### Account Types

| Account Type      | Purpose                                                                                                                                                                                                                                      |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `BallotBox`       | Stores voting state, consensus threshold, and ballot configuration. Each BallotBox is uniquely identified by `snapshot_slot`, ensuring a 1:1 mapping between slots and ballot boxes. Contains a snapshot of the voter list at creation time. |
| `ConsensusResult` | Stores the finalized `meta_merkle_root` and `snapshot_hash` for a completed vote.                                                                                                                                                            |
| `MetaMerkleProof` | Stores the proof data required to verify a single validator's stake in a snapshot.                                                                                                                                                           |
| `ProgramConfig`   | Stores program-wide configuration, including admin and global operator whitelist.                                                                                                                                                            |

### Instruction Set

| Instruction Name              | Signer                                | Description                                                                                                                                                                                             |
| ----------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `init_program_config`         | Admin                                 | Initializes a default `ProgramConfig` account. All configs except `authority` needs to be set separately.                                                                                               |
| `update_program_config`       | Admin                                 | Updates `ProgramConfig` parameters.                                                                                                                                                                     |
| `finalize_proposed_authority` | Proposed Authority                    | Finalizes the proposed authority.                                                                                                                                                                       |
| `update_operator_whitelist`   | Admin                                 | Adds or removes operators from the whitelist in `ProgramConfig`.                                                                                                                                        |
| `init_ballot_box`             | Gov Contract PDA                      | Initializes a new `BallotBox` for a snapshot selection vote. Requires `snapshot_slot` parameter which must be greater than current slot. Snapshot of `whitelisted_operators` is stored in `voter_list`. |
| `cast_vote`                   | Operator                              | Casts a vote in a `BallotBox` for a specific `Ballot`.                                                                                                                                                  |
| `remove_vote`                 | Operator                              | Removes a previously casted vote                                                                                                                                                                        |
| `finalize_ballot`             | Any (payer)                           | Creates a `ConsensusResult` after consensus is reached for a `BallotBox`                                                                                                                                |
| `set_tie_breaker`             | Tie Breaker Admin                     | Allows the tie breaker admin to select any winning ballot (not limited to existing ballots) if consensus is not reached by expiry. Sets `tie_breaker_consensus` flag to true.                           |
| `reset_ballot_box`            | Tie Breaker Admin                     | Allows recovery of a bricked BallotBox by clearing votes and tallies. Only allowed when consensus not reached, voting not expired, and ballot tallies at max length.                                    |
| `init_meta_merkle_proof`      | Any (payer)                           | Initializes a `MetaMerkleProof` account to store proof and merkle leaf for a vote account.                                                                                                              |
| `verify_merkle_proof`         | Permissionless                        | Verifies that a stake or vote account leaf is included in the `ConsensusResult` merkle root                                                                                                             |
| `close_meta_merkle_proof`     | Creator (permissionless after expiry) | Closes a `MetaMerkleProof` account                                                                                                                                                                      |

## Design Constraints and Considerations

### 1. Merkle Tree Format

Snapshots are structured as a two-tier Merkle tree:

- [`MetaMerkleSnapshot`](../../cli/src/merkle.rs#L11): encapsulates the top-level Merkle root, snapshot slot, and all associated leaf bundles.
- [`MetaMerkleLeaf`](../../programs/gov-v1/src/state/proof.rs#L40): represents a `VoteAccount` node in the top-level Merkle tree, containing the subroot of its delegated stake tree and total stake.
- [`StakeMerkleLeaf`](../../programs/gov-v1/src/state/proof.rs#L64): represents a `StakeAccount` leaf in the bottom-level Merkle tree, containing the voting wallet, stake pubkey, and active delegated stake.

All hashing uses SHA-256 and follows Solana canonical Merkle tree layout (`left || right`, sorted lexicographically by node).

---

### 2. Transaction Size Limits

Solana enforces a strict 1232-byte limit on transaction payloads. Based on current Solana Mainnet statistics, there are 1087 active validators with the largest validator Everstake having 195492 delegated stake accounts. To ensure that `verify` instructions remain composable on extremely large validator and stake account sets, even in CPI calls from a voting program:

- The ConsensusResult account stores only the finalized `meta_merkle_root` and `snapshot_hash`, minimizing CU utilization.
- `MetaMerkleProof` is used to store the leaf data and proof required to verify stake weight of a single validator in a snapshot, reducing transaction payload of the subsequent `verify` instruction.
- The `verify` instruction is invoked separately (through CPI from the voting program) to verify either the stake weight of a vote account or a stake account.
- `MetaMerkleProof` is intended to be initialized once per validator per governance vote, typically by the first voter. Subsequent voters for the same validator reuse this proof account without having to reinitialize.
- `MetaMerkleProof` can be closed before the indicated expiry time by its creator or permissionlessly after expiry.
- Supporting UI should handle edge cases of checking the existence and initializing of `MetaMerkleProof` if needed, and closing it after end of voting.

---

### 3. Operator Voting

Operator voting has the following constraints:

- A maximum of 64 operators can be whitelisted for voting concurrently
- The `min_consensus_threshold_bps` is fixed at the time of `BallotBox` initialization and cannot be changed.
- All whitelisted operators have equal voting weight.
- Operators can continue to cast votes after consensus is reached, though votes cannot be removed once cast.
- There is a limit of 64 unique ballots that can be cast, including any ballots where votes were subsequently reduced to 0.
- The voter set is fixed at `BallotBox` creation time via the `voter_list` field, which contains a snapshot of `ProgramConfig.whitelisted_operators`. Only operators in this snapshot can vote in the ballot box.

### 4. Operator Whitelisting and Voter Snapshot

The `BallotBox` stores a snapshot of `whitelisted_operators` at creation time in the `voter_list` field. This ensures that:

- Changes to the operator whitelist during voting do not affect ongoing votes
- The consensus threshold is calculated based on the snapshot size, preventing disputes
- Each ballot box has a fixed, immutable voter set

---

### 5. Snapshot Identity and Uniqueness

Each snapshot is uniquely identified by its `snapshot_hash`, defined as the SHA-256 hash of the Borsh-serialized `MetaMerkleSnapshot`. This enables UIs and operators to confirm that any off-chain file corresponds to an on-chain ConsensusResult.

Each `BallotBox` is uniquely identified by its `snapshot_slot`, which is used as a seed for the PDA. This ensures a 1:1 mapping between slots and ballot boxes, preventing recreation and ensuring each slot can only have one ballot box.

---

### 6. Tie Breaking and Recovery

**Tie Breaking:**
If consensus is not reached before `vote_expiry_timestamp`, the `tie_breaker_admin` is allowed to select any ballot value (not limited to existing ballots in the BallotBox). This ensures liveness and allows governance recovery from operator deadlock. When a tie breaker is used, the `tie_breaker_consensus` flag is set to `true` in the `BallotBox` and propagated to the `ConsensusResult`.

**BallotBox Recovery:**
In the event that a `BallotBox` becomes bricked with invalid ballots, the `reset_ballot_box` instruction allows the tie breaker admin to clear all votes and ballot tallies, enabling recovery. This is only allowed when:

- Consensus has not been reached
- Voting has not expired (after expiry, use `set_tie_breaker` instead)
- The ballot tallies have reached maximum length (all entries used)

This recovery mechanism replaces the previous approach of recreating BallotBoxes, which is no longer possible due to the 1:1 slot mapping.

---

### 7. Cross-Program Invocation (CPI) and Testing

**Production CPI Requirement:**

The `init_ballot_box` instruction enforces a CPI requirement in production, requiring the `proposal` account to be a PDA from the governance program (`GoVpHPV3EY89hwKJjfw19jTdgMsGKG4UFSE2SfJqTuhc`). This ensures that ballot boxes can only be created through the governance program's proposal flow.

**Testing with `skip-pda-check` Feature:**

For local testing without setting up a full governance program, the `skip-pda-check` feature flag disables the PDA check, allowing `init_ballot_box` to be called directly with any signer as the `proposal` account.

```bash
anchor test -- --features skip-pda-check
```

**For production deployments**, build without the feature flag (default) to enforce the CPI requirement:

```bash
anchor build
```
