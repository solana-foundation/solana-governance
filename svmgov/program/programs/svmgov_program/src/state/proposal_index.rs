use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ProposalIndex {
    pub current_index: u32,
    pub bump: u8,
}
