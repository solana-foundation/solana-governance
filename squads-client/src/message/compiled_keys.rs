//! Port of `solana_sdk::message::v0::CompiledKeys` (private upstream) used by
//! Squads' `try_compile`. Sourced from `sdk/rs/src/vault_transaction/compiled_keys.rs`
//! in `Squads-Protocol/v4`.
//!
//! The key difference from a vanilla compile pass is that program IDs are intentionally
//! NOT marked as invoked while building the `key_meta_map` (compare with the standard
//! solana implementation, which marks them invoked and prevents loading them from ALTs).
//! This relaxation lets `vault_transaction_create` keep static keys small by hoisting
//! program IDs into address-lookup tables.

use std::collections::BTreeMap;

use solana_program::instruction::Instruction;
use solana_program::message::v0::{LoadedAddresses, MessageAddressTableLookup};
use solana_program::message::{AddressLookupTableAccount, MessageHeader};
use solana_program::pubkey::Pubkey;

use super::MessageCompileError;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub(super) struct CompiledKeyMeta {
    is_signer: bool,
    is_writable: bool,
    is_invoked: bool,
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub(super) struct CompiledKeys {
    payer: Option<Pubkey>,
    key_meta_map: BTreeMap<Pubkey, CompiledKeyMeta>,
}

impl CompiledKeys {
    /// Walk the instructions and build a per-key set of (signer, writable, invoked) flags.
    pub(super) fn compile(instructions: &[Instruction], payer: Option<Pubkey>) -> Self {
        let mut key_meta_map = BTreeMap::<Pubkey, CompiledKeyMeta>::new();
        for ix in instructions {
            // NOTE: program ids are intentionally NOT marked as invoked here. This
            // diverges from the upstream solana implementation and is what allows
            // program ids to be loaded via address lookup tables for size compression.
            key_meta_map.entry(ix.program_id).or_default();
            for account_meta in &ix.accounts {
                let meta = key_meta_map.entry(account_meta.pubkey).or_default();
                meta.is_signer |= account_meta.is_signer;
                meta.is_writable |= account_meta.is_writable;
            }
        }
        if let Some(payer) = &payer {
            let meta = key_meta_map.entry(*payer).or_default();
            meta.is_signer = true;
            meta.is_writable = true;
        }
        Self {
            payer,
            key_meta_map,
        }
    }

    /// Collapse the map into a `MessageHeader` and the ordered static key vector.
    /// The ordering is: payer, other writable signers, readonly signers, writable
    /// non-signers, readonly non-signers.
    pub(super) fn try_into_message_components(
        self,
    ) -> Result<(MessageHeader, Vec<Pubkey>), MessageCompileError> {
        let try_into_u8 = |num: usize| -> Result<u8, MessageCompileError> {
            u8::try_from(num).map_err(|_| MessageCompileError::AccountIndexOverflow)
        };

        let Self {
            payer,
            mut key_meta_map,
        } = self;

        if let Some(payer) = &payer {
            key_meta_map.remove(payer);
        }

        let writable_signer_keys: Vec<Pubkey> = payer
            .into_iter()
            .chain(
                key_meta_map
                    .iter()
                    .filter_map(|(k, v)| (v.is_signer && v.is_writable).then_some(*k)),
            )
            .collect();
        let readonly_signer_keys: Vec<Pubkey> = key_meta_map
            .iter()
            .filter_map(|(k, v)| (v.is_signer && !v.is_writable).then_some(*k))
            .collect();
        let writable_non_signer_keys: Vec<Pubkey> = key_meta_map
            .iter()
            .filter_map(|(k, v)| (!v.is_signer && v.is_writable).then_some(*k))
            .collect();
        let readonly_non_signer_keys: Vec<Pubkey> = key_meta_map
            .iter()
            .filter_map(|(k, v)| (!v.is_signer && !v.is_writable).then_some(*k))
            .collect();

        let signers_len = writable_signer_keys.len() + readonly_signer_keys.len();

        let header = MessageHeader {
            num_required_signatures: try_into_u8(signers_len)?,
            num_readonly_signed_accounts: try_into_u8(readonly_signer_keys.len())?,
            num_readonly_unsigned_accounts: try_into_u8(readonly_non_signer_keys.len())?,
        };

        let static_account_keys: Vec<Pubkey> = std::iter::empty()
            .chain(writable_signer_keys)
            .chain(readonly_signer_keys)
            .chain(writable_non_signer_keys)
            .chain(readonly_non_signer_keys)
            .collect();

        Ok((header, static_account_keys))
    }

    /// Extract any keys that can be loaded from the given lookup table account. Keys
    /// that are signers or have been marked as invoked cannot be hoisted into ALTs.
    /// (We never mark program ids as invoked, so program ids *are* eligible for ALTs.)
    pub(super) fn try_extract_table_lookup(
        &mut self,
        lookup_table_account: &AddressLookupTableAccount,
    ) -> Result<Option<(MessageAddressTableLookup, LoadedAddresses)>, MessageCompileError> {
        let (writable_indexes, drained_writable_keys) = self
            .try_drain_keys_found_in_lookup_table(&lookup_table_account.addresses, |meta| {
                !meta.is_signer && !meta.is_invoked && meta.is_writable
            })?;
        let (readonly_indexes, drained_readonly_keys) = self
            .try_drain_keys_found_in_lookup_table(&lookup_table_account.addresses, |meta| {
                !meta.is_signer && !meta.is_invoked && !meta.is_writable
            })?;

        if writable_indexes.is_empty() && readonly_indexes.is_empty() {
            return Ok(None);
        }

        Ok(Some((
            MessageAddressTableLookup {
                account_key: lookup_table_account.key,
                writable_indexes,
                readonly_indexes,
            },
            LoadedAddresses {
                writable: drained_writable_keys,
                readonly: drained_readonly_keys,
            },
        )))
    }

    fn try_drain_keys_found_in_lookup_table(
        &mut self,
        lookup_table_addresses: &[Pubkey],
        key_meta_filter: impl Fn(&CompiledKeyMeta) -> bool,
    ) -> Result<(Vec<u8>, Vec<Pubkey>), MessageCompileError> {
        let mut lookup_table_indexes: Vec<u8> = Vec::new();
        let mut drained_keys: Vec<Pubkey> = Vec::new();
        for (key, meta) in self.key_meta_map.iter() {
            if !key_meta_filter(meta) {
                continue;
            }
            if let Some(lookup_table_index) = lookup_table_addresses
                .iter()
                .position(|addr| addr == key)
                .and_then(|i| u8::try_from(i).ok())
            {
                lookup_table_indexes.push(lookup_table_index);
                drained_keys.push(*key);
            }
        }
        // Now remove the drained keys from the meta map so subsequent ALTs / static
        // resolution don't double-count them.
        for key in &drained_keys {
            self.key_meta_map.remove(key);
        }
        // Validate that every lookup_table_index fits in u8 (we filtered already, so
        // this is a defensive no-op).
        for idx in &lookup_table_indexes {
            let _ = idx;
        }
        Ok((lookup_table_indexes, drained_keys))
    }
}
