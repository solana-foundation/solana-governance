use anchor_lang::{prelude::*, solana_program::vote};

use crate::{
    error::GovernanceError, events::MerkleRootFlushed, state::{GlobalConfig, Proposal},
    utils::get_epoch_slot_range,
};

#[derive(Accounts)]
pub struct FlushMerkleRoot<'info> {
    #[account(mut)]
    pub signer: Signer<'info>, // Proposal author
    #[account(
        mut,
        constraint = proposal.author == signer.key() @ GovernanceError::Unauthorized,
        constraint = !proposal.finalized @ GovernanceError::ProposalFinalized,
    )]
    pub proposal: Account<'info, Proposal>,
    /// CHECK: Vote account is too big to deserialize, so we check on owner and size
    #[account(
        constraint = spl_vote_account.owner == &vote::program::ID @ ProgramError::InvalidAccountOwner,
    )]
    pub spl_vote_account: UncheckedAccount<'info>,
    /// CHECK: Ballot box account - may or may not exist, checked with data_is_empty()
    pub ballot_box: UncheckedAccount<'info>,
    /// CHECK: Ballot program account
    #[account(
        constraint = ballot_program.key == &ncn_snapshot::ID @ ProgramError::InvalidAccountOwner,
    )]
    pub ballot_program: UncheckedAccount<'info>,
    /// CHECK: Program config account
    #[account(
        seeds = [b"ProgramConfig"],
        bump,
        seeds::program = ballot_program.key(),
        constraint = program_config.owner == &ncn_snapshot::ID @ ProgramError::InvalidAccountOwner,
    )]
    pub program_config: UncheckedAccount<'info>,
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
    pub system_program: Program<'info, System>,
}

impl<'info> FlushMerkleRoot<'info> {
    pub fn flush_merkle_root(&mut self) -> Result<()> {
        let clock = Clock::get()?;

        // Prevent flushing once voting has started
        require!(
            clock.epoch < self.proposal.start_epoch,
            GovernanceError::CannotModifyAfterStart
        );

        // Clear the consensus_result
        require!(
            self.proposal.snapshot_slot > 0,
            GovernanceError::InvalidSnapshotSlot
        );
        require!(
            self.proposal.consensus_result.is_some(),
            GovernanceError::ConsensusResultNotSet
        );

        // Recalculate snapshot_slot based on current epoch
        // Using the same logic as in support_proposal
        let target_epoch = clock.epoch + self.global_config.snapshot_epoch_extension;
        let (start_slot, _) = get_epoch_slot_range(target_epoch);
        let offset_result = (start_slot as i64)
            .checked_add(self.global_config.snapshot_slot_offset)
            .ok_or(GovernanceError::ArithmeticOverflow)?;
        require!(offset_result >= 0, GovernanceError::ArithmeticOverflow);
        let snapshot_slot = offset_result as u64;
        self.proposal.snapshot_slot = snapshot_slot;
        // start voting 1 epoch after snapshot
        self.proposal.start_epoch = target_epoch + 1;
        self.proposal.end_epoch = target_epoch + 1 + self.global_config.voting_epochs;

        // Calculate new consensus_result PDA based on new snapshot_slot
        let (consensus_result_pda, _) = Pubkey::find_program_address(
            &[b"ConsensusResult", &snapshot_slot.to_le_bytes()],
            &self.ballot_program.key,
        );

        self.proposal.consensus_result = Some(consensus_result_pda);

        // Initialize ballot box if it doesn't exist
        if self.ballot_box.data_is_empty() {
            // Create seed components with sufficient lifetime
            let proposal_seed_val = self.proposal.proposal_seed.to_le_bytes();
            let vote_account_key = self.proposal.vote_account_pubkey.key();

            let seeds: &[&[u8]] = &[
                b"proposal".as_ref(),
                &proposal_seed_val,
                vote_account_key.as_ref(),
                &[self.proposal.proposal_bump],
            ];
            let signer = &[&seeds[..]];
            // Initialize the ballot box via CPI
            let cpi_ctx = CpiContext::new_with_signer(
                self.ballot_program.to_account_info(),
                ncn_snapshot::cpi::accounts::InitBallotBox {
                    payer: self.signer.to_account_info(),
                    proposal: self.proposal.to_account_info(),
                    ballot_box: self.ballot_box.to_account_info(),
                    program_config: self.program_config.to_account_info(),
                    system_program: self.system_program.to_account_info(),
                },
                signer,
            );

            ncn_snapshot::cpi::init_ballot_box(
                cpi_ctx,
                snapshot_slot,
                self.proposal.proposal_seed,
                self.spl_vote_account.key(),
            )?;
        }

        emit!(MerkleRootFlushed {
            proposal_id: self.proposal.key(),
            author: self.signer.key(),
            new_snapshot_slot: self.proposal.snapshot_slot,
            flush_timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}
