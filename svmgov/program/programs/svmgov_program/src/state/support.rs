use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Support {
    pub proposal: Pubkey,
    pub validator: Pubkey,
    pub bump: u8,
}
