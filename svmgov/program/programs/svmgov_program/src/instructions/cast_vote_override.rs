use anchor_lang::{
    prelude::*,
    solana_program::{
        stake::program as stake_program,
        vote::{program as vote_program, state::VoteState},
    },
};

use crate::{
    calculate_vote_lamports,
    constants::*,
    error::GovernanceError,
    events::VoteOverrideCast,
    merkle_helpers::verify_merkle_proof_cpi,
    state::{Proposal, Vote, VoteOverride, VoteOverrideCache},
};
use gov_v1::{ConsensusResult, MetaMerkleProof, StakeMerkleLeaf};

#[derive(Accounts)]
pub struct CastVoteOverride<'info> {
    #[account(mut)]
    pub signer: Signer<'info>, // Voter (staker/delegator)
    #[account(mut)]
    pub proposal: Account<'info, Proposal>, // Proposal being voted on
    /// CHECK: Validator vote account. Might not yet exist
    #[account(
        mut,
        seeds = [b"vote", proposal.key().as_ref(), spl_vote_account.key.as_ref()],
        bump,
    )]
    pub validator_vote: UncheckedAccount<'info>, // Validator's existing vote (if any)
    /// CHECK: Vote account is too big to deserialize, so we check on owner and size, then compare node_pubkey with signer
    #[account(
        constraint = spl_vote_account.owner == &vote_program::ID @ ProgramError::InvalidAccountOwner,
        constraint = spl_vote_account.data_len() == VoteState::size_of() @ GovernanceError::InvalidVoteAccountSize
    )]
    pub spl_vote_account: UncheckedAccount<'info>,
    #[account(
        init,
        payer = signer,
        space = ANCHOR_DISCRIMINATOR + VoteOverride::INIT_SPACE,
        seeds = [b"vote_override", proposal.key().as_ref(), spl_stake_account.key.as_ref(), validator_vote.key().as_ref()],
        bump
    )]
    pub vote_override: Account<'info, VoteOverride>, // New override account
    /// CHECK: Vote override cache account - using init_if_needed to prevent prefunding DoS
    #[account(
        init_if_needed,
        payer = signer,
        space = 8 + VoteOverrideCache::INIT_SPACE,
        seeds = [b"vote_override_cache", proposal.key().as_ref(), validator_vote.key().as_ref()],
        bump
    )]
    pub vote_override_cache: Account<'info, VoteOverrideCache>,
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

impl<'info> CastVoteOverride<'info> {
    pub fn cast_vote_override(
        &mut self,
        for_votes_bp: u64,
        against_votes_bp: u64,
        abstain_votes_bp: u64,
        stake_merkle_proof: Vec<[u8; 32]>,
        stake_merkle_leaf: StakeMerkleLeaf,
        bumps: &CastVoteOverrideBumps,
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

        require!(
            self.proposal.consensus_result.is_some(),
            GovernanceError::ConsensusResultNotSet
        );

        // unwrap is safe because we checked that the consensus result is set in the previous require
        require_keys_eq!(
            self.proposal.consensus_result.unwrap(),
            self.consensus_result.key(),
            GovernanceError::InvalidConsensusResultPDA
        );
        let consensus_result_data = self.consensus_result.try_borrow_data()?;
        let consensus_result = ConsensusResult::try_deserialize(&mut &consensus_result_data[..])?;

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

        // Calculate delegator's vote lamports
        let for_votes_lamports = calculate_vote_lamports!(delegator_stake, for_votes_bp)?;
        let against_votes_lamports = calculate_vote_lamports!(delegator_stake, against_votes_bp)?;
        let abstain_votes_lamports = calculate_vote_lamports!(delegator_stake, abstain_votes_bp)?;

        // Check that validator vote exists
        // If account does not exist, cache the delegator's vote in override PDA

        if self.validator_vote.data_len() > 0 && self.validator_vote.owner == &crate::ID {
            // Attempt to deserialize the validator vote account
            let mut validator_vote: Vote = match anchor_lang::AccountDeserialize::try_deserialize(
                &mut self.validator_vote.data.borrow().as_ref(),
            ) {
                Ok(vote) => vote,
                Err(_) => {
                    // Account exists but is not a valid Vote - treat as non-existent
                    // Fall through to cache path below
                    return Err(GovernanceError::InvalidVoteAccount.into());
                }
            };

            // Subtract validator's vote
            self.proposal.sub_vote_lamports(
                validator_vote.for_votes_lamports,
                validator_vote.against_votes_lamports,
                validator_vote.abstain_votes_lamports,
            )?;

            // Add delegator's vote
            self.proposal.add_vote_lamports(
                for_votes_lamports,
                against_votes_lamports,
                abstain_votes_lamports,
            )?;

            // Calculate total overridden stake (current delegator + previously overridden delegators)
            let total_overridden = delegator_stake
                .checked_add(validator_vote.override_lamports)
                .ok_or(GovernanceError::ArithmeticOverflow)?;
            let new_validator_stake = validator_stake
                .checked_sub(total_overridden)
                .ok_or(GovernanceError::ArithmeticOverflow)?;

            // Calculate new validator votes for each category based on actual lamports
            let for_votes_lamports_new =
                calculate_vote_lamports!(new_validator_stake, validator_vote.for_votes_bp)?;
            let against_votes_lamports_new =
                calculate_vote_lamports!(new_validator_stake, validator_vote.against_votes_bp)?;
            let abstain_votes_lamports_new =
                calculate_vote_lamports!(new_validator_stake, validator_vote.abstain_votes_bp)?;

            // Add validator's new vote
            self.proposal.add_vote_lamports(
                for_votes_lamports_new,
                against_votes_lamports_new,
                abstain_votes_lamports_new,
            )?;

            // Store ONLY validator's reduced votes (not including delegator override)
            // The delegator's votes are already added to proposal totals separately
            validator_vote.for_votes_lamports = for_votes_lamports_new;
            validator_vote.against_votes_lamports = against_votes_lamports_new;
            validator_vote.abstain_votes_lamports = abstain_votes_lamports_new;
            validator_vote.override_lamports = validator_vote
                .override_lamports
                .checked_add(delegator_stake)
                .ok_or(GovernanceError::ArithmeticOverflow)?;

            // Serialize the updated validator vote back to the account
            anchor_lang::AccountSerialize::try_serialize(
                &validator_vote,
                &mut self.validator_vote.data.borrow_mut().as_mut(),
            )?;

            // Store override
            self.vote_override.set_inner(VoteOverride {
                delegator: self.signer.key(),
                stake_account: stake_merkle_leaf.stake_account,
                validator: meta_merkle_leaf.vote_account,
                proposal: self.proposal.key(),
                vote_account_validator: self.validator_vote.key(),
                for_votes_bp,
                against_votes_bp,
                abstain_votes_bp,
                stake_amount: delegator_stake,
                vote_override_timestamp: clock.unix_timestamp,
                bump: bumps.vote_override,
                for_votes_lamports,
                against_votes_lamports,
                abstain_votes_lamports,
            });
            if self.vote_override_cache.total_stake == 0 {
                // First override for this validator - initialize cache
                self.vote_override_cache.set_inner(VoteOverrideCache {
                    validator: meta_merkle_leaf.vote_account,
                    proposal: self.proposal.key(),
                    vote_account_validator: self.validator_vote.key(),
                    for_votes_bp,
                    against_votes_bp,
                    abstain_votes_bp,
                    for_votes_lamports,
                    against_votes_lamports,
                    abstain_votes_lamports,
                    total_stake: delegator_stake,
                    bump: bumps.vote_override_cache,
                });
            } else {
                // Subsequent override for this validator - update cache
                require_eq!(
                    self.vote_override_cache.proposal,
                    self.proposal.key(),
                    GovernanceError::InvalidVoteAccount
                );
                require_eq!(
                    self.vote_override_cache.vote_account_validator,
                    self.validator_vote.key(),
                    GovernanceError::InvalidVoteAccount
                );

                self.vote_override_cache.for_votes_bp = self
                    .vote_override_cache
                    .for_votes_bp
                    .checked_add(for_votes_bp)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;
                self.vote_override_cache.against_votes_bp = self
                    .vote_override_cache
                    .against_votes_bp
                    .checked_add(against_votes_bp)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;
                self.vote_override_cache.abstain_votes_bp = self
                    .vote_override_cache
                    .abstain_votes_bp
                    .checked_add(abstain_votes_bp)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;

                self.vote_override_cache.for_votes_lamports = self
                    .vote_override_cache
                    .for_votes_lamports
                    .checked_add(for_votes_lamports)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;
                self.vote_override_cache.against_votes_lamports = self
                    .vote_override_cache
                    .against_votes_lamports
                    .checked_add(against_votes_lamports)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;
                self.vote_override_cache.abstain_votes_lamports = self
                    .vote_override_cache
                    .abstain_votes_lamports
                    .checked_add(abstain_votes_lamports)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;

                self.vote_override_cache.total_stake = self
                    .vote_override_cache
                    .total_stake
                    .checked_add(delegator_stake)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;
            }
        } else {
            // validator has no vote yet, so just store delegator's vote in override PDA
            // Path 2a: nobody -> delegator (first delegator to override)
            // Path 2b: delegator -> delegator (multiple delegators override same validator)

            // With Anchor's init_if_needed, the account is automatically created if it doesn't exist
            // or reused if it already exists. We just need to check if we're updating an existing cache.

            // Check if this is the first delegator (cache is empty) or subsequent delegator
            if self.vote_override_cache.total_stake == 0 {
                // Path 2a: First delegator - initialize cache
                self.vote_override_cache.set_inner(VoteOverrideCache {
                    validator: meta_merkle_leaf.vote_account,
                    proposal: self.proposal.key(),
                    vote_account_validator: self.validator_vote.key(),
                    for_votes_bp,
                    against_votes_bp,
                    abstain_votes_bp,
                    for_votes_lamports,
                    against_votes_lamports,
                    abstain_votes_lamports,
                    total_stake: delegator_stake,
                    bump: bumps.vote_override_cache,
                });
            } else {
                // Path 2b: Subsequent delegator - update cache
                // Validate cache is for the same validator and proposal
                require_eq!(
                    self.vote_override_cache.proposal,
                    self.proposal.key(),
                    GovernanceError::InvalidVoteAccount
                );
                require_eq!(
                    self.vote_override_cache.vote_account_validator,
                    self.validator_vote.key(),
                    GovernanceError::InvalidVoteAccount
                );

                // Update cache by adding delegator's vote
                self.vote_override_cache.for_votes_bp = self
                    .vote_override_cache
                    .for_votes_bp
                    .checked_add(for_votes_bp)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;
                self.vote_override_cache.against_votes_bp = self
                    .vote_override_cache
                    .against_votes_bp
                    .checked_add(against_votes_bp)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;
                self.vote_override_cache.abstain_votes_bp = self
                    .vote_override_cache
                    .abstain_votes_bp
                    .checked_add(abstain_votes_bp)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;

                self.vote_override_cache.for_votes_lamports = self
                    .vote_override_cache
                    .for_votes_lamports
                    .checked_add(for_votes_lamports)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;
                self.vote_override_cache.against_votes_lamports = self
                    .vote_override_cache
                    .against_votes_lamports
                    .checked_add(against_votes_lamports)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;
                self.vote_override_cache.abstain_votes_lamports = self
                    .vote_override_cache
                    .abstain_votes_lamports
                    .checked_add(abstain_votes_lamports)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;

                self.vote_override_cache.total_stake = self
                    .vote_override_cache
                    .total_stake
                    .checked_add(delegator_stake)
                    .ok_or(GovernanceError::ArithmeticOverflow)?;
            }

            // Initialize the VoteOverride account with delegator's vote data
            // This covers both cases: delegator -> delegator and nobody -> delegator
            self.vote_override.set_inner(VoteOverride {
                delegator: self.signer.key(),
                stake_account: stake_merkle_leaf.stake_account,
                validator: meta_merkle_leaf.vote_account,
                proposal: self.proposal.key(),
                vote_account_validator: self.validator_vote.key(),
                for_votes_bp,
                against_votes_bp,
                abstain_votes_bp,
                stake_amount: delegator_stake,
                vote_override_timestamp: clock.unix_timestamp,
                bump: bumps.vote_override,
                for_votes_lamports,
                against_votes_lamports,
                abstain_votes_lamports,
            });
        }

        // Emit vote override cast event
        emit!(VoteOverrideCast {
            proposal_id: self.proposal.key(),
            delegator: self.signer.key(),
            stake_account: stake_merkle_leaf.stake_account,
            validator: meta_merkle_leaf.vote_account,
            for_votes_bp,
            against_votes_bp,
            abstain_votes_bp,
            for_votes_lamports,
            against_votes_lamports,
            abstain_votes_lamports,
            stake_amount: delegator_stake,
            vote_timestamp: clock.unix_timestamp,
        });

        self.proposal.vote_count += 1;

        Ok(())
    }
}
