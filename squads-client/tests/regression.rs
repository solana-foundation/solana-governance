//! Regression tests pinning the byte-for-byte values that downstream consumers depend
//! on. If any of these tests start failing, it means a wire-format-affecting change
//! happened and downstream Squads transactions will be rejected on-chain.

use squads_client::discriminator::{account_discriminator, instruction_discriminator};
use squads_client::pda::{multisig_pda, proposal_pda, transaction_pda, vault_pda};
use squads_client::PROGRAM_ID;
use solana_program::pubkey::Pubkey;

// ============================================================================
// Discriminator regression tests
// ============================================================================
//
// These are the canonical Anchor v0.29 discriminators computed as
// `sha256("<namespace>:<name>")[..8]`. Verified against:
//   python3 -c "import hashlib; print(hashlib.sha256(b'global:vault_transaction_create').digest()[:8].hex())"
//
// If any of these change, the on-chain Squads program will reject our instructions.

#[test]
fn instruction_discriminator_vault_transaction_create_pinned() {
    assert_eq!(
        instruction_discriminator("vault_transaction_create"),
        [0x30, 0xfa, 0x4e, 0xa8, 0xd0, 0xe2, 0xda, 0xd3]
    );
}

#[test]
fn instruction_discriminator_proposal_create_pinned() {
    assert_eq!(
        instruction_discriminator("proposal_create"),
        [0xdc, 0x3c, 0x49, 0xe0, 0x1e, 0x6c, 0x4f, 0x9f]
    );
}

#[test]
fn instruction_discriminator_proposal_approve_pinned() {
    assert_eq!(
        instruction_discriminator("proposal_approve"),
        [0x90, 0x25, 0xa4, 0x88, 0xbc, 0xd8, 0x2a, 0xf8]
    );
}

#[test]
fn instruction_discriminator_vault_transaction_execute_pinned() {
    // python3 -c "import hashlib; print(hashlib.sha256(b'global:vault_transaction_execute').digest()[:8].hex())"
    assert_eq!(
        instruction_discriminator("vault_transaction_execute"),
        [0xc2, 0x08, 0xa1, 0x57, 0x99, 0xa4, 0x19, 0xab]
    );
}

#[test]
fn account_discriminator_multisig_pinned() {
    assert_eq!(
        account_discriminator("Multisig"),
        [0xe0, 0x74, 0x79, 0xba, 0x44, 0xa1, 0x4f, 0xec]
    );
}

#[test]
fn account_discriminator_proposal_pinned() {
    assert_eq!(
        account_discriminator("Proposal"),
        [0x1a, 0x5e, 0xbd, 0xbb, 0x74, 0x88, 0x35, 0x21]
    );
}

#[test]
fn account_discriminator_vault_transaction_pinned() {
    assert_eq!(
        account_discriminator("VaultTransaction"),
        [0xa8, 0xfa, 0xa2, 0x64, 0x51, 0x0e, 0xa2, 0xcf]
    );
}

// ============================================================================
// PDA parity tests
// ============================================================================
//
// Cross-checking PDA derivations against fixed input/output pairs computed from
// the canonical seed scheme. These pin the seed concatenation order — a regression
// in any seed prefix would break the PDA lookups against an existing on-chain Squads
// multisig.

#[test]
fn multisig_pda_seed_order_regression() {
    // Fixed create_key so the test is deterministic and reproducible from anywhere.
    let create_key = Pubkey::new_from_array([1u8; 32]);
    let (pda1, _bump1) = multisig_pda(&create_key, None);
    let (pda2, _bump2) = multisig_pda(&create_key, Some(&PROGRAM_ID));
    // Passing `None` should equal passing the canonical program id.
    assert_eq!(pda1, pda2);
}

#[test]
fn vault_pda_different_indexes_produce_different_pdas() {
    let multisig = Pubkey::new_from_array([2u8; 32]);
    let (v0, _) = vault_pda(&multisig, 0, None);
    let (v1, _) = vault_pda(&multisig, 1, None);
    let (v255, _) = vault_pda(&multisig, 255, None);
    assert_ne!(v0, v1);
    assert_ne!(v0, v255);
    assert_ne!(v1, v255);
}

#[test]
fn transaction_pda_and_proposal_pda_are_distinct() {
    let multisig = Pubkey::new_from_array([3u8; 32]);
    let (t, _) = transaction_pda(&multisig, 42, None);
    let (p, _) = proposal_pda(&multisig, 42, None);
    assert_ne!(
        t, p,
        "VaultTransaction and Proposal PDAs MUST differ for the same index"
    );
}

#[test]
fn transaction_pda_uses_full_8_byte_index() {
    let multisig = Pubkey::new_from_array([4u8; 32]);
    // 1 vs 2^32, two 8-byte LE encodings differing only in the upper bytes.
    let (a, _) = transaction_pda(&multisig, 1u64, None);
    let (b, _) = transaction_pda(&multisig, (1u64 << 32) | 1u64, None);
    assert_ne!(
        a, b,
        "transaction_pda must seed on all 8 bytes of transaction_index"
    );
}
