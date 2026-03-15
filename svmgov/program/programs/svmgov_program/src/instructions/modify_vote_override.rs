use anchor_lang::{
    prelude::*,
    solana_program::{
        stake::program as stake_program,
        vote::{program as vote_program, state::VoteState},
    },
};
use gov_v1::{ConsensusResult, MetaMerkleProof, StakeMerkleLeaf};

use crate::{
    calculate_vote_lamports,
    constants::*,
    error::GovernanceError,
    events::VoteOverrideModified,
    merkle_helpers::verify_merkle_proof_cpi,
    state::{Proposal, Vote, VoteOverride, VoteOverrideCache},
};

#[derive(Accounts)]
pub struct ModifyVoteOverride<'info> {
    pub signer: Signer<'info>, // Voter (staker/delegator)
    #[account(mut)]
    pub proposal: Account<'info, Proposal>, // Proposal being voted on
    /// CHECK: Validator vote account. Must exist for modification
    #[account(
        mut,
        seeds = [b"vote", proposal.key().as_ref(), spl_vote_account.key.as_ref()],
        bump,
    )]
    pub validator_vote: UncheckedAccount<'info>, // Validator's existing vote
    /// CHECK: Vote account is too big to deserialize, so we check on owner and size, then compare node_pubkey with signer
    #[account(
        constraint = spl_vote_account.owner == &vote_program::ID @ ProgramError::InvalidAccountOwner,
        constraint = spl_vote_account.data_len() == VoteState::size_of() @ GovernanceError::InvalidVoteAccountSize
    )]
    pub spl_vote_account: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"vote_override", proposal.key().as_ref(), spl_stake_account.key.as_ref(), validator_vote.key().as_ref()],
        bump = vote_override.bump
    )]
    pub vote_override: Account<'info, VoteOverride>, // Existing override account to modify
    /// CHECK: Vote override cache account. Might not yet exist
    #[account(
        mut,
        seeds = [b"vote_override_cache", proposal.key().as_ref(), validator_vote.key().as_ref()],
        bump
    )]
    pub vote_override_cache: UncheckedAccount<'info>,
    /// CHECK: stake account for override
    #[account(
        constraint = spl_stake_account.owner == &stake_program::ID @ ProgramError::InvalidAccountOwner,
    )]
    pub spl_stake_account: UncheckedAccount<'info>,
    /// CHECK: The snapshot program (gov-v1 or mock)
    // #[account(constraint = snapshot_program.key() == gov_v1::ID @ GovernanceError::InvalidSnapshotProgram)]
    pub snapshot_program: UncheckedAccount<'info>,
    /// CHECK: Consensus result account owned by snapshot program
    pub consensus_result: UncheckedAccount<'info>,
    /// CHECK: Meta merkle proof account owned by snapshot program
    pub meta_merkle_proof: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> ModifyVoteOverride<'info> {
    pub fn modify_vote_override(
        &mut self,
        for_votes_bp: u64,
        against_votes_bp: u64,
        abstain_votes_bp: u64,
        stake_merkle_proof: Vec<[u8; 32]>,
        stake_merkle_leaf: StakeMerkleLeaf,
        bumps: &ModifyVoteOverrideBumps,
    ) -> Result<()> {
        // Check that the proposal is open for voting
        require!(!self.proposal.finalized, GovernanceError::ProposalFinalized);

        // Get the current epoch from the Clock sysvar
        let clock = Clock::get()?;
        let current_epoch = clock.epoch;
        require!(
            self.proposal.start_epoch <= current_epoch,
            GovernanceError::VotingNotStarted
        );
        require!(
            current_epoch < self.proposal.end_epoch,
            GovernanceError::ProposalClosed
        );

        // Validate that the basis points sum to 10,000 (100%)
        let total_bp = for_votes_bp
            .checked_add(against_votes_bp)
            .and_then(|sum| sum.checked_add(abstain_votes_bp))
            .ok_or(GovernanceError::ArithmeticOverflow)?;
        require!(
            total_bp == BASIS_POINTS_MAX,
            GovernanceError::InvalidVoteDistribution
        );

        // Validate snapshot program ownership
        require!(
            self.consensus_result.owner == self.snapshot_program.key,
            GovernanceError::MustBeOwnedBySnapshotProgram
        );
        require!(
            self.meta_merkle_proof.owner == self.snapshot_program.key,
            GovernanceError::MustBeOwnedBySnapshotProgram
        );
        let consensus_result_data = self.consensus_result.try_borrow_data()?;
        let consensus_result = ConsensusResult::try_deserialize(&mut &consensus_result_data[..])?;

        require_keys_eq!(
            self.proposal.consensus_result.unwrap(),
            self.consensus_result.key(),
            GovernanceError::InvalidConsensusResultPDA
        );
        require!(
            consensus_result
                .ballot
                .meta_merkle_root
                .iter()
                .any(|&x| x != 0),
            GovernanceError::InvalidMerkleRoot
        );
        // Deserialize MetaMerkleProof for crosschecking
        let meta_account_data = self.meta_merkle_proof.try_borrow_data()?;
        let meta_merkle_proof = MetaMerkleProof::try_deserialize(&mut &meta_account_data[..])?;

        let meta_merkle_leaf = meta_merkle_proof.meta_merkle_leaf;

        require_eq!(
            meta_merkle_proof.consensus_result,
            self.consensus_result.key(),
            GovernanceError::InvalidConsensusResultPDA
        );

        require_eq!(
            stake_merkle_leaf.voting_wallet,
            self.signer.key(),
            GovernanceError::StakeAccountOwnerMismatch
        );

        require_gt!(
            stake_merkle_leaf.active_stake,
            0u64,
            GovernanceError::NotEnoughStake
        );

        // Ensure stake leaf contains the correct stake account
        require_eq!(
            stake_merkle_leaf.stake_account,
            self.spl_stake_account.key(),
            GovernanceError::InvalidStakeAccount
        );

        require_eq!(
            meta_merkle_leaf.vote_account,
            self.spl_vote_account.key(),
            GovernanceError::InvalidVoteAccount
        );

        // Verify that the override account belongs to this delegator and stake account
        require_eq!(
            self.vote_override.stake_account,
            stake_merkle_leaf.stake_account,
            GovernanceError::InvalidStakeAccount
        );

        verify_merkle_proof_cpi(
            &self.meta_merkle_proof.to_account_info(),
            &self.consensus_result.to_account_info(),
            &self.snapshot_program.to_account_info(),
            Some(stake_merkle_proof),
            Some(stake_merkle_leaf.clone()),
        )?;

        // Use verified stake amounts
        let delegator_stake = stake_merkle_leaf.active_stake;
        let validator_stake = meta_merkle_leaf.active_stake;

        // Store old values for event emission
        let old_for_votes_bp = self.vote_override.for_votes_bp;
        let old_against_votes_bp = self.vote_override.against_votes_bp;
        let old_abstain_votes_bp = self.vote_override.abstain_votes_bp;
        let old_for_votes_lamports = self.vote_override.for_votes_lamports;
        let old_against_votes_lamports = self.vote_override.against_votes_lamports;
        let old_abstain_votes_lamports = self.vote_override.abstain_votes_lamports;

        // Calculate delegator's new vote lamports
        let for_votes_lamports = calculate_vote_lamports!(delegator_stake, for_votes_bp)?;
        let against_votes_lamports = calculate_vote_lamports!(delegator_stake, against_votes_bp)?;
        let abstain_votes_lamports = calculate_vote_lamports!(delegator_stake, abstain_votes_bp)?;

        // Update the override account with new values
        self.vote_override.for_votes_bp = for_votes_bp;
        self.vote_override.against_votes_bp = against_votes_bp;
        self.vote_override.abstain_votes_bp = abstain_votes_bp;
        self.vote_override.for_votes_lamports = for_votes_lamports;
        self.vote_override.against_votes_lamports = against_votes_lamports;
        self.vote_override.abstain_votes_lamports = abstain_votes_lamports;
        self.vote_override.vote_override_timestamp = clock.unix_timestamp;

        if self.validator_vote.owner == &crate::ID
            && self.validator_vote.data_len() == (ANCHOR_DISCRIMINATOR + Vote::INIT_SPACE)
        {
            // Subtract old delegator's vote from proposal totals
            self.proposal.sub_vote_lamports(
                old_for_votes_lamports,
                old_against_votes_lamports,
                old_abstain_votes_lamports,
            )?;

            // Add new delegator's vote to proposal totals
            self.proposal.add_vote_lamports(
                for_votes_lamports,
                against_votes_lamports,
                abstain_votes_lamports,
            )?;
        } else {
            require!(
                self.vote_override_cache.owner == &crate::ID,
                GovernanceError::InvalidVoteAccount
            );
            // Update vote override cache if it exists
            // Use try_deserialize to properly handle discriminator
            let vote_override_cache_result: Result<VoteOverrideCache> =
                anchor_lang::AccountDeserialize::try_deserialize(
                    &mut self.vote_override_cache.data.borrow_mut().as_ref(),
                );
            if let Ok(mut vote_override_cache) = vote_override_cache_result {
                // Update cache by subtracting old values and adding new ones
                vote_override_cache.for_votes_lamports = vote_override_cache
                    .for_votes_lamports
                    .checked_sub(old_for_votes_lamports)
                    .and_then(|val| val.checked_add(for_votes_lamports))
                    .ok_or(GovernanceError::ArithmeticOverflow)?;

                vote_override_cache.against_votes_lamports = vote_override_cache
                    .against_votes_lamports
                    .checked_sub(old_against_votes_lamports)
                    .and_then(|val| val.checked_add(against_votes_lamports))
                    .ok_or(GovernanceError::ArithmeticOverflow)?;

                vote_override_cache.abstain_votes_lamports = vote_override_cache
                    .abstain_votes_lamports
                    .checked_sub(old_abstain_votes_lamports)
                    .and_then(|val| val.checked_add(abstain_votes_lamports))
                    .ok_or(GovernanceError::ArithmeticOverflow)?;

                // Serialize the updated cache back to the account data
                let mut cache_data = self.vote_override_cache.data.borrow_mut();
                anchor_lang::AccountSerialize::try_serialize(
                    &vote_override_cache,
                    &mut cache_data.as_mut(),
                )
                .map_err(|e| {
                    msg!("Error serializing VoteOverrideCache: {}", e);
                    GovernanceError::ArithmeticOverflow
                })?;
            }
        }

        // Emit vote override modified event
        emit!(VoteOverrideModified {
            proposal_id: self.proposal.key(),
            delegator: self.signer.key(),
            stake_account: stake_merkle_leaf.stake_account,
            validator: meta_merkle_leaf.vote_account,
            old_for_votes_bp,
            old_against_votes_bp,
            old_abstain_votes_bp,
            new_for_votes_bp: for_votes_bp,
            new_against_votes_bp: against_votes_bp,
            new_abstain_votes_bp: abstain_votes_bp,
            for_votes_lamports,
            against_votes_lamports,
            abstain_votes_lamports,
            stake_amount: delegator_stake,
            modification_timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}
