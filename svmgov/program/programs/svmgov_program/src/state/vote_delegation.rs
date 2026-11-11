use anchor_lang::prelude::*;

/// Standing delegation of a governance voter's authority to another key.
///
/// The `subject` is whoever's vote is delegated: an SPL vote account for a
/// validator, or a voting wallet (a stake account's withdraw authority) for a
/// staker. PDA seeds: `[VoteDelegation::SEED, subject]`.
///
/// A validator delegation is created by the vote account's node identity or
/// authorized withdrawer; a staker delegation by the wallet itself. Either is
/// inert until the delegate accepts. While accepted, the delegate alone may act
/// for the subject (create, support, vote, modify, override) and the subject's
/// own voting key is locked out until the delegation is removed by the delegate
/// or the subject's root authority (the authorized withdrawer, or the wallet).
/// Removal returns the rent to that root authority.
#[account]
#[derive(InitSpace)]
pub struct VoteDelegation {
    /// Key authorized to act for `subject` once `delegation_accepted` is set.
    /// First field so clients can find delegations by delegate with a memcmp at
    /// offset 8.
    pub delegate: Pubkey,
    /// SPL vote account (validator) or voting wallet (staker) whose governance
    /// authority is delegated.
    pub subject: Pubkey,
    /// Set by `accept_vote_delegation`. Until then the subject's own key keeps
    /// voting and the delegate has no authority.
    pub delegation_accepted: bool,
    pub bump: u8,
}

impl VoteDelegation {
    pub const SEED: &'static [u8] = b"vote_delegation";
}
