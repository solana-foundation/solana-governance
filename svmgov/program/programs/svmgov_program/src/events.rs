use anchor_lang::prelude::*;

#[event]
pub struct ProposalCreated {
    pub proposal_id: Pubkey,
    pub author: Pubkey,
    pub title: String,
    pub description: String,
    pub creation_timestamp: i64,
    /// Key that signed the transaction: the identity, or its accepted delegate.
    pub signer: Pubkey,
}

#[event]
pub struct ProposalDescriptionUpdated {
    pub proposal_id: Pubkey,
    pub author: Pubkey,
    pub previous_description: String,
    pub new_description: String,
    pub update_timestamp: i64,
    /// Key that signed the transaction: the identity, or its accepted delegate.
    pub signer: Pubkey,
}

#[event]
pub struct ProposalSupported {
    pub proposal_id: Pubkey,
    pub supporter: Pubkey,
    pub cluster_support_lamports: u64,
    pub voting_activated: bool,
    pub snapshot_slot: u64,
    /// Key that signed the transaction: the identity, or its accepted delegate.
    pub signer: Pubkey,
}

#[event]
pub struct ProposalRetallied {
    pub proposal_id: Pubkey,
    pub caller: Pubkey,
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
    /// Key that signed the transaction: the identity, or its accepted delegate.
    pub signer: Pubkey,
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
    /// Key that signed the transaction: the voting wallet, or its accepted delegate.
    pub signer: Pubkey,
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
    /// Key that signed the transaction: the identity, or its accepted delegate.
    pub signer: Pubkey,
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
    /// Key that signed the transaction: the voting wallet, or its accepted delegate.
    pub signer: Pubkey,
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

#[event]
pub struct AdminNominated {
    pub current_admin: Pubkey,
    pub pending_admin: Pubkey,
}

#[event]
pub struct AdminTransferred {
    pub previous_admin: Pubkey,
    pub new_admin: Pubkey,
}

#[event]
pub struct VoteDelegationCreated {
    /// SPL vote account (validator) or voting wallet (staker) being delegated.
    pub subject: Pubkey,
    /// The subject's own voting key at creation time: the vote account's node
    /// identity, or the wallet itself.
    pub identity: Pubkey,
    pub delegate: Pubkey,
    /// Key that created the delegation: the identity or authorized withdrawer
    /// of a vote account, or the wallet itself.
    pub signer: Pubkey,
}

#[event]
pub struct VoteDelegationAccepted {
    pub subject: Pubkey,
    pub delegate: Pubkey,
}

#[event]
pub struct VoteDelegationRemoved {
    pub subject: Pubkey,
    pub delegate: Pubkey,
    pub signer: Pubkey,
    pub was_accepted: bool,
}
