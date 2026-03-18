use anchor_lang::prelude::*;

#[event]
pub struct ProposalCreated {
    pub proposal_id: Pubkey,
    pub author: Pubkey,
    pub title: String,
    pub description: String,
    pub creation_timestamp: i64,
}

#[event]
pub struct ProposalSupported {
    pub proposal_id: Pubkey,
    pub supporter: Pubkey,
    pub cluster_support_lamports: u64,
    pub voting_activated: bool,
    pub snapshot_slot: u64,
}

#[event]
pub struct VoteCast {
    pub proposal_id: Pubkey,
    pub voter: Pubkey,
    pub vote_account: Pubkey,
    pub for_votes_bp: u64,
    pub against_votes_bp: u64,
    pub abstain_votes_bp: u64,
    pub for_votes_lamports: u64,
    pub against_votes_lamports: u64,
    pub abstain_votes_lamports: u64,
    pub vote_timestamp: i64,
}

#[event]
pub struct VoteOverrideCast {
    pub proposal_id: Pubkey,
    pub delegator: Pubkey,
    pub stake_account: Pubkey,
    pub validator: Pubkey,
    pub for_votes_bp: u64,
    pub against_votes_bp: u64,
    pub abstain_votes_bp: u64,
    pub for_votes_lamports: u64,
    pub against_votes_lamports: u64,
    pub abstain_votes_lamports: u64,
    pub stake_amount: u64,
    pub vote_timestamp: i64,
}

#[event]
pub struct VoteModified {
    pub proposal_id: Pubkey,
    pub voter: Pubkey,
    pub vote_account: Pubkey,
    pub old_for_votes_bp: u64,
    pub old_against_votes_bp: u64,
    pub old_abstain_votes_bp: u64,
    pub new_for_votes_bp: u64,
    pub new_against_votes_bp: u64,
    pub new_abstain_votes_bp: u64,
    pub for_votes_lamports: u64,
    pub against_votes_lamports: u64,
    pub abstain_votes_lamports: u64,
    pub modification_timestamp: i64,
}

#[event]
pub struct VoteOverrideModified {
    pub proposal_id: Pubkey,
    pub delegator: Pubkey,
    pub stake_account: Pubkey,
    pub validator: Pubkey,
    pub old_for_votes_bp: u64,
    pub old_against_votes_bp: u64,
    pub old_abstain_votes_bp: u64,
    pub new_for_votes_bp: u64,
    pub new_against_votes_bp: u64,
    pub new_abstain_votes_bp: u64,
    pub for_votes_lamports: u64,
    pub against_votes_lamports: u64,
    pub abstain_votes_lamports: u64,
    pub stake_amount: u64,
    pub modification_timestamp: i64,
}

#[event]
pub struct ProposalFinalized {
    pub proposal_id: Pubkey,
    pub finalizer: Pubkey,
    pub total_for_votes: u64,
    pub total_against_votes: u64,
    pub total_abstain_votes: u64,
    pub total_votes_count: u32,
    pub finalization_timestamp: i64,
}

#[event]
pub struct MerkleRootFlushed {
    pub proposal_id: Pubkey,
    pub author: Pubkey,
    pub new_snapshot_slot: u64,
    pub flush_timestamp: i64,
}
