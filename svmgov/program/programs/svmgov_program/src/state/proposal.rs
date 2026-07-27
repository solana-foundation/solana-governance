use crate::{constants::*, error::GovernanceError};
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Proposal {
    /// The public key of the validator who created this proposal
    pub author: Pubkey,
    #[max_len(MAX_TITLE_ACCOUNT_SIZE)]
    pub title: String,
    #[max_len(MAX_DESC_ACCOUNT_SIZE)]
    pub description: String,
    pub creation_epoch: u64,
    pub start_epoch: u64,
    pub end_epoch: u64,
    pub proposer_stake_weight_bp: u64,
    pub cluster_support_lamports: u64,
    /// Total lamports voted in favor of this proposal
    pub for_votes_lamports: u64,
    /// Total lamports voted against this proposal
    pub against_votes_lamports: u64,
    /// Total lamports that abstained from voting on this proposal
    pub abstain_votes_lamports: u64,
    pub voting: bool,
    pub finalized: bool,
    pub proposal_bump: u8,
    pub creation_timestamp: i64,
    pub vote_count: u32,
    pub index: u32,
    pub consensus_result: Option<Pubkey>,
    /// Slot number when the validator stake snapshot was taken
    pub snapshot_slot: u64,
    // Seeds for CPI
    pub proposal_seed: u64,
    pub vote_account_pubkey: Pubkey,
    /// Number of supporter entries stored in the account data at the fixed
    /// offset [`Proposal::SUPPORTERS_OFFSET`].
    ///
    /// The supporter vote-account pubkeys are intentionally NOT a field of
    /// this struct: they live past the struct's Borsh capacity boundary
    /// (`discriminator + INIT_SPACE`), so Anchor only (de)serializes this
    /// 4-byte count. The entries are read zero-copy via
    /// [`Proposal::supporters`] and written by [`Proposal::append_supporter`].
    pub num_supporters: u32,
}

/// The zero-copy supporter accessors reinterpret raw account bytes as
/// `[Pubkey]`; that is only sound for this exact layout.
const _: () = assert!(
    core::mem::size_of::<Pubkey>() == 32 && core::mem::align_of::<Pubkey>() == 1
);

impl Proposal {
    /// Byte offset of the first supporter entry in the account data. Ignores the
    /// actual serialization size of the `Proposal` for simplicity (`Option` and 
    /// `String` have variable length when serialized).
    pub const SUPPORTERS_OFFSET: usize = ANCHOR_DISCRIMINATOR + Self::INIT_SPACE;

    /// Zero-copy view of the supporter vote-account pubkeys. `data` must be
    /// this proposal's full account data (discriminator included).
    pub fn supporters<'d>(&self, data: &'d [u8]) -> Result<&'d [Pubkey]> {
        let len = self.num_supporters as usize;
        let bytes = data
            .get(Self::SUPPORTERS_OFFSET..Self::SUPPORTERS_OFFSET + len * core::mem::size_of::<Pubkey>())
            .ok_or(ProgramError::AccountDataTooSmall)?;
        // SAFETY: Pubkey is repr(transparent) over [u8; 32] with size 32 and
        // align 1 (compile-time-asserted above), so any correctly sized byte
        // region reinterprets as a [Pubkey] with no alignment or validity
        // requirements to violate; the lifetime stays tied to `data`.
        Ok(unsafe { core::slice::from_raw_parts(bytes.as_ptr().cast::<Pubkey>(), len) })
    }

    /// Writes one supporter entry after the existing ones and bumps
    /// `num_supporters`. The account must already have been realloc'd to hold
    /// the extra 32 bytes (see the constraint in `support_proposal`).
    pub fn append_supporter(&mut self, data: &mut [u8], vote_account: Pubkey) -> Result<()> {
        let entry_size = core::mem::size_of::<Pubkey>();
        let offset = Self::SUPPORTERS_OFFSET + self.num_supporters as usize * entry_size;
        data.get_mut(offset..offset + entry_size)
            .ok_or(ProgramError::AccountDataTooSmall)?
            .copy_from_slice(vote_account.as_ref());
        self.num_supporters += 1;
        Ok(())
    }
}

impl Default for Proposal {
    fn default() -> Self {
        Self {
            author: Pubkey::default(),
            title: "".to_string(),
            description: "".to_string(),
            creation_epoch: 0,
            start_epoch: 0,
            end_epoch: 0,
            proposer_stake_weight_bp: 0,
            cluster_support_lamports: 0,
            for_votes_lamports: 0,
            against_votes_lamports: 0,
            abstain_votes_lamports: 0,
            voting: false,
            finalized: false,
            proposal_bump: 0,
            creation_timestamp: 0,
            vote_count: 0,
            index: 0,
            snapshot_slot: 0,
            consensus_result: None,
            proposal_seed: 0,
            vote_account_pubkey: Pubkey::default(),
            num_supporters: 0,
        }
    }
}

impl Proposal {
    pub fn add_vote_lamports(
        &mut self,
        for_votes: u64,
        against_votes: u64,
        abstain_votes: u64,
    ) -> Result<()> {
        self.for_votes_lamports = self
            .for_votes_lamports
            .checked_add(for_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        self.against_votes_lamports = self
            .against_votes_lamports
            .checked_add(against_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        self.abstain_votes_lamports = self
            .abstain_votes_lamports
            .checked_add(abstain_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        Ok(())
    }

    pub fn sub_vote_lamports(
        &mut self,
        for_votes: u64,
        against_votes: u64,
        abstain_votes: u64,
    ) -> Result<()> {
        self.for_votes_lamports = self
            .for_votes_lamports
            .checked_sub(for_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        self.against_votes_lamports = self
            .against_votes_lamports
            .checked_sub(against_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        self.abstain_votes_lamports = self
            .abstain_votes_lamports
            .checked_sub(abstain_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        Ok(())
    }

    pub fn add_cluster_support(&mut self, support_lamports: u64) -> Result<()> {
        self.cluster_support_lamports = self
            .cluster_support_lamports
            .checked_add(support_lamports)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        Ok(())
    }
}
