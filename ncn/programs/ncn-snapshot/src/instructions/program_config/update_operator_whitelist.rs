use anchor_lang::prelude::*;

use crate::{error::ErrorCode, ProgramConfig};

#[derive(Accounts)]
pub struct UpdateOperatorWhitelist<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = authority
    )]
    pub program_config: Box<Account<'info, ProgramConfig>>,
}

pub fn handler(
    ctx: Context<UpdateOperatorWhitelist>,
    operators_to_add: Option<Vec<Pubkey>>,
    operators_to_remove: Option<Vec<Pubkey>>,
) -> Result<()> {
    // Validate no overlap between add and remove lists.
    if let (Some(add), Some(remove)) = (&operators_to_add, &operators_to_remove) {
        let add_set: std::collections::HashSet<Pubkey> = add.iter().cloned().collect();
        let remove_set: std::collections::HashSet<Pubkey> = remove.iter().cloned().collect();
        let overlap = add_set.intersection(&remove_set).next().is_some();
        require!(!overlap, ErrorCode::OverlappingWhitelistEntries);
    }

    let program_config = &mut ctx.accounts.program_config;
    program_config.remove_operators(operators_to_remove);
    program_config.add_operators(operators_to_add)?;

    Ok(())
}
