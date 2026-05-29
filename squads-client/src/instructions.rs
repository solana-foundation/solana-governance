//! Anchor instruction builders for `vault_transaction_create`, `proposal_create`, and
//! `proposal_approve` on the Squads V4 multisig program.
//!
//! Each builder returns a [`solana_program::instruction::Instruction`] with the correct
//! account ordering, discriminator, and borsh-encoded args.

use std::io::Write;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
#[allow(deprecated)]
use solana_program::system_program;

use crate::discriminator::instruction_discriminator;

// ============================================================================
// vault_transaction_create
// ============================================================================

/// Args for `vault_transaction_create`. The on-chain Anchor signature is:
///
/// ```ignore
/// pub struct VaultTransactionCreateArgs {
///     pub vault_index: u8,
///     pub ephemeral_signers: u8,
///     pub transaction_message: Vec<u8>,
///     pub memo: Option<String>,
/// }
/// ```
#[derive(Clone, Debug)]
pub struct VaultTransactionCreateArgs {
    /// Vault that will execute the wrapped instructions (defaults to `0`).
    pub vault_index: u8,
    /// Number of ephemeral signer PDAs required by the wrapped instructions (typically `0`).
    pub ephemeral_signers: u8,
    /// Borsh-serialized [`TransactionMessage`] bytes.
    pub transaction_message: Vec<u8>,
    /// Optional indexer memo.
    pub memo: Option<String>,
}

impl VaultTransactionCreateArgs {
    /// Serializes the args using the Anchor wire format (discriminator NOT included).
    pub fn try_to_vec(&self) -> std::io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(8 + self.transaction_message.len() + 32);
        out.push(self.vault_index);
        out.push(self.ephemeral_signers);
        // Vec<u8> with u32 length prefix (borsh default).
        (self.transaction_message.len() as u32).serialize(&mut out)?;
        out.write_all(&self.transaction_message)?;
        // Option<String> with single tag byte + (Some) u32 length + bytes.
        match &self.memo {
            None => out.push(0),
            Some(s) => {
                out.push(1);
                (s.len() as u32).serialize(&mut out)?;
                out.write_all(s.as_bytes())?;
            }
        }
        Ok(out)
    }
}

/// Account inputs for `vault_transaction_create`.
#[derive(Clone, Copy, Debug)]
pub struct VaultTransactionCreateAccounts {
    /// The multisig account whose vault will execute the transaction.
    pub multisig: Pubkey,
    /// The new `VaultTransaction` PDA to be initialized. Caller must derive this
    /// with [`crate::pda::transaction_pda`] using `multisig.transaction_index + 1`.
    pub transaction: Pubkey,
    /// Member of the multisig creating the transaction. Must hold the `Initiate` permission.
    pub creator: Pubkey,
    /// Funds the transaction-account rent. May be the same as `creator`.
    pub rent_payer: Pubkey,
}

/// Build a `vault_transaction_create` instruction.
pub fn vault_transaction_create_ix(
    program_id: &Pubkey,
    accounts: VaultTransactionCreateAccounts,
    args: &VaultTransactionCreateArgs,
) -> std::io::Result<Instruction> {
    let mut data = Vec::with_capacity(8 + args.transaction_message.len() + 32);
    data.extend_from_slice(&instruction_discriminator("vault_transaction_create"));
    data.extend(args.try_to_vec()?);

    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(accounts.multisig, false),
            AccountMeta::new(accounts.transaction, false),
            AccountMeta::new_readonly(accounts.creator, true),
            AccountMeta::new(accounts.rent_payer, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    })
}

/// Convenience wrapper that compiles `inner_instructions` into a `TransactionMessage`
/// and emits the resulting `vault_transaction_create` instruction.
///
/// `vault_pda` is the vault PDA derived from the multisig and `vault_index`.
#[allow(clippy::too_many_arguments)]
pub fn vault_transaction_create_from_instructions(
    program_id: &Pubkey,
    multisig: &Pubkey,
    transaction: &Pubkey,
    creator: &Pubkey,
    rent_payer: &Pubkey,
    vault_index: u8,
    vault_pda: &Pubkey,
    inner_instructions: &[solana_program::instruction::Instruction],
    address_lookup_table_accounts: &[solana_program::message::AddressLookupTableAccount],
    memo: Option<String>,
) -> Result<Instruction, crate::error::SquadsError> {
    let message = crate::message::try_compile(
        vault_pda,
        inner_instructions,
        address_lookup_table_accounts,
    )?;
    let mut transaction_message = Vec::new();
    message
        .serialize(&mut transaction_message)
        .map_err(crate::error::SquadsError::BorshEncode)?;

    let args = VaultTransactionCreateArgs {
        vault_index,
        ephemeral_signers: 0,
        transaction_message,
        memo,
    };

    vault_transaction_create_ix(
        program_id,
        VaultTransactionCreateAccounts {
            multisig: *multisig,
            transaction: *transaction,
            creator: *creator,
            rent_payer: *rent_payer,
        },
        &args,
    )
    .map_err(crate::error::SquadsError::BorshEncode)
}

// ============================================================================
// proposal_create
// ============================================================================

/// Args for `proposal_create`.
#[derive(Clone, Debug)]
pub struct ProposalCreateArgs {
    /// The transaction index this proposal is associated with (must equal an existing
    /// `VaultTransaction` index, typically the one just initialized via
    /// [`vault_transaction_create_ix`]).
    pub transaction_index: u64,
    /// If `true`, the proposal is created in `Draft` status (no votes accepted until
    /// `proposal_activate` is called). Set to `false` for the normal active flow.
    pub draft: bool,
}

impl ProposalCreateArgs {
    fn try_to_vec(&self) -> std::io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(9);
        out.write_all(&self.transaction_index.to_le_bytes())?;
        out.push(if self.draft { 1 } else { 0 });
        Ok(out)
    }
}

/// Account inputs for `proposal_create`.
#[derive(Clone, Copy, Debug)]
pub struct ProposalCreateAccounts {
    /// The multisig.
    pub multisig: Pubkey,
    /// The proposal PDA to be initialized. Derive via [`crate::pda::proposal_pda`].
    pub proposal: Pubkey,
    /// Member of the multisig creating the proposal. Must hold `Initiate` or `Vote`.
    pub creator: Pubkey,
    /// Funds the proposal account rent. May be the same as `creator`.
    pub rent_payer: Pubkey,
}

/// Build a `proposal_create` instruction.
pub fn proposal_create_ix(
    program_id: &Pubkey,
    accounts: ProposalCreateAccounts,
    args: &ProposalCreateArgs,
) -> std::io::Result<Instruction> {
    let mut data = Vec::with_capacity(8 + 9);
    data.extend_from_slice(&instruction_discriminator("proposal_create"));
    data.extend(args.try_to_vec()?);

    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(accounts.multisig, false),
            AccountMeta::new(accounts.proposal, false),
            AccountMeta::new_readonly(accounts.creator, true),
            AccountMeta::new(accounts.rent_payer, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    })
}

// ============================================================================
// proposal_approve
// ============================================================================

/// Args for `proposal_approve` (and the other `proposal_*_vote` instructions).
#[derive(Clone, Debug, Default)]
pub struct ProposalVoteArgs {
    /// Optional indexer memo.
    pub memo: Option<String>,
}

impl ProposalVoteArgs {
    fn try_to_vec(&self) -> std::io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(8);
        match &self.memo {
            None => out.push(0),
            Some(s) => {
                out.push(1);
                (s.len() as u32).serialize(&mut out)?;
                out.write_all(s.as_bytes())?;
            }
        }
        Ok(out)
    }
}

/// Account inputs for `proposal_approve`.
#[derive(Clone, Copy, Debug)]
pub struct ProposalApproveAccounts {
    /// The multisig (read-only).
    pub multisig: Pubkey,
    /// The member casting the approve vote. Must hold `Vote` permission.
    pub member: Pubkey,
    /// The proposal being approved.
    pub proposal: Pubkey,
}

/// Build a `proposal_approve` instruction.
pub fn proposal_approve_ix(
    program_id: &Pubkey,
    accounts: ProposalApproveAccounts,
    args: &ProposalVoteArgs,
) -> std::io::Result<Instruction> {
    let mut data = Vec::with_capacity(8 + 8);
    data.extend_from_slice(&instruction_discriminator("proposal_approve"));
    data.extend(args.try_to_vec()?);

    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(accounts.multisig, false),
            AccountMeta::new(accounts.member, true),
            AccountMeta::new(accounts.proposal, false),
        ],
        data,
    })
}

// ============================================================================
// Deserialization helpers (round-trip support for testing)
// ============================================================================

impl VaultTransactionCreateArgs {
    /// Decodes args from raw instruction bytes (with the 8-byte discriminator prefix stripped).
    pub fn try_from_slice(mut bytes: &[u8]) -> std::io::Result<Self> {
        let mut single = [0u8; 1];
        std::io::Read::read_exact(&mut bytes, &mut single)?;
        let vault_index = single[0];
        std::io::Read::read_exact(&mut bytes, &mut single)?;
        let ephemeral_signers = single[0];
        let len = u32::deserialize_reader(&mut bytes)? as usize;
        let mut transaction_message = vec![0u8; len];
        std::io::Read::read_exact(&mut bytes, &mut transaction_message)?;
        let memo = <Option<String>>::deserialize_reader(&mut bytes)?;
        Ok(Self {
            vault_index,
            ephemeral_signers,
            transaction_message,
            memo,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discriminator::instruction_discriminator;
    use solana_program::instruction::AccountMeta;

    #[test]
    fn vault_transaction_create_args_roundtrip() {
        let args = VaultTransactionCreateArgs {
            vault_index: 0,
            ephemeral_signers: 0,
            transaction_message: vec![0u8; 16],
            memo: Some("test memo".to_string()),
        };
        let bytes = args.try_to_vec().unwrap();
        let decoded = VaultTransactionCreateArgs::try_from_slice(&bytes).unwrap();
        assert_eq!(decoded.vault_index, args.vault_index);
        assert_eq!(decoded.ephemeral_signers, args.ephemeral_signers);
        assert_eq!(decoded.transaction_message, args.transaction_message);
        assert_eq!(decoded.memo, args.memo);
    }

    #[test]
    fn vault_transaction_create_ix_has_correct_account_ordering() {
        let program_id = Pubkey::new_unique();
        let multisig = Pubkey::new_unique();
        let transaction = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let rent_payer = Pubkey::new_unique();
        let args = VaultTransactionCreateArgs {
            vault_index: 0,
            ephemeral_signers: 0,
            transaction_message: vec![],
            memo: None,
        };
        let ix = vault_transaction_create_ix(
            &program_id,
            VaultTransactionCreateAccounts {
                multisig,
                transaction,
                creator,
                rent_payer,
            },
            &args,
        )
        .unwrap();
        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 5);
        assert_eq!(
            ix.accounts[0],
            AccountMeta::new(multisig, false),
            "multisig is mut, not signer"
        );
        assert_eq!(
            ix.accounts[1],
            AccountMeta::new(transaction, false),
            "transaction is mut, not signer"
        );
        assert_eq!(
            ix.accounts[2],
            AccountMeta::new_readonly(creator, true),
            "creator is readonly signer"
        );
        assert_eq!(
            ix.accounts[3],
            AccountMeta::new(rent_payer, true),
            "rent_payer is mut signer"
        );
        assert_eq!(
            ix.accounts[4],
            AccountMeta::new_readonly(system_program::ID, false),
            "system_program is readonly non-signer"
        );
        assert_eq!(
            &ix.data[..8],
            &instruction_discriminator("vault_transaction_create")
        );
    }

    #[test]
    fn proposal_create_ix_data_format() {
        let program_id = Pubkey::new_unique();
        let ix = proposal_create_ix(
            &program_id,
            ProposalCreateAccounts {
                multisig: Pubkey::new_unique(),
                proposal: Pubkey::new_unique(),
                creator: Pubkey::new_unique(),
                rent_payer: Pubkey::new_unique(),
            },
            &ProposalCreateArgs {
                transaction_index: 42,
                draft: false,
            },
        )
        .unwrap();
        assert_eq!(
            &ix.data[..8],
            &instruction_discriminator("proposal_create")
        );
        // After the discriminator, u64 le-bytes + bool byte.
        let mut expected_args = vec![];
        expected_args.extend_from_slice(&42u64.to_le_bytes());
        expected_args.push(0);
        assert_eq!(&ix.data[8..], expected_args.as_slice());
    }

    #[test]
    fn proposal_approve_ix_data_format() {
        let program_id = Pubkey::new_unique();
        let ix = proposal_approve_ix(
            &program_id,
            ProposalApproveAccounts {
                multisig: Pubkey::new_unique(),
                member: Pubkey::new_unique(),
                proposal: Pubkey::new_unique(),
            },
            &ProposalVoteArgs { memo: None },
        )
        .unwrap();
        assert_eq!(
            &ix.data[..8],
            &instruction_discriminator("proposal_approve")
        );
        // After discriminator, the Option<String> is just a 0 tag.
        assert_eq!(&ix.data[8..], &[0u8]);
    }

    #[test]
    fn proposal_approve_ix_with_memo() {
        let ix = proposal_approve_ix(
            &Pubkey::new_unique(),
            ProposalApproveAccounts {
                multisig: Pubkey::new_unique(),
                member: Pubkey::new_unique(),
                proposal: Pubkey::new_unique(),
            },
            &ProposalVoteArgs {
                memo: Some("ok".into()),
            },
        )
        .unwrap();
        // After discriminator: tag=1, len=2, "ok"
        let body = &ix.data[8..];
        assert_eq!(body[0], 1, "Some tag");
        assert_eq!(&body[1..5], &2u32.to_le_bytes());
        assert_eq!(&body[5..7], b"ok");
    }
}
