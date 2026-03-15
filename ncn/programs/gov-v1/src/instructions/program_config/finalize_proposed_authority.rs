use anchor_lang::prelude::*;

use crate::ProgramConfig;

#[derive(Accounts)]
pub struct FinalizeProposedAuthority<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        constraint = program_config.proposed_authority.is_some() && program_config.proposed_authority.unwrap() == authority.key()
    )]
    pub program_config: Box<Account<'info, ProgramConfig>>,
}

pub fn handler(ctx: Context<FinalizeProposedAuthority>) -> Result<()> {
    let program_config = &mut ctx.accounts.program_config;
    program_config.authority = ctx.accounts.authority.key();
    program_config.proposed_authority = None;

    Ok(())
}
