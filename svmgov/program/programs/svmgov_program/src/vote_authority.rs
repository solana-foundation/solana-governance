//! Resolution of who may act for an SPL vote account in governance: its node
//! identity by default, or the accepted delegate recorded in a `VoteDelegation`.

use anchor_lang::prelude::*;
use solana_vote_interface::state::VoteStateVersions;

use crate::{error::GovernanceError, state::VoteDelegation};

/// Keys that control an SPL vote account.
pub struct VoteAccountAuthorities {
    /// Validator identity (`node_pubkey`), the default governance voting key.
    pub node_pubkey: Pubkey,
    /// Root authority of the vote account, normally held cold.
    pub authorized_withdrawer: Pubkey,
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

/// Loads the `VoteDelegation` for `vote_account` from its PDA, if this program
/// has initialized one. A missing or empty account yields `None`; an account
/// owned by this program that does not hold a delegation for `vote_account` is
/// an error.
pub fn load_vote_delegation(
    vote_delegation: &AccountInfo,
    vote_account: &Pubkey,
) -> Result<Option<VoteDelegation>> {
    if vote_delegation.data_len() == 0 || vote_delegation.owner != &crate::ID {
        return Ok(None);
    }
    let delegation: VoteDelegation =
        AccountDeserialize::try_deserialize(&mut vote_delegation.data.borrow().as_ref())
            .map_err(|_| GovernanceError::InvalidVoteDelegation)?;
    require_keys_eq!(
        delegation.vote_account,
        *vote_account,
        GovernanceError::InvalidVoteDelegation
    );
    Ok(Some(delegation))
}

/// Requires `signer` to hold governance authority for `vote_account`, whose
/// identity key is `identity`. With an accepted delegation on record only the
/// delegate qualifies and the identity is locked out; otherwise only the
/// identity qualifies, failing with `identity_mismatch`. A delegate acting on a
/// delegation that is still pending gets `VoteDelegationNotAccepted` so the
/// missing acceptance step is evident.
pub fn require_vote_authority(
    signer: &Pubkey,
    identity: &Pubkey,
    vote_delegation: &AccountInfo,
    vote_account: &Pubkey,
    identity_mismatch: GovernanceError,
) -> Result<()> {
    match load_vote_delegation(vote_delegation, vote_account)? {
        Some(delegation) if delegation.delegation_accepted => {
            require_keys_eq!(
                *signer,
                delegation.delegate,
                GovernanceError::NotVoteDelegate
            );
        }
        Some(delegation) if delegation.delegate == *signer => {
            return Err(GovernanceError::VoteDelegationNotAccepted.into());
        }
        _ => {
            require_keys_eq!(*signer, *identity, identity_mismatch);
        }
    }
    Ok(())
}
