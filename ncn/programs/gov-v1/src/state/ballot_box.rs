use anchor_lang::prelude::*;

use crate::MAX_OPERATOR_WHITELIST;

pub const MAX_OPERATOR_VOTES: usize = 64;
pub const MAX_BALLOT_TALLIES: usize = 64;

#[account]
#[derive(InitSpace, Debug)]
pub struct BallotBox {
    /// Bump seed for the PDA
    pub bump: u8,
    /// The epoch this ballot box is for
    pub epoch: u64,
    /// Slot when this ballot box was created
    pub slot_created: u64,
    /// Slot when consensus was reached
    pub slot_consensus_reached: u64,
    /// Min. percentage of votes required to finalize for this ballot box.
    pub min_consensus_threshold_bps: u16,
    /// The ballot that got at least min_consensus_threshold of votes
    pub winning_ballot: Ballot,
    /// Operator votes
    #[max_len(MAX_OPERATOR_VOTES)]
    pub operator_votes: Vec<OperatorVote>,
    /// Mapping of ballots votes to stake weight
    #[max_len(MAX_BALLOT_TALLIES)]
    pub ballot_tallies: Vec<BallotTally>,
    /// Timestamp when voting ends. Tie breaker admin will decide the results
    /// if no consensus is reached by then.
    pub vote_expiry_timestamp: i64,
    /// Slot for which the snapshot is taken
    pub snapshot_slot: u64,
    /// Snapshot of whitelisted operators at BallotBox creation
    #[max_len(MAX_OPERATOR_WHITELIST)]
    pub voter_list: Vec<Pubkey>,
    /// Whether consensus was reached via tie breaker
    pub tie_breaker_consensus: bool,
}

impl BallotBox {
    pub fn pda(snapshot_slot: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"BallotBox", &snapshot_slot.to_le_bytes()], &crate::ID)
    }

    pub fn has_vote_expired(&self, current_timestamp: i64) -> bool {
        current_timestamp >= self.vote_expiry_timestamp
    }

    pub fn has_consensus_reached(&self) -> bool {
        self.slot_consensus_reached != 0
    }
}

/// Inner struct of BallotBox
#[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, InitSpace, PartialEq, Default)]
pub struct Ballot {
    /// The merkle root of the meta merkle tree
    pub meta_merkle_root: [u8; 32],
    /// SHA256 hash of borsh serialized snapshot. Optional.
    pub snapshot_hash: [u8; 32],
}

/// Inner struct of BallotBox
#[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, InitSpace, PartialEq)]
pub struct OperatorVote {
    /// The operator that cast the vote
    pub operator: Pubkey,
    /// The slot the operator voted
    pub slot_voted: u64,
    /// The index of the ballot in the ballot_tallies
    pub ballot_index: u8,
}

/// Inner struct of BallotBox
#[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, InitSpace, PartialEq)]
pub struct BallotTally {
    /// Index of the tally within the ballot_tallies
    pub index: u8,
    /// The ballot being tallied
    pub ballot: Ballot,
    /// The number of votes for this ballot. Each vote is equally weighted.
    pub tally: u8,
}
