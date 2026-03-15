# Solana Governance Contract
A decentralized governance system built on the Solana blockchain.

## Overview
This repository contains a Solana program that enables a decentralized governance system. The contract allows validators to create proposals, vote on them, and tally the results.

## Features

* **Proposal creation**: Validators can create new proposals with a title, description, and voting period, with merkle proof verification for stake validation.
* **Proposal support**: Validators can show support for a proposal, which helps to activate voting with enhanced stake verification.
* **Voting**: Validators can cast votes on a proposal, with their vote weight determined by their stake.
* **Delegator voting**: Delegators can vote independently on proposals using their stake accounts, either before their validator votes (cached) or as an override after the validator has voted.
* **Vote override**: When a validator has already voted, delegators can override their validator's vote using stake account verification and merkle proofs.
* **Vote caching**: When delegators vote before their validator, their votes are cached and will be applied when the validator eventually votes.
* **Merkle proof verification**: Comprehensive integration with external snapshot programs for stake verification.
* **PDA utilities**: Robust program-derived address derivation for all contract accounts.
* **Enhanced validation**: Improved error handling and input validation throughout the contract.

## Contract Structure

The contract is organized into several modules:

* `error.rs`: Defines custom error codes used throughout the contract with enhanced validation messages.
* `lib.rs`: Contains the main program logic, including functions for creating proposals, casting votes, and finalizing results.
* `merkle_helpers.rs`: Provides utilities for merkle proof verification and cross-program invocation.
* `utils.rs`: Provides utility functions, such as calculating stake weights in basis points and PDA derivation.
* `state`: Defines the data structures used to store proposal, vote, vote override, and vote override cache information.
* `instructions`: Contains the implementation of each instruction, including create proposal, cast vote, cast vote override, modify vote, support proposal, finalize proposal, and add merkle root.


## CLI Interface

This repository also includes a command-line interface (CLI) program, `svmgov`, which provides a convenient way to interact with the contract. The `svmgov` CLI allows validators and delegators to create proposals, cast votes, override votes, and perform other actions on the contract with a simple CLI interface. The validator/delegator identity keypair is necessary for most commands, and supports API integration for real-time stake verification.

## Usage

To use this contract, you'll need to:

1. **Build and deploy**: Build the contract using `anchor build` and deploy it to a Solana cluster.
2. **Initialize index**: Use the `initialize_index` instruction to set up the proposal index PDA.
3. **Create a proposal**: Use the `create_proposal` instruction to create a new proposal with merkle proof verification for stake validation.
4. **Support a proposal**: Use the `support_proposal` instruction to show support for a proposal with stake verification.
5. **Cast a vote**: Use the `cast_vote` instruction to cast a validator vote on a proposal.
6. **Cast delegator vote**: Use the `cast_vote_override` instruction for delegators to vote on a proposal. This works in two scenarios:
   - **Independent voting**: If the validator hasn't voted yet, the delegator's vote is cached and will be applied when the validator votes
   - **Override voting**: If the validator has already voted, the delegator's vote overrides the validator's vote for their stake portion
7. **Modify vote**: Use the `modify_vote` instruction to update an existing vote.
8. **Add merkle root**: Use the `add_merkle_root` instruction to set the merkle root hash for a proposal.
9. **Finalize proposal**: Use the `finalize_proposal` instruction to determine the outcome after voting ends.

## Events

The contract emits comprehensive events for all major governance actions. Frontend applications and external services can listen to these events to track governance activity in real-time. All events are automatically included in the generated IDL.

### ProposalCreated
Emitted when a new proposal is created.

<details>
<summary><strong>Click to view event fields</strong></summary>

- `proposal_id: Pubkey` - The unique identifier of the proposal
- `author: Pubkey` - The public key of the validator who created the proposal
- `title: String` - The proposal title
- `description: String` - The proposal description
- `start_epoch: u64` - The epoch when voting begins
- `end_epoch: u64` - The epoch when voting ends
- `snapshot_slot: u64` - The slot when the validator stake snapshot was taken
- `creation_timestamp: i64` - Unix timestamp of proposal creation

</details>

### ProposalSupported
Emitted when a validator supports a proposal.

<details>
<summary><strong>Click to view event fields</strong></summary>

- `proposal_id: Pubkey` - The proposal being supported
- `supporter: Pubkey` - The validator providing support
- `cluster_support_lamports: u64` - Total lamports of cluster support after this action
- `voting_activated: bool` - Whether this support activated voting (5% threshold reached)

</details>

### VoteCast
Emitted when a validator casts their vote.

<details>
<summary><strong>Click to view event fields</strong></summary>

- `proposal_id: Pubkey` - The proposal being voted on
- `voter: Pubkey` - The validator casting the vote
- `vote_account: Pubkey` - The validator's vote account
- `for_votes_bp: u64` - Basis points allocated to "For"
- `against_votes_bp: u64` - Basis points allocated to "Against"
- `abstain_votes_bp: u64` - Basis points allocated to "Abstain"
- `for_votes_lamports: u64` - Lamports allocated to "For" (based on stake)
- `against_votes_lamports: u64` - Lamports allocated to "Against" (based on stake)
- `abstain_votes_lamports: u64` - Lamports allocated to "Abstain" (based on stake)
- `vote_timestamp: i64` - Unix timestamp of the vote

</details>

### VoteOverrideCast
Emitted when a delegator votes on a proposal, either as an independent vote (cached if validator hasn't voted) or as an override of their validator's existing vote.

<details>
<summary><strong>Click to view event fields</strong></summary>

- `proposal_id: Pubkey` - The proposal being voted on
- `delegator: Pubkey` - The delegator overriding the vote
- `stake_account: Pubkey` - The stake account being used
- `validator: Pubkey` - The validator whose vote is being overridden
- `for_votes_bp: u64` - Basis points allocated to "For"
- `against_votes_bp: u64` - Basis points allocated to "Against"
- `abstain_votes_bp: u64` - Basis points allocated to "Abstain"
- `for_votes_lamports: u64` - Lamports allocated to "For"
- `against_votes_lamports: u64` - Lamports allocated to "Against"
- `abstain_votes_lamports: u64` - Lamports allocated to "Abstain"
- `stake_amount: u64` - The amount of stake being used for the override
- `vote_timestamp: i64` - Unix timestamp of the vote override

</details>

### VoteModified
Emitted when a validator modifies their existing vote.

<details>
<summary><strong>Click to view event fields</strong></summary>

- `proposal_id: Pubkey` - The proposal being voted on
- `voter: Pubkey` - The validator modifying their vote
- `vote_account: Pubkey` - The validator's vote account
- `old_for_votes_bp: u64` - Previous basis points for "For"
- `old_against_votes_bp: u64` - Previous basis points for "Against"
- `old_abstain_votes_bp: u64` - Previous basis points for "Abstain"
- `new_for_votes_bp: u64` - New basis points for "For"
- `new_against_votes_bp: u64` - New basis points for "Against"
- `new_abstain_votes_bp: u64` - New basis points for "Abstain"
- `for_votes_lamports: u64` - Lamports allocated to "For"
- `against_votes_lamports: u64` - Lamports allocated to "Against"
- `abstain_votes_lamports: u64` - Lamports allocated to "Abstain"
- `modification_timestamp: i64` - Unix timestamp of the modification

</details>

### MerkleRootAdded
Emitted when a merkle root hash is added to a proposal.

<details>
<summary><strong>Click to view event fields</strong></summary>

- `proposal_id: Pubkey` - The proposal receiving the merkle root
- `author: Pubkey` - The validator adding the merkle root
- `merkle_root_hash: [u8; 32]` - The merkle root hash bytes

</details>

### ProposalFinalized
Emitted when a proposal is finalized after voting ends.

<details>
<summary><strong>Click to view event fields</strong></summary>

- `proposal_id: Pubkey` - The finalized proposal
- `finalizer: Pubkey` - The account that finalized the proposal
- `total_for_votes: u64` - Total lamports voted "For"
- `total_against_votes: u64` - Total lamports voted "Against"
- `total_abstain_votes: u64` - Total lamports voted "Abstain"
- `total_votes_count: u32` - Total number of votes cast
- `finalization_timestamp: i64` - Unix timestamp of finalization

</details>

## Event Usage

Frontend applications can listen to these events using Anchor's event system:

```javascript
import * as anchor from '@coral-xyz/anchor';

// Initialize program
const program = new anchor.Program(IDL, PROGRAM_ID, provider);

// Listen for proposal creation
const proposalListener = program.addEventListener('ProposalCreated', (event, slot) => {
  console.log('New proposal:', event.title);
  // Update proposals list in UI
});

// Listen for votes
const voteListener = program.addEventListener('VoteCast', (event, slot) => {
  console.log('Vote cast:', event.forVotesBp, 'basis points');
  // Update voting results in real-time
});

// Listen for delegator votes (both independent and override)
const overrideListener = program.addEventListener('VoteOverrideCast', (event, slot) => {
  console.log('Delegator vote cast:', event.forVotesBp, 'basis points');
  // Update delegator voting status (could be cached or override)
});

// Cleanup listeners when component unmounts
// program.removeEventListener(proposalListener);
```

Events are strongly typed and included in the generated TypeScript types from the IDL.


## Development

To contribute to this project, you'll need:

1. **Rust and Solana tools**: Install Rust and the Solana(agave) CLI using the official instructions.
2. **Cargo**: Use Cargo to build and manage dependencies for the contract.
3. **Anchor**: Use Anchor to generate and manage the contract's IDL files.
