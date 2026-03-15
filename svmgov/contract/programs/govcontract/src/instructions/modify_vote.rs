use anchor_lang::{
    prelude::*,
    solana_program::vote::{program as vote_program, state::VoteState},
};

use crate::{
    calculate_vote_lamports,
    constants::*,
    error::GovernanceError,
    events::VoteModified,
    merkle_helpers::verify_merkle_proof_cpi,
    state::{Proposal, Vote},
};
use gov_v1::{ConsensusResult, MetaMerkleProof};

#[derive(Accounts)]
pub struct ModifyVote<'info> {
    pub signer: Signer<'info>, // Voter (validator)
    #[account(mut)]
    pub proposal: Account<'info, Proposal>, // Proposal being modified
    #[account(
        mut,
        seeds = [b"vote", proposal.key().as_ref(), spl_vote_account.key().as_ref()],
        bump = vote.bump,
    )]
    pub vote: Account<'info, Vote>, // Existing vote to modify
    /// CHECK: Vote account is too big to deserialize, so we check on owner and size, then compare node_pubkey with signer
    #[account(
        constraint = spl_vote_account.owner == &vote_program::ID @ ProgramError::InvalidAccountOwner,
        constraint = spl_vote_account.data_len() == VoteState::size_of() @ GovernanceError::InvalidVoteAccountSize
    )]
    pub spl_vote_account: UncheckedAccount<'info>,
    /// CHECK: The snapshot program (gov-v1 or mock)
    // #[account(constraint = snapshot_program.key() == gov_v1::ID @ GovernanceError::InvalidSnapshotProgram)]
    pub snapshot_program: UncheckedAccount<'info>,
    /// CHECK: Consensus result account owned by snapshot program
    pub consensus_result: UncheckedAccount<'info>,
    /// CHECK: Meta merkle proof account owned by snapshot program
    pub meta_merkle_proof: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>, // For account operations
}

impl<'info> ModifyVote<'info> {
    pub fn modify_vote(
        &mut self,
        for_votes_bp: u64,
        against_votes_bp: u64,
        abstain_votes_bp: u64,
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
        // Ensure the proof's vote_account matches the provided SPL vote account
        require_eq!(
            meta_merkle_leaf.vote_account,
            self.spl_vote_account.key(),
            GovernanceError::InvalidVoteAccount
        );
        require_gt!(
            meta_merkle_leaf.active_stake,
            0u64,
            GovernanceError::NotEnoughStake
        );

        verify_merkle_proof_cpi(
            &self.meta_merkle_proof.to_account_info(),
            &self.consensus_result.to_account_info(),
            &self.snapshot_program.to_account_info(),
            None,
            None,
        )?;

        // Subtract old lamports from proposal totals
        self.proposal.sub_vote_lamports(
            self.vote.for_votes_lamports,
            self.vote.against_votes_lamports,
            self.vote.abstain_votes_lamports,
        )?;

        // Calculate new effective votes for each category based on actual lamports
        let full_validator_stake = meta_merkle_leaf.active_stake;
        let voter_stake = full_validator_stake
            .checked_sub(self.vote.override_lamports)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        let for_votes_lamports = calculate_vote_lamports!(voter_stake, for_votes_bp)?;
        let against_votes_lamports = calculate_vote_lamports!(voter_stake, against_votes_bp)?;
        let abstain_votes_lamports = calculate_vote_lamports!(voter_stake, abstain_votes_bp)?;

        // Add new lamports to proposal totals
        self.proposal.add_vote_lamports(
            for_votes_lamports,
            against_votes_lamports,
            abstain_votes_lamports,
        )?;

        emit!(VoteModified {
            proposal_id: self.proposal.key(),
            voter: self.signer.key(),
            vote_account: self.spl_vote_account.key(),
            old_for_votes_bp: self.vote.for_votes_bp,
            old_against_votes_bp: self.vote.against_votes_bp,
            old_abstain_votes_bp: self.vote.abstain_votes_bp,
            new_for_votes_bp: for_votes_bp,
            new_against_votes_bp: against_votes_bp,
            new_abstain_votes_bp: abstain_votes_bp,
            for_votes_lamports,
            against_votes_lamports,
            abstain_votes_lamports,
            modification_timestamp: clock.unix_timestamp,
        });

        // Update the vote account with new distribution and lamports
        self.vote.for_votes_bp = for_votes_bp;
        self.vote.against_votes_bp = against_votes_bp;
        self.vote.abstain_votes_bp = abstain_votes_bp;
        self.vote.for_votes_lamports = for_votes_lamports;
        self.vote.against_votes_lamports = against_votes_lamports;
        self.vote.abstain_votes_lamports = abstain_votes_lamports;
        self.vote.vote_timestamp = clock.unix_timestamp;

        Ok(())
    }
}
