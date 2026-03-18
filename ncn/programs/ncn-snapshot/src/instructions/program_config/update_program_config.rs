use anchor_lang::prelude::*;

use crate::ProgramConfig;

#[derive(Accounts)]
pub struct UpdateProgramConfig<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = authority
    )]
    pub program_config: Box<Account<'info, ProgramConfig>>,
}

pub fn handler(
    ctx: Context<UpdateProgramConfig>,
    proposed_authority: Option<Pubkey>,
    min_consensus_threshold_bps: Option<u16>,
    tie_breaker_admin: Option<Pubkey>,
    vote_duration: Option<i64>,
) -> Result<()> {
    let program_config = &mut ctx.accounts.program_config;
    if let Some(proposed_authority) = proposed_authority {
        program_config.proposed_authority = Some(proposed_authority);
    }
    if let Some(min_consensus_threshold_bps) = min_consensus_threshold_bps {
        require_gt!(min_consensus_threshold_bps, 0);
        require_gte!(10000, min_consensus_threshold_bps);
        program_config.min_consensus_threshold_bps = min_consensus_threshold_bps;
    }
    if let Some(tie_breaker_admin) = tie_breaker_admin {
        program_config.tie_breaker_admin = tie_breaker_admin;
    }
    if let Some(vote_duration) = vote_duration {
        require_gt!(vote_duration, 0);
        program_config.vote_duration = vote_duration;
    }

    Ok(())
}
