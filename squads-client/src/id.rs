//! Program identifiers for the canonical Squads V4 multisig program.

use solana_program::pubkey::Pubkey;

/// Canonical mainnet/devnet program ID for the Squads V4 multisig program.
///
/// See the [Squads V4 README](https://github.com/Squads-Protocol/v4) for confirmation
/// and any alternative deployment IDs (e.g. Eclipse uses a different program ID).
pub const PROGRAM_ID: Pubkey =
    solana_program::pubkey!("SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf");

/// Returns the supplied program ID if `Some`, or the canonical [`PROGRAM_ID`] otherwise.
///
/// Useful for PDA derivation and instruction builders that accept an optional override.
#[inline]
pub fn with_program_id_or_default(program_id: Option<&Pubkey>) -> Pubkey {
    program_id.copied().unwrap_or(PROGRAM_ID)
}
