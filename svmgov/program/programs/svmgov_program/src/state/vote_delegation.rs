use anchor_lang::prelude::*;

/// Standing delegation of an SPL vote account's governance authority to another key.
///
/// PDA seeds: `[VoteDelegation::SEED, spl_vote_account]`. The vote account's node
/// identity or authorized withdrawer creates it; it is inert until the delegate
/// accepts. While accepted, the delegate alone may create, support, vote on and
/// modify votes for the vote account, and the node identity is locked out until
/// the delegate or the authorized withdrawer removes the delegation.
#[account]
#[derive(InitSpace)]
pub struct VoteDelegation {
    /// Key authorized to act for `vote_account` once `delegation_accepted` is set.
    /// First field so clients can find delegations by delegate with a memcmp at
    /// offset 8.
    pub delegate: Pubkey,
    /// SPL vote account whose governance authority is delegated.
    pub vote_account: Pubkey,
    /// Key that created and paid for this account; receives its rent on removal.
    pub creator: Pubkey,
    /// Set by `accept_vote_delegation`. Until then the identity keeps voting and
    /// the delegate has no authority.
    pub delegation_accepted: bool,
    pub bump: u8,
}

impl VoteDelegation {
    pub const SEED: &'static [u8] = b"vote_delegation";
}
