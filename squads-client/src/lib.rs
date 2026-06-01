//! A minimal Rust client for the Squads V4 multisig program.
//!
//! See `README.md` in the crate root for an overview and rationale.

pub mod client;
pub mod discriminator;
pub mod error;
pub mod id;
pub mod instructions;
pub mod message;
pub mod pda;
#[cfg(feature = "rpc")]
pub mod router;
pub mod small_vec;
pub mod state;

pub use client::{BuiltVaultTransaction, SquadsClient};
#[cfg(feature = "rpc")]
pub use client::SquadsMultisigClient;
pub use error::{Result as SquadsResult, SquadsError};
pub use id::{with_program_id_or_default, PROGRAM_ID};
pub use instructions::{
    proposal_approve_ix, proposal_create_ix, vault_transaction_create_from_instructions,
    vault_transaction_create_ix, ProposalApproveAccounts, ProposalCreateAccounts,
    ProposalCreateArgs, ProposalVoteArgs, VaultTransactionCreateAccounts,
    VaultTransactionCreateArgs,
};
#[cfg(feature = "rpc")]
pub use router::{route_or_send, RoutedOutcome, RouterRpc, SquadsRoutingConfig};
pub use message::{
    try_compile, CompiledInstruction, MessageAddressTableLookup, MessageCompileError,
    TransactionMessage,
};
pub use pda::{
    ephemeral_signer_pda, multisig_pda, program_config_pda, proposal_pda, spending_limit_pda,
    transaction_pda, vault_pda,
};
pub use small_vec::SmallVec;
pub use state::{
    Member, Multisig, Permission, Permissions, Proposal, ProposalStatus, VaultTransaction,
};
