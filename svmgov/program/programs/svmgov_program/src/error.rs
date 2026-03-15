use anchor_lang::prelude::*;

#[error_code]
pub enum GovernanceError {
    #[msg("Insufficient stake to perform this action")]
    NotEnoughStake,
    #[msg("The title of the proposal cannot be empty")]
    TitleEmpty,
    #[msg("The title of the proposal is too long, max 50 char")]
    TitleTooLong,
    #[msg("The description of the proposal cannot be empty")]
    DescriptionEmpty,
    #[msg("The description of the proposal is too long, max 250 char")]
    DescriptionTooLong,
    #[msg("The description of the proposal must point to a github link")]
    DescriptionInvalid,
    #[msg("Invalid proposal ID")]
    InvalidProposalId,
    #[msg("Voting on proposal not yet started")]
    VotingNotStarted,
    #[msg("Proposal voting period has ended")]
    ProposalClosed,
    #[msg("Proposal has already been finalized")]
    ProposalFinalized,
    #[msg("Vote distribution must add up to 100% in Basis Points")]
    InvalidVoteDistribution,
    #[msg("Voting period not yet ended")]
    VotingPeriodNotEnded,
    #[msg("Invalid vote account")]
    InvalidVoteAccount,
    #[msg("Failed to deserialize node_pubkey from Vote account")]
    FailedDeserializeNodePubkey,
    #[msg("Deserialized node_pubkey from Vote accounts does not match")]
    VoteNodePubkeyMismatch,
    #[msg("Not enough accounts for tally")]
    NotEnoughAccounts,
    #[msg("Cluster stake cannot be zero")]
    InvalidClusterStake,
    #[msg("Start epoch must be current or future epoch")]
    InvalidStartEpoch,
    #[msg("Voting length must be bigger than 0")]
    InvalidVotingLength,
    #[msg("Invalid Vote account version")]
    InvalidVoteAccountVersion,
    #[msg("Invalid Vote account size")]
    InvalidVoteAccountSize,
    #[msg("Invalid stake account")]
    InvalidStakeAccount,
    #[msg("Invalid stake account state")]
    InvalidStakeState,
    #[msg("Invalid Stake account size")]
    InvalidStakeAccountSize,
    #[msg("Invalid Snapshot program: provided program ID does not match the expected Merkle Verifier Service program")]
    InvalidSnapshotProgram,
    #[msg("Only the original proposal author can add the merkle root hash")]
    UnauthorizedMerkleRootUpdate,
    #[msg("Merkle root hash is already set for this proposal")]
    MerkleRootAlreadySet,
    #[msg("Merkle root hash cannot be all zeros")]
    InvalidMerkleRoot,
    #[msg("Invalid snapshot slot: snapshot slot must be past or current slot")]
    InvalidSnapshotSlot,
    #[msg("Account must be owned by Snapshot program")]
    MustBeOwnedBySnapshotProgram,
    #[msg("Invalid consensus result PDA")]
    InvalidConsensusResultPDA,
    #[msg("Cannot deserialize MetaMerkleProof PDA")]
    CannotDeserializeMetaMerkleProofPDA,
    #[msg("Cannot deserialize ConsensusResult")]
    CannotDeserializeConsensusResult,
    #[msg("Cannot modify proposal after voting has started")]
    CannotModifyAfterStart,
    #[msg("Voting length exceeds maximum allowed epochs")]
    VotingLengthTooLong,
    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow,
    #[msg("Snapshot program has been upgraded, update protection triggered")]
    SnapshotProgramUpgraded,
    #[msg("Merkle root hash has not been set for this proposal")]
    MerkleRootNotSet,
    #[msg("Support period has expired for this proposal")]
    SupportPeriodExpired,
    #[msg("Not within the support period")]
    NotInSupportPeriod,
    #[msg("Consensus result has not been set for this proposal")]
    ConsensusResultNotSet,
    #[msg("Unauthorized: caller is not authorized to perform this action")]
    Unauthorized,
    #[msg("Proposal is not in voting phase")]
    ProposalNotInVotingPhase,
    #[msg("Invalid vote override cache")]
    InvalidVoteOverrideCache,
    #[msg("Stake account owner mismatch")]
    StakeAccountOwnerMismatch,
    #[msg("Unauthorized: only the admin can perform this action")]
    UnauthorizedAdmin,
}
