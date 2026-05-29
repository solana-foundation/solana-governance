//! Error type for `squads-client`.

use solana_program::pubkey::Pubkey;
use thiserror::Error;

use crate::message::MessageCompileError;

/// Errors that can be produced by this crate.
#[derive(Debug, Error)]
pub enum SquadsError {
    /// The proposer is not a member of the multisig.
    #[error("proposer {proposer} is not a member of multisig {multisig}")]
    ProposerNotMember {
        /// The proposer pubkey that was being verified.
        proposer: Pubkey,
        /// The multisig whose member list was checked.
        multisig: Pubkey,
    },

    /// The proposer is a member of the multisig but lacks the `Initiate` permission.
    #[error(
        "proposer {proposer} is a member of multisig {multisig} but lacks the Initiate permission"
    )]
    ProposerNotAuthorized {
        /// The proposer pubkey that was being verified.
        proposer: Pubkey,
        /// The multisig whose permissions were checked.
        multisig: Pubkey,
    },

    /// The on-chain account's leading 8-byte discriminator did not match what was expected.
    #[error(
        "discriminator mismatch for {type_name}: expected {expected:?}, got {actual:?}"
    )]
    DiscriminatorMismatch {
        /// The Rust type whose discriminator was expected.
        type_name: &'static str,
        /// The expected 8 bytes.
        expected: [u8; 8],
        /// The actual 8 bytes from the account data.
        actual: [u8; 8],
    },

    /// The on-chain account data was shorter than the minimum required (less than 8 bytes
    /// for the discriminator, or shorter than the body it claims to encode).
    #[error("account data too short: expected at least {expected} bytes, got {actual}")]
    AccountDataTooShort {
        /// Minimum byte count required.
        expected: usize,
        /// Actual byte count provided.
        actual: usize,
    },

    /// A Borsh deserialization error.
    #[error("borsh decode error: {0}")]
    BorshDecode(std::io::Error),

    /// A Borsh serialization error.
    #[error("borsh encode error: {0}")]
    BorshEncode(std::io::Error),

    /// Failed to compile a `&[Instruction]` into a [`TransactionMessage`](crate::TransactionMessage).
    #[error("transaction message compile error: {0:?}")]
    MessageCompile(MessageCompileError),

    /// Failed to fetch account data from the RPC.
    #[error("failed to fetch account {pubkey} from RPC: {reason}")]
    RpcFetch {
        /// The pubkey whose account we tried to fetch.
        pubkey: Pubkey,
        /// String form of the underlying RPC error (preserved as a String to avoid
        /// pinning a specific `solana-rpc-client` version in the error type).
        reason: String,
    },

    /// An account was fetched successfully but its data didn't decode into the expected type.
    #[error("failed to decode account {pubkey} as {type_name}: {reason}")]
    AccountDecode {
        /// The pubkey whose account data we tried to decode.
        pubkey: Pubkey,
        /// The Rust type name we tried to decode into.
        type_name: &'static str,
        /// Human-readable failure reason.
        reason: String,
    },
}

impl From<MessageCompileError> for SquadsError {
    fn from(err: MessageCompileError) -> Self {
        SquadsError::MessageCompile(err)
    }
}

/// A specialized [`Result`] type for `squads-client`.
pub type Result<T> = std::result::Result<T, SquadsError>;
