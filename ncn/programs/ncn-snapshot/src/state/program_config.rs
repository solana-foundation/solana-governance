use crate::error::ErrorCode;
use std::collections::HashSet;

use anchor_lang::prelude::*;

pub const MAX_OPERATOR_WHITELIST: usize = 64;

#[derive(InitSpace, Debug)]
#[account]
pub struct ProgramConfig {
    /// Authority allowed to update the config.
    pub authority: Pubkey,
    /// Authority to be set to upon finalization of proposal.
    pub proposed_authority: Option<Pubkey>,
    /// Operators whitelisted to participate in voting.
    /// A snapshot of this list will be taken at the time of BallotBox creation.
    #[max_len(MAX_OPERATOR_WHITELIST)]
    pub whitelisted_operators: Vec<Pubkey>,
    /// Min. percentage of votes required to finalize a ballot. Used during BallotBox creation.
    pub min_consensus_threshold_bps: u16,
    /// Admin allowed to decide the winning ballot if vote expires before consensus.
    pub tie_breaker_admin: Pubkey,
    /// Duration for which ballot box will be opened for voting.
    pub vote_duration: i64,
}

impl ProgramConfig {
    pub fn pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"ProgramConfig"], &crate::ID)
    }

    pub fn remove_operators(&mut self, operators_to_remove: Option<Vec<Pubkey>>) {
        if let Some(operators) = operators_to_remove {
            let remove_set: HashSet<Pubkey> = operators.into_iter().collect();
            self.whitelisted_operators
                .retain(|op| !remove_set.contains(op));
        }
    }

    // Add operators to the whitelist. Duplicate operators are ignored.
    pub fn add_operators(&mut self, operators_to_add: Option<Vec<Pubkey>>) -> Result<()> {
        if let Some(new_operators) = operators_to_add {
            let mut existing_set: HashSet<Pubkey> =
                self.whitelisted_operators.iter().cloned().collect();
            for op in new_operators.into_iter() {
                if existing_set.insert(op) {
                    self.whitelisted_operators.push(op);
                }
            }
            require!(
                self.whitelisted_operators.len() <= MAX_OPERATOR_WHITELIST,
                ErrorCode::VecFull
            );
        }
        Ok(())
    }
}
