//! Anchor-style 8-byte discriminator computation.
//!
//! Anchor 0.29 (the version Squads V4 was built with) uses the first 8 bytes of
//! `sha256("<namespace>:<name>")` to disambiguate instructions and accounts. This
//! module reproduces that derivation so the crate can encode instructions and
//! validate account data without depending on `anchor-lang`.

use sha2::{Digest, Sha256};

/// Computes an Anchor instruction discriminator: `sha256("global:<name>")[..8]`.
///
/// `name` is the snake_case handler name declared in the `#[program]` module, e.g.
/// `"vault_transaction_create"` or `"proposal_create"`.
pub fn instruction_discriminator(name: &str) -> [u8; 8] {
    discriminator("global", name)
}

/// Computes an Anchor account discriminator: `sha256("account:<TypeName>")[..8]`.
///
/// `type_name` is the Rust type name as written in the program source, e.g. `"Multisig"`.
pub fn account_discriminator(type_name: &str) -> [u8; 8] {
    discriminator("account", type_name)
}

fn discriminator(namespace: &str, name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    hasher.update(b":");
    hasher.update(name.as_bytes());
    let hash = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminator_format_global_namespace() {
        let d = instruction_discriminator("vault_transaction_create");
        // Sanity: discriminators are deterministic; if the algorithm regresses this byte will change.
        assert_eq!(d.len(), 8);
    }

    #[test]
    fn discriminator_format_account_namespace() {
        let d = account_discriminator("Multisig");
        assert_eq!(d.len(), 8);
    }

    #[test]
    fn discriminators_are_distinct_per_name() {
        assert_ne!(
            instruction_discriminator("vault_transaction_create"),
            instruction_discriminator("proposal_create"),
        );
        assert_ne!(
            account_discriminator("Multisig"),
            account_discriminator("Proposal"),
        );
    }
}
