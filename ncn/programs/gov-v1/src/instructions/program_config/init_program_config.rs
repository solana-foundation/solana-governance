use anchor_lang::prelude::*;

use crate::ProgramConfig;

#[derive(Accounts)]
pub struct InitProgramConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    #[account(
        init,
        seeds = [b"ProgramConfig".as_ref()],
        bump,
        payer = payer,
        space = 8 + ProgramConfig::INIT_SPACE
    )]
    pub program_config: Box<Account<'info, ProgramConfig>>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitProgramConfig>) -> Result<()> {
    let program_config = &mut ctx.accounts.program_config;
    program_config.authority = ctx.accounts.authority.key();

    Ok(())
}
