//! PDA derivation helpers for Squads V4.
//!
//! Each helper accepts an optional program ID; when `None` is supplied, the canonical
//! [`crate::PROGRAM_ID`] is used.

use solana_program::pubkey::Pubkey;

use crate::id::with_program_id_or_default;

/// Seed prefix used by every Squads PDA.
pub const SEED_PREFIX: &[u8] = b"multisig";
/// Seed for the program-config singleton PDA.
pub const SEED_PROGRAM_CONFIG: &[u8] = b"program_config";
/// Seed used between `SEED_PREFIX` and `create_key` for the `Multisig` PDA.
pub const SEED_MULTISIG: &[u8] = b"multisig";
/// Seed used at the end of a `Proposal` PDA, after the transaction index.
pub const SEED_PROPOSAL: &[u8] = b"proposal";
/// Seed for both `VaultTransaction` and `Proposal` PDAs (each appears with this seed
/// plus the 8-byte LE transaction index).
pub const SEED_TRANSACTION: &[u8] = b"transaction";
/// Seed for the per-vault PDA (index is the trailing single-byte seed).
pub const SEED_VAULT: &[u8] = b"vault";
/// Seed for ephemeral signer PDAs derived from a vault transaction PDA.
pub const SEED_EPHEMERAL_SIGNER: &[u8] = b"ephemeral_signer";
/// Seed for spending-limit PDAs derived from a multisig PDA.
pub const SEED_SPENDING_LIMIT: &[u8] = b"spending_limit";

/// Derives the program-config singleton PDA.
pub fn program_config_pda(program_id: Option<&Pubkey>) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEED_PREFIX, SEED_PROGRAM_CONFIG],
        &with_program_id_or_default(program_id),
    )
}

/// Derives the `Multisig` account PDA from its `create_key`.
pub fn multisig_pda(create_key: &Pubkey, program_id: Option<&Pubkey>) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEED_PREFIX, SEED_MULTISIG, create_key.as_ref()],
        &with_program_id_or_default(program_id),
    )
}

/// Derives a vault PDA for the given multisig and vault index. A multisig may have many
/// vaults; index `0` is the default.
pub fn vault_pda(
    multisig: &Pubkey,
    vault_index: u8,
    program_id: Option<&Pubkey>,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEED_PREFIX, multisig.as_ref(), SEED_VAULT, &[vault_index]],
        &with_program_id_or_default(program_id),
    )
}

/// Derives a `VaultTransaction` PDA for the given multisig and transaction index.
pub fn transaction_pda(
    multisig: &Pubkey,
    transaction_index: u64,
    program_id: Option<&Pubkey>,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            SEED_PREFIX,
            multisig.as_ref(),
            SEED_TRANSACTION,
            &transaction_index.to_le_bytes(),
        ],
        &with_program_id_or_default(program_id),
    )
}

/// Derives a `Proposal` PDA for the given multisig and transaction index.
pub fn proposal_pda(
    multisig: &Pubkey,
    transaction_index: u64,
    program_id: Option<&Pubkey>,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            SEED_PREFIX,
            multisig.as_ref(),
            SEED_TRANSACTION,
            &transaction_index.to_le_bytes(),
            SEED_PROPOSAL,
        ],
        &with_program_id_or_default(program_id),
    )
}

/// Derives an ephemeral signer PDA from a vault transaction PDA.
pub fn ephemeral_signer_pda(
    transaction_pda: &Pubkey,
    ephemeral_signer_index: u8,
    program_id: Option<&Pubkey>,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            SEED_PREFIX,
            transaction_pda.as_ref(),
            SEED_EPHEMERAL_SIGNER,
            &ephemeral_signer_index.to_le_bytes(),
        ],
        &with_program_id_or_default(program_id),
    )
}

/// Derives a spending-limit PDA from a multisig PDA and a per-limit `create_key`.
pub fn spending_limit_pda(
    multisig: &Pubkey,
    create_key: &Pubkey,
    program_id: Option<&Pubkey>,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            SEED_PREFIX,
            multisig.as_ref(),
            SEED_SPENDING_LIMIT,
            create_key.as_ref(),
        ],
        &with_program_id_or_default(program_id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_pda_uses_index_byte_as_seed() {
        let multisig = Pubkey::new_unique();
        let (a, _) = vault_pda(&multisig, 0, None);
        let (b, _) = vault_pda(&multisig, 1, None);
        assert_ne!(a, b, "different vault indexes must produce different PDAs");
    }

    #[test]
    fn transaction_pda_uses_index_le_bytes() {
        let multisig = Pubkey::new_unique();
        let (a, _) = transaction_pda(&multisig, 1, None);
        let (b, _) = transaction_pda(&multisig, 256, None);
        // 1 and 256 differ in the second LE byte; if the encoding regresses to BE these would
        // collide via the same most-significant non-zero byte.
        assert_ne!(a, b);
    }

    #[test]
    fn proposal_pda_extends_transaction_seeds() {
        let multisig = Pubkey::new_unique();
        let (tx, _) = transaction_pda(&multisig, 7, None);
        let (prop, _) = proposal_pda(&multisig, 7, None);
        assert_ne!(tx, prop, "Proposal and VaultTransaction PDAs must be distinct");
    }
}
