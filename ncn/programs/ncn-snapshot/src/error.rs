use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Operator not whitelisted")]
    OperatorNotWhitelisted,
    #[msg("Operator has voted")]
    OperatorHasVoted,
    #[msg("Operator has not voted")]
    OperatorHasNotVoted,
    #[msg("Voting has expired")]
    VotingExpired,
    #[msg("Voting not expired")]
    VotingNotExpired,
    #[msg("Consensus has reached")]
    ConsensusReached,
    #[msg("Consensus not reached")]
    ConsensusNotReached,
    #[msg("Invalid ballot")]
    InvalidBallot,
    #[msg("Invalid merkle inputs")]
    InvalidMerkleInputs,
    #[msg("Invalid merkle proof")]
    InvalidMerkleProof,
    #[msg("Vector size exceeded")]
    VecFull,
    #[msg("Overlapping operators in add and remove lists")]
    OverlappingWhitelistEntries,
    #[msg("Invalid ballot index")]
    InvalidBallotIndex,
    #[msg("Snapshot slot must be greater than current slot")]
    InvalidSnapshotSlot,
    #[msg("Ballot tallies not at max length")]
    BallotTalliesNotMaxLength,
    #[msg("Invalid proposal")]
    InvalidProposal,
}
