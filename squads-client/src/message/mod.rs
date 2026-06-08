//! `TransactionMessage` wire format and the `try_compile` helper that converts a
//! `&[Instruction]` into the compressed shape expected by `vault_transaction_create`.
//!
//! This is a port of `sdk/rs/src/vault_transaction/{vault_transaction_message.rs,compiled_keys.rs}`
//! from `Squads-Protocol/v4` (MIT OR Apache-2.0), adapted to be self-contained.

mod compiled_keys;

use std::io::{Read, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::Instruction;
use solana_program::message::v0::MessageAddressTableLookup as V0MessageAddressTableLookup;
use solana_program::message::{AccountKeys, AddressLookupTableAccount, MessageHeader};
use solana_program::pubkey::Pubkey;

use crate::small_vec::SmallVec;

use self::compiled_keys::CompiledKeys;

/// Errors that can be produced by [`try_compile`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageCompileError {
    /// The account keys list contains more keys than fit in a `u8` index.
    AccountIndexOverflow,
    /// An address lookup table lookup index exceeds `u8::MAX`.
    AddressTableLookupIndexOverflow,
    /// `AccountKeys::try_compile_instructions` failed.
    InstructionCompileFailure(String),
    /// The underlying `solana_program::message::v0::Message::try_compile` error type.
    SolanaCompileError(String),
}

impl std::fmt::Display for MessageCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccountIndexOverflow => write!(f, "account index overflow"),
            Self::AddressTableLookupIndexOverflow => write!(f, "address lookup table index overflow"),
            Self::InstructionCompileFailure(msg) => write!(f, "instruction compile failure: {msg}"),
            Self::SolanaCompileError(msg) => write!(f, "solana compile error: {msg}"),
        }
    }
}

impl std::error::Error for MessageCompileError {}

/// Wire-format transaction message. This is the unvalidated input shape the on-chain
/// `vault_transaction_create` instruction expects; the program decodes it into a
/// validated `VaultTransactionMessage` on-chain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TransactionMessage {
    /// Number of signer pubkeys in `account_keys`.
    pub num_signers: u8,
    /// Number of writable signer pubkeys in `account_keys`.
    pub num_writable_signers: u8,
    /// Number of writable non-signer pubkeys in `account_keys`.
    pub num_writable_non_signers: u8,
    /// Unique account pubkeys (including program IDs) required for the transaction.
    /// Ordering: writable-signers, readonly-signers, writable-non-signers, readonly-non-signers.
    pub account_keys: SmallVec<u8, Pubkey>,
    /// Compiled instructions.
    pub instructions: SmallVec<u8, CompiledInstruction>,
    /// Address-table lookups used to load additional accounts.
    pub address_table_lookups: SmallVec<u8, MessageAddressTableLookup>,
}

impl BorshSerialize for TransactionMessage {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&[
            self.num_signers,
            self.num_writable_signers,
            self.num_writable_non_signers,
        ])?;
        self.account_keys.serialize(writer)?;
        self.instructions.serialize(writer)?;
        self.address_table_lookups.serialize(writer)?;
        Ok(())
    }
}

impl BorshDeserialize for TransactionMessage {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut header = [0u8; 3];
        reader.read_exact(&mut header)?;
        let account_keys = SmallVec::<u8, Pubkey>::deserialize_reader(reader)?;
        let instructions = SmallVec::<u8, CompiledInstruction>::deserialize_reader(reader)?;
        let address_table_lookups =
            SmallVec::<u8, MessageAddressTableLookup>::deserialize_reader(reader)?;
        Ok(Self {
            num_signers: header[0],
            num_writable_signers: header[1],
            num_writable_non_signers: header[2],
            account_keys,
            instructions,
            address_table_lookups,
        })
    }
}

/// A single instruction in [`TransactionMessage`]. `account_indexes` reference the
/// `account_keys` vec; `program_id_index` references the program ID's entry there.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompiledInstruction {
    /// Index into `account_keys` of the program that will run this instruction.
    pub program_id_index: u8,
    /// Indices into `account_keys` of the accounts passed to this instruction.
    pub account_indexes: SmallVec<u8, u8>,
    /// Raw instruction data.
    pub data: SmallVec<u16, u8>,
}

impl BorshSerialize for CompiledInstruction {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&[self.program_id_index])?;
        self.account_indexes.serialize(writer)?;
        self.data.serialize(writer)?;
        Ok(())
    }
}

impl BorshDeserialize for CompiledInstruction {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut tag = [0u8; 1];
        reader.read_exact(&mut tag)?;
        let account_indexes = SmallVec::<u8, u8>::deserialize_reader(reader)?;
        let data = SmallVec::<u16, u8>::deserialize_reader(reader)?;
        Ok(Self {
            program_id_index: tag[0],
            account_indexes,
            data,
        })
    }
}

/// A single address-table lookup in [`TransactionMessage`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageAddressTableLookup {
    /// Address of the lookup table account.
    pub account_key: Pubkey,
    /// Indices into the lookup table for writable accounts.
    pub writable_indexes: SmallVec<u8, u8>,
    /// Indices into the lookup table for readonly accounts.
    pub readonly_indexes: SmallVec<u8, u8>,
}

impl BorshSerialize for MessageAddressTableLookup {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.account_key.to_bytes())?;
        self.writable_indexes.serialize(writer)?;
        self.readonly_indexes.serialize(writer)?;
        Ok(())
    }
}

impl BorshDeserialize for MessageAddressTableLookup {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut key_bytes = [0u8; 32];
        reader.read_exact(&mut key_bytes)?;
        let writable_indexes = SmallVec::<u8, u8>::deserialize_reader(reader)?;
        let readonly_indexes = SmallVec::<u8, u8>::deserialize_reader(reader)?;
        Ok(Self {
            account_key: Pubkey::from(key_bytes),
            writable_indexes,
            readonly_indexes,
        })
    }
}

/// Compresses a slice of `Instruction`s plus any address-lookup-table accounts into a
/// [`TransactionMessage`] suitable for embedding inside `vault_transaction_create`.
///
/// `vault_key` is the vault PDA from whose perspective the instructions are signed at
/// execution time. It is treated as the implicit fee-payer / signer for the compiled
/// message; the on-chain Squads program signs CPI invocations on its behalf during
/// `vault_transaction_execute`.
///
/// This is a near-direct port of `solana_program::message::v0::Message::try_compile`,
/// with one critical difference: program IDs are NOT marked as invoked (so they can be
/// loaded from address-lookup tables to compress the message). See the upstream
/// `compiled_keys.rs` for the original comment.
pub fn try_compile(
    vault_key: &Pubkey,
    instructions: &[Instruction],
    address_lookup_table_accounts: &[AddressLookupTableAccount],
) -> Result<TransactionMessage, MessageCompileError> {
    let mut compiled_keys = CompiledKeys::compile(instructions, Some(*vault_key));

    let mut address_table_lookups = Vec::with_capacity(address_lookup_table_accounts.len());
    let mut loaded_addresses_list = Vec::with_capacity(address_lookup_table_accounts.len());
    for lookup_table_account in address_lookup_table_accounts {
        if let Some((lookup, loaded_addresses)) =
            compiled_keys.try_extract_table_lookup(lookup_table_account)?
        {
            address_table_lookups.push(lookup);
            loaded_addresses_list.push(loaded_addresses);
        }
    }

    let (header, static_keys) = compiled_keys.try_into_message_components()?;
    let dynamic_keys: solana_program::message::v0::LoadedAddresses =
        loaded_addresses_list.into_iter().collect();
    let account_keys = AccountKeys::new(&static_keys, Some(&dynamic_keys));
    let instructions = account_keys
        .try_compile_instructions(instructions)
        .map_err(|err| MessageCompileError::InstructionCompileFailure(format!("{err:?}")))?;

    let num_static_keys: u8 = static_keys
        .len()
        .try_into()
        .map_err(|_| MessageCompileError::AccountIndexOverflow)?;

    let MessageHeader {
        num_required_signatures,
        num_readonly_signed_accounts,
        num_readonly_unsigned_accounts,
    } = header;

    let compiled_instructions: Vec<CompiledInstruction> = instructions
        .into_iter()
        .map(|ix| CompiledInstruction {
            program_id_index: ix.program_id_index,
            account_indexes: ix.accounts.into(),
            data: ix.data.into(),
        })
        .collect();

    let compiled_lookups: Vec<MessageAddressTableLookup> = address_table_lookups
        .into_iter()
        .map(|lookup: V0MessageAddressTableLookup| MessageAddressTableLookup {
            account_key: lookup.account_key,
            writable_indexes: lookup.writable_indexes.into(),
            readonly_indexes: lookup.readonly_indexes.into(),
        })
        .collect();

    Ok(TransactionMessage {
        num_signers: num_required_signatures,
        num_writable_signers: num_required_signatures.saturating_sub(num_readonly_signed_accounts),
        num_writable_non_signers: num_static_keys
            .saturating_sub(num_required_signatures)
            .saturating_sub(num_readonly_unsigned_accounts),
        account_keys: static_keys.into(),
        instructions: compiled_instructions.into(),
        address_table_lookups: compiled_lookups.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::instruction::AccountMeta;

    fn dummy_instruction(program_id: Pubkey, accounts: Vec<AccountMeta>, data: Vec<u8>) -> Instruction {
        Instruction {
            program_id,
            accounts,
            data,
        }
    }

    #[test]
    fn try_compile_one_writable_account_no_signers() {
        let vault = Pubkey::new_unique();
        let prog = Pubkey::new_unique();
        let writable = Pubkey::new_unique();
        let ix = dummy_instruction(
            prog,
            vec![AccountMeta::new(writable, false)],
            vec![1, 2, 3],
        );
        let msg = try_compile(&vault, &[ix], &[]).unwrap();
        // The vault is forced to be a writable signer. With one extra writable non-signer
        // and the program ID (readonly non-signer), we should have:
        // num_signers = 1, num_writable_signers = 1, num_writable_non_signers = 1
        assert_eq!(msg.num_signers, 1);
        assert_eq!(msg.num_writable_signers, 1);
        assert_eq!(msg.num_writable_non_signers, 1);
        assert_eq!(msg.account_keys.len(), 3);
        assert_eq!(msg.instructions.len(), 1);
        assert_eq!(msg.address_table_lookups.len(), 0);
    }

    #[test]
    fn try_compile_orders_account_keys_signers_first() {
        let vault = Pubkey::new_unique();
        let prog = Pubkey::new_unique();
        let writable = Pubkey::new_unique();
        let readonly = Pubkey::new_unique();
        let ix = dummy_instruction(
            prog,
            vec![
                AccountMeta::new(writable, false),
                AccountMeta::new_readonly(readonly, false),
            ],
            vec![],
        );
        let msg = try_compile(&vault, &[ix], &[]).unwrap();
        let keys = msg.account_keys.as_slice();
        // First key is the writable signer (the vault).
        assert_eq!(keys[0], vault);
        assert_eq!(msg.num_signers, 1);
        assert_eq!(msg.num_writable_signers, 1);
    }

    #[test]
    fn transaction_message_borsh_roundtrip() {
        let vault = Pubkey::new_unique();
        let prog = Pubkey::new_unique();
        let target = Pubkey::new_unique();
        let ix = dummy_instruction(
            prog,
            vec![AccountMeta::new(target, false)],
            vec![0xaa, 0xbb, 0xcc],
        );
        let msg = try_compile(&vault, &[ix], &[]).unwrap();

        let mut buf = vec![];
        msg.serialize(&mut buf).unwrap();
        let decoded = TransactionMessage::try_from_slice(&buf).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn compiled_instruction_data_uses_u16_length_prefix() {
        // 300 bytes of data should fit in u16 length but not u8.
        let big_data = vec![0u8; 300];
        let vault = Pubkey::new_unique();
        let prog = Pubkey::new_unique();
        let ix = dummy_instruction(prog, vec![], big_data.clone());
        let msg = try_compile(&vault, &[ix], &[]).unwrap();

        let mut buf = vec![];
        msg.serialize(&mut buf).unwrap();
        // We don't pin the exact byte offset, but a round-trip should succeed and the
        // decoded data should equal `big_data`.
        let decoded = TransactionMessage::try_from_slice(&buf).unwrap();
        assert_eq!(decoded.instructions.len(), 1);
        let inst = &decoded.instructions.as_slice()[0];
        assert_eq!(inst.data.as_slice(), big_data.as_slice());
    }
}
