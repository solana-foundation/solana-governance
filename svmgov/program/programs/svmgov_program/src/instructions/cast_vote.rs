use anchor_lang::{
    prelude::*,
    solana_program::vote::{program as vote_program, state::VoteState},
};

use crate::{
    calculate_vote_lamports,
    constants::*,
    error::GovernanceError,
    events::VoteCast,
    merkle_helpers::verify_merkle_proof_cpi,
    state::{Proposal, Vote, VoteOverrideCache},
};
use gov_v1::{ConsensusResult, MetaMerkleProof};

#[derive(Accounts)]
pub struct CastVote<'info> {
    #[account(mut)]
    pub signer: Signer<'info>, // Voter (validator)
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    #[account(
        init,
        payer = signer,
        space = ANCHOR_DISCRIMINATOR + Vote::INIT_SPACE,
        seeds = [b"vote", proposal.key().as_ref(), spl_vote_account.key().as_ref()],
        bump
    )]
    pub vote: Account<'info, Vote>, // New vote account
    /// CHECK: Vote account is too big to deserialize, so we check on owner and size, then compare node_pubkey with signer
    #[account(
        constraint = spl_vote_account.owner == &vote_program::ID @ ProgramError::InvalidAccountOwner,
        constraint = spl_vote_account.data_len() == VoteState::size_of() @ GovernanceError::InvalidVoteAccountSize
    )]
    pub spl_vote_account: UncheckedAccount<'info>,
    /// CHECK: Vote override cache account. Might not yet exist
    #[account(
        mut,
        seeds = [b"vote_override_cache", proposal.key().as_ref(), vote.key().as_ref()],
        bump
    )]
    pub vote_override_cache: UncheckedAccount<'info>,
    /// CHECK: The snapshot program (gov-v1 or mock)
    // #[account(constraint = snapshot_program.key() == gov_v1::ID @ GovernanceError::InvalidSnapshotProgram)]
    pub snapshot_program: UncheckedAccount<'info>,
    /// CHECK: Consensus result account owned by snapshot program
    pub consensus_result: UncheckedAccount<'info>,
    /// CHECK: Meta merkle proof account owned by snapshot program
    pub meta_merkle_proof: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> CastVote<'info> {
    pub fn cast_vote(
        &mut self,
        for_votes_bp: u64,
        against_votes_bp: u64,
        abstain_votes_bp: u64,
        bumps: &CastVoteBumps,
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

        // Crosscheck consensus result
        require_eq!(
            meta_merkle_proof.consensus_result,
            self.consensus_result.key(),
            GovernanceError::InvalidConsensusResultPDA
        );

        // Ensure leaf matches signer and has sufficient stake
        require_eq!(
            meta_merkle_leaf.voting_wallet,
            self.signer.key(),
            GovernanceError::InvalidVoteAccount
        );
        require_gt!(
            meta_merkle_leaf.active_stake,
            0u64,
            GovernanceError::NotEnoughStake
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
            None,
            None,
        )?;

        // Calculate effective votes for each category based on actual lamports
        let voter_stake = meta_merkle_leaf.active_stake;
        let for_votes_lamports = calculate_vote_lamports!(voter_stake, for_votes_bp)?;
        let against_votes_lamports = calculate_vote_lamports!(voter_stake, against_votes_bp)?;
        let abstain_votes_lamports = calculate_vote_lamports!(voter_stake, abstain_votes_bp)?;

        // Check if override cache PDA exists and has been initialized
        // If it does, apply cached delegator votes
        if self.vote_override_cache.data_len() > 0 && self.vote_override_cache.owner == &crate::ID {
            let override_cache: VoteOverrideCache =
                match anchor_lang::AccountDeserialize::try_deserialize(
                    &mut self.vote_override_cache.data.borrow().as_ref(),
                ) {
                    Ok(cache) => cache,
                    Err(_) => {
                        // Account exists but is not a valid VoteOverrideCache - treat as non-existent
                        return Err(GovernanceError::InvalidVoteOverrideCache.into());
                    }
                };

            // Add cached votes
            self.proposal.add_vote_lamports(
                override_cache.for_votes_lamports,
                override_cache.against_votes_lamports,
                override_cache.abstain_votes_lamports,
            )?;

            let new_validator_stake = voter_stake
                .checked_sub(override_cache.total_stake)
                .ok_or(GovernanceError::ArithmeticOverflow)?;

            // Calculate new validator votes for each category based on actual lamports
            let for_votes_lamports_new =
                calculate_vote_lamports!(new_validator_stake, for_votes_bp)?;
            let against_votes_lamports_new =
                calculate_vote_lamports!(new_validator_stake, against_votes_bp)?;
            let abstain_votes_lamports_new =
                calculate_vote_lamports!(new_validator_stake, abstain_votes_bp)?;

            // Add validator's reduced votes to proposal
            self.proposal.add_vote_lamports(
                for_votes_lamports_new,
                against_votes_lamports_new,
                abstain_votes_lamports_new,
            )?;

            // Initialize the Vote PDA with all required fields
            self.vote.set_inner(Vote {
                validator: self.signer.key(),
                proposal: self.proposal.key(),
                for_votes_bp,
                against_votes_bp,
                abstain_votes_bp,
                for_votes_lamports: for_votes_lamports_new,
                against_votes_lamports: against_votes_lamports_new,
                abstain_votes_lamports: abstain_votes_lamports_new,
                override_lamports: override_cache.total_stake,
                stake: voter_stake,
                vote_timestamp: clock.unix_timestamp,
                bump: bumps.vote,
            });

            // Emit the missing VoteCast event
            emit!(VoteCast {
                proposal_id: self.proposal.key(),
                voter: self.signer.key(),
                vote_account: self.spl_vote_account.key(),
                for_votes_bp,
                against_votes_bp,
                abstain_votes_bp,
                for_votes_lamports: for_votes_lamports_new,
                against_votes_lamports: against_votes_lamports_new,
                abstain_votes_lamports: abstain_votes_lamports_new,
                vote_timestamp: clock.unix_timestamp,
            });
        } else {
            self.proposal.add_vote_lamports(
                for_votes_lamports,
                against_votes_lamports,
                abstain_votes_lamports,
            )?;

            // Store the vote distribution in the Vote PDA
            self.vote.set_inner(Vote {
                validator: self.signer.key(),
                proposal: self.proposal.key(),
                for_votes_bp,
                against_votes_bp,
                abstain_votes_bp,
                for_votes_lamports,
                against_votes_lamports,
                abstain_votes_lamports,
                override_lamports: 0,
                stake: voter_stake,
                vote_timestamp: clock.unix_timestamp,
                bump: bumps.vote,
            });

            // Emit vote cast event
            emit!(VoteCast {
                proposal_id: self.proposal.key(),
                voter: self.signer.key(),
                vote_account: self.spl_vote_account.key(),
                for_votes_bp,
                against_votes_bp,
                abstain_votes_bp,
                for_votes_lamports,
                against_votes_lamports,
                abstain_votes_lamports,
                vote_timestamp: clock.unix_timestamp,
            });
        }

        self.proposal.vote_count += 1;

        Ok(())
    }
}
