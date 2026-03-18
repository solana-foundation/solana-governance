use anchor_lang::prelude::*;

use crate::Ballot;

#[account]
#[derive(InitSpace, Debug)]
pub struct ConsensusResult {
    /// Snapshot slot used for the ballot box
    pub snapshot_slot: u64,
    /// Ballot
    pub ballot: Ballot,
    /// Whether consensus was reached via tie breaker
    pub tie_breaker_consensus: bool,
}

impl ConsensusResult {
    pub fn pda(snapshot_slot: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"ConsensusResult", &snapshot_slot.to_le_bytes()],
            &crate::ID,
        )
    }
}
