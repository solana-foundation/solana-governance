//! Resolution of who may act for a governance voter: the voter's own key by
//! default (a validator's node identity, or a staker's voting wallet), or the
//! accepted delegate recorded in a `VoteDelegation`.

use anchor_lang::{
    prelude::*,
    solana_program::vote::{program as vote_program, state::VoteState},
};
use solana_vote_interface::state::VoteStateVersions;

use crate::{error::GovernanceError, state::VoteDelegation};

/// Keys that control an SPL vote account.
pub struct VoteAccountAuthorities {
    /// Validator identity (`node_pubkey`), the default governance voting key.
    pub node_pubkey: Pubkey,
    /// Root authority of the vote account, normally held cold.
    pub authorized_withdrawer: Pubkey,
}

/// Whether `account` is an initialized SPL vote account: owned by the vote
/// program and of the current vote state size.
pub fn is_live_vote_account(account: &AccountInfo) -> bool {
    account.owner == &vote_program::ID && account.data_len() == VoteState::size_of()
}

/// Reads the node identity and authorized withdrawer from raw SPL vote account
/// data. Fails for uninitialized or undecodable vote state.
pub fn read_vote_account_authorities(vote_account_data: &[u8]) -> Result<VoteAccountAuthorities> {
    let versioned = VoteStateVersions::deserialize(vote_account_data)
        .map_err(|_| GovernanceError::FailedDeserializeNodePubkey)?;
    let (node_pubkey, authorized_withdrawer) = match &versioned {
        VoteStateVersions::V3(v) => (v.node_pubkey.to_bytes(), v.authorized_withdrawer.to_bytes()),
        VoteStateVersions::V4(v) => (v.node_pubkey.to_bytes(), v.authorized_withdrawer.to_bytes()),
        VoteStateVersions::V1_14_11(v) => {
            (v.node_pubkey.to_bytes(), v.authorized_withdrawer.to_bytes())
        }
        VoteStateVersions::Uninitialized => {
            return Err(GovernanceError::InvalidVoteAccountVersion.into())
        }
    };
    Ok(VoteAccountAuthorities {
        node_pubkey: Pubkey::new_from_array(node_pubkey),
        authorized_withdrawer: Pubkey::new_from_array(authorized_withdrawer),
    })
}

/// Loads the `VoteDelegation` for `subject` from its PDA, if this program has
/// initialized one. A missing or empty account yields `None`; an account owned
/// by this program that does not hold a delegation for `subject` is an error.
pub fn load_vote_delegation(
    vote_delegation: &AccountInfo,
    subject: &Pubkey,
) -> Result<Option<VoteDelegation>> {
    if vote_delegation.data_len() == 0 || vote_delegation.owner != &crate::ID {
        return Ok(None);
    }
    let delegation: VoteDelegation =
        AccountDeserialize::try_deserialize(&mut vote_delegation.data.borrow().as_ref())
            .map_err(|_| GovernanceError::InvalidVoteDelegation)?;
    require_keys_eq!(
        delegation.subject,
        *subject,
        GovernanceError::InvalidVoteDelegation
    );
    Ok(Some(delegation))
}

/// Requires `signer` to hold governance authority for `subject` (an SPL vote
/// account or a voting wallet), whose own voting key is `identity`. With an
/// accepted delegation on record only the delegate qualifies and the identity
/// is locked out; otherwise only the identity qualifies, failing with
/// `identity_mismatch`. A pending delegation never restricts the identity, even
/// one naming the identity itself. Any other key named as delegate on a pending
/// delegation gets `VoteDelegationNotAccepted` so the missing acceptance step
/// is evident.
///
/// | `vote_delegation`  | `signer`   | Result                      |
/// |--------------------|------------|-----------------------------|
/// | none               | `identity` | `Ok`                        |
/// | none               | other      | `identity_mismatch`         |
/// | pending            | `identity` | `Ok`                        |
/// | pending            | delegate   | `VoteDelegationNotAccepted` |
/// | pending            | other      | `identity_mismatch`         |
/// | accepted           | delegate   | `Ok`                        |
/// | accepted           | `identity` | `NotVoteDelegate`           |
/// | accepted           | other      | `NotVoteDelegate`           |
/// | not for `subject`  | any        | `InvalidVoteDelegation`     |
pub fn require_vote_authority(
    signer: &Pubkey,
    identity: &Pubkey,
    vote_delegation: &AccountInfo,
    subject: &Pubkey,
    identity_mismatch: GovernanceError,
) -> Result<()> {
    match load_vote_delegation(vote_delegation, subject)? {
        Some(delegation) if delegation.delegation_accepted => {
            require_keys_eq!(
                *signer,
                delegation.delegate,
                GovernanceError::NotVoteDelegate
            );
        }
        Some(delegation) if delegation.delegate == *signer && signer != identity => {
            return Err(GovernanceError::VoteDelegationNotAccepted.into());
        }
        _ => {
            require_keys_eq!(*signer, *identity, identity_mismatch);
        }
    }
    Ok(())
}
